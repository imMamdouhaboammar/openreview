use crate::error::{AppError, Result};
use crate::github::GitHubClient;
use crate::sandbox::traits::SandboxInstance;
use crate::workflow::command::WorkflowMode;
use crate::workflow::params::WorkflowParams;
use crate::workflow::verification::ProjectVerifier;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishAuditRecord {
    pub actor: String,
    pub installation_id: Option<u64>,
    pub repo_full_name: String,
    pub pr_number: u64,
    pub target_branch: String,
    pub starting_sha: Option<String>,
    pub published_sha: Option<String>,
    pub action_taken: String,
    pub verification_passed: bool,
    pub success: bool,
    pub reason: Option<String>,
}

pub struct WorkflowPublisher;

impl WorkflowPublisher {
    pub async fn evaluate_and_publish(
        sandbox: &dyn SandboxInstance,
        gh_client: Option<&GitHubClient>,
        params: &WorkflowParams,
    ) -> Result<PublishAuditRecord> {
        let [owner, repo] = match params.repo_full_name.split_once('/') {
            Some((o, r)) => [o, r],
            None => {
                return Err(AppError::Config(format!(
                    "Invalid repo: {}",
                    params.repo_full_name
                )));
            }
        };

        // 1. Publishing must be strictly IMPOSSIBLE in Review mode
        if params.mode == WorkflowMode::Review {
            return Err(AppError::PublishingForbiddenInReviewMode);
        }

        // 2. Dry-Run Gate
        if params.dry_run {
            info!(
                "Dry-run mode active for {} #{}: skipping remote git push",
                params.repo_full_name, params.pr_number
            );
            return Ok(PublishAuditRecord {
                actor: params.actor.clone(),
                installation_id: params.installation_id,
                repo_full_name: params.repo_full_name.clone(),
                pr_number: params.pr_number,
                target_branch: params.pr_branch.clone(),
                starting_sha: params.head_sha.clone(),
                published_sha: None,
                action_taken: "dry_run_simulated".to_string(),
                verification_passed: true,
                success: true,
                reason: Some("Dry-run mode: remote changes simulated without push".to_string()),
            });
        }

        // 3. Check if files were actually modified
        let has_changes = sandbox.has_uncommitted_changes().await?;
        if !has_changes {
            info!(
                "No files modified during fix workflow for PR #{}",
                params.pr_number
            );
            return Ok(PublishAuditRecord {
                actor: params.actor.clone(),
                installation_id: params.installation_id,
                repo_full_name: params.repo_full_name.clone(),
                pr_number: params.pr_number,
                target_branch: params.pr_branch.clone(),
                starting_sha: params.head_sha.clone(),
                published_sha: None,
                action_taken: "no_changes".to_string(),
                verification_passed: true,
                success: true,
                reason: Some("No modifications detected in workspace".to_string()),
            });
        }

        // 4. Verification Gate: Test suite must pass before pushing
        let verification = ProjectVerifier::run_verification(sandbox).await?;
        if !verification.all_passed {
            let reason = "Automated verification checks failed in sandbox".to_string();
            warn!("{}: aborting git push for PR #{}", reason, params.pr_number);

            if let Some(gh) = gh_client {
                let report_md = format!(
                    "## ⚠️ Fix Publishing Blocked\n\nAutomated tests failed on proposed fixes. To prevent breaking the build, changes were not published.\n\n{}\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*",
                    verification.to_markdown()
                );
                let _ = gh
                    .post_comment(owner, repo, params.pr_number, &report_md)
                    .await;
            }

            return Ok(PublishAuditRecord {
                actor: params.actor.clone(),
                installation_id: params.installation_id,
                repo_full_name: params.repo_full_name.clone(),
                pr_number: params.pr_number,
                target_branch: params.pr_branch.clone(),
                starting_sha: params.head_sha.clone(),
                published_sha: None,
                action_taken: "verification_blocked".to_string(),
                verification_passed: false,
                success: false,
                reason: Some(reason),
            });
        }

        // 5. Check whether the target branch changed concurrently on remote
        if let Some(gh) = gh_client
            && let Some(start_sha) = &params.head_sha
            && let Ok(current_pr) = gh.get_pull_request(owner, repo, params.pr_number).await
            && &current_pr.head.sha != start_sha
        {
            let reason = format!(
                "Target branch changed concurrently: started at {}, current is {}",
                start_sha, current_pr.head.sha
            );
            warn!("{}: aborting git push", reason);
            let comment = format!(
                "## ⚠️ Fix Publishing Aborted\n\nBranch `{}` was updated with new commits while OpenReview was processing. Pushing was aborted to prevent overwriting commits.\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*",
                params.pr_branch
            );
            let _ = gh
                .post_comment(owner, repo, params.pr_number, &comment)
                .await;

            return Ok(PublishAuditRecord {
                actor: params.actor.clone(),
                installation_id: params.installation_id,
                repo_full_name: params.repo_full_name.clone(),
                pr_number: params.pr_number,
                target_branch: params.pr_branch.clone(),
                starting_sha: params.head_sha.clone(),
                published_sha: None,
                action_taken: "concurrency_blocked".to_string(),
                verification_passed: true,
                success: false,
                reason: Some(reason),
            });
        }

        // 6. Safe Commit and Push
        let commit_msg = format!("openreview: automated fixes requested by @{}", params.actor);
        sandbox
            .commit_and_push(&commit_msg, &params.pr_branch)
            .await?;

        // Determine resulting SHA
        let rev_out = sandbox.run_command("git", &["rev-parse", "HEAD"]).await?;
        let published_sha = rev_out.stdout.trim().to_string();

        info!(
            "Successfully published fix commit {} to {} on branch {}",
            published_sha, params.repo_full_name, params.pr_branch
        );

        // 7. Post Fix Summary and Verification Report to PR
        if let Some(gh) = gh_client {
            let summary_comment = format!(
                "## ✅ Fix Applied & Verified\n\nOpenReview applied requested code fixes in commit `{}` to branch `{}`.\n\n{}\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*",
                if published_sha.len() >= 7 {
                    &published_sha[..7]
                } else {
                    &published_sha
                },
                params.pr_branch,
                verification.to_markdown()
            );
            let _ = gh
                .post_comment(owner, repo, params.pr_number, &summary_comment)
                .await;
        }

        Ok(PublishAuditRecord {
            actor: params.actor.clone(),
            installation_id: params.installation_id,
            repo_full_name: params.repo_full_name.clone(),
            pr_number: params.pr_number,
            target_branch: params.pr_branch.clone(),
            starting_sha: params.head_sha.clone(),
            published_sha: Some(published_sha),
            action_taken: "committed_and_pushed".to_string(),
            verification_passed: true,
            success: true,
            reason: None,
        })
    }
}
