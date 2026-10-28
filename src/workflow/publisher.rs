use crate::error::{AppError, Result};
use crate::github::GitHubClient;
use crate::sandbox::traits::SandboxInstance;
use crate::workflow::command::WorkflowMode;
use crate::workflow::params::WorkflowParams;
use crate::workflow::verification::ProjectVerifier;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
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

/// GitHub can briefly expose the previous PR head after a successful Git push.
/// Report success only after observing the requested SHA, but allow a bounded
/// period of API propagation instead of incorrectly claiming a failed push.
async fn confirm_published_sha(
    gh: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
    published_sha: &str,
) -> Result<()> {
    let mut last_observed = String::new();
    for attempt in 0..5 {
        match gh.get_pull_request(owner, repo, pr_number).await {
            Ok(pr) if pr.head.sha == published_sha => return Ok(()),
            Ok(pr) => last_observed = pr.head.sha,
            Err(error) => last_observed = error.to_string(),
        }
        if attempt < 4 {
            sleep(Duration::from_millis(250 * (attempt + 1))).await;
        }
    }
    Err(AppError::Workflow(format!(
        "Push returned successfully but remote PR head could not be confirmed: expected {published_sha}, last observed {last_observed}. Check GitHub before retrying to avoid duplicate publication"
    )))
}

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
                verification_passed: false,
                success: true,
                reason: Some("Dry-run mode: remote changes simulated without push".to_string()),
            });
        }

        let gh = gh_client.ok_or_else(|| {
            AppError::Unauthorized("Publishing requires an authenticated GitHub App client".into())
        })?;
        let starting_sha = params.head_sha.as_deref().ok_or_else(|| {
            AppError::Workflow("Publishing requires the verified starting PR head SHA".into())
        })?;
        if params.actor.trim().is_empty() {
            return Err(AppError::Unauthorized(
                "Publishing requires an identified authorizer".into(),
            ));
        }

        // Recheck authorization after the agent completes: access can be revoked mid-run.
        let permission = gh
            .get_collaborator_permission(owner, repo, &params.actor)
            .await?;
        if !crate::github::client::is_permission_authorized_for_fix(&permission) {
            return Err(AppError::Unauthorized(
                "Fix authorization was revoked during execution".into(),
            ));
        }

        // 3. Check if files were actually modified
        let has_changes = sandbox.has_uncommitted_changes().await?;
        if !has_changes {
            info!(
                "No files modified during fix workflow for PR #{}",
                params.pr_number
            );
            gh.post_comment(owner, repo, params.pr_number,
                "## Fix review completed\n\nNo repository changes were made; nothing was committed or pushed.").await?;
            return Ok(PublishAuditRecord {
                actor: params.actor.clone(),
                installation_id: params.installation_id,
                repo_full_name: params.repo_full_name.clone(),
                pr_number: params.pr_number,
                target_branch: params.pr_branch.clone(),
                starting_sha: params.head_sha.clone(),
                published_sha: None,
                action_taken: "no_changes".to_string(),
                verification_passed: false,
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
        let current_pr = gh.get_pull_request(owner, repo, params.pr_number).await?;
        let head_repo = current_pr.head.repo.as_ref().map(|r| r.full_name.as_str());
        if head_repo != Some(params.repo_full_name.as_str())
            || current_pr.head.r#ref != params.pr_branch
        {
            return Err(AppError::Workflow(
                "PR head repository or target branch changed; publish aborted".into(),
            ));
        }
        if current_pr.head.sha != starting_sha {
            let reason = format!(
                "Target branch changed concurrently: started at {}, current is {}",
                starting_sha, current_pr.head.sha
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

        // Branch protection can change while the agent runs. Recheck it on the
        // authenticated publishing path, after verification and head-SHA checks.
        // The fourth argument is retained for the existing client API but is
        // no longer used to infer authorization from inaccessible Admin APIs.
        let access = gh
            .check_push_access(owner, repo, &params.pr_branch, "")
            .await?;
        if !access.can_push {
            return Err(AppError::Workflow(access.reason.unwrap_or_else(|| {
                "Branch protection prevents autonomous Fix publishing".into()
            })));
        }

        // 6. Safe Commit and Push
        let commit_msg = format!("openreview: automated fixes requested by @{}", params.actor);
        sandbox
            .commit_and_push(&commit_msg, &params.pr_branch)
            .await?;

        // Determine resulting SHA
        let rev_out = sandbox.run_command("git", &["rev-parse", "HEAD"]).await?;
        if rev_out.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "Unable to determine published git commit: {}",
                rev_out.stderr
            )));
        }
        let published_sha = rev_out.stdout.trim().to_string();
        confirm_published_sha(gh, owner, repo, params.pr_number, &published_sha).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn github_pr_head_eventual_consistency_is_retried_before_reporting_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for head in ["old-head", "new-head"] {
                let (mut connection, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 2048];
                let bytes = connection.read(&mut request).await.unwrap();
                assert!(
                    String::from_utf8_lossy(&request[..bytes])
                        .contains("GET /repos/owner/repo/pulls/7")
                );
                let body = serde_json::json!({
                    "id": 1,
                    "number": 7,
                    "title": "Review",
                    "head": {"ref": "feature", "sha": head, "repo": null},
                    "base": {"ref": "main", "sha": "base", "repo": null},
                    "html_url": "https://github.com/owner/repo/pull/7",
                    "user": {"id": 1, "login": "owner"}
                })
                .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                connection.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let client =
            GitHubClient::with_base_url("test-token".into(), format!("http://{addr}")).unwrap();
        confirm_published_sha(&client, "owner", "repo", 7, "new-head")
            .await
            .unwrap();
        server.await.unwrap();
    }
    struct ReadySandbox {
        root: tempfile::TempDir,
    }

    #[async_trait::async_trait]
    impl SandboxInstance for ReadySandbox {
        async fn run_command(
            &self,
            _: &str,
            _: &[&str],
        ) -> Result<crate::sandbox::traits::CommandOutput> {
            Err(AppError::Sandbox("unexpected command".into()))
        }

        async fn run_bash(&self, _: &str) -> Result<crate::sandbox::traits::CommandOutput> {
            Err(AppError::Sandbox("unexpected shell".into()))
        }

        async fn run_verification(
            &self,
            command: &str,
        ) -> Result<crate::sandbox::traits::CommandOutput> {
            assert_eq!(command, "cargo test");
            Ok(crate::sandbox::traits::CommandOutput {
                exit_code: 0,
                stdout: "running 1 test\ntest result: ok. 1 passed; 0 failed".into(),
                stderr: String::new(),
            })
        }

        async fn read_file(&self, _: &str) -> Result<String> {
            Err(AppError::Sandbox("unexpected read".into()))
        }

        async fn write_file(&self, _: &str, _: &str) -> Result<()> {
            Err(AppError::Sandbox("unexpected write".into()))
        }

        async fn has_uncommitted_changes(&self) -> Result<bool> {
            Ok(true)
        }

        async fn commit_and_push(&self, _: &str, _: &str) -> Result<()> {
            Err(AppError::Workflow("unsafe push reached".into()))
        }

        fn work_dir(&self) -> &std::path::Path {
            self.root.path()
        }
    }

    #[tokio::test]
    async fn branch_protection_is_rechecked_after_verification_before_publishing() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("Cargo.toml"), "[package]").unwrap();
        let sandbox = ReadySandbox { root };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let repository =
            r#"{"id":1,"name":"repo","full_name":"owner/repo","owner":{"id":1,"login":"owner"}}"#;
        let pr = r#"{"id":7,"number":7,"title":"Fix","head":{"ref":"feature","sha":"initial","repo":{"id":1,"name":"repo","full_name":"owner/repo","owner":{"id":1,"login":"owner"}}},"base":{"ref":"main","sha":"base","repo":null},"html_url":"https://github.com/owner/repo/pull/7","user":{"id":2,"login":"alice"}}"#;
        let responses = [
            (
                "/repos/owner/repo/collaborators/alice/permission",
                r#"{"permission":"write","user":{"id":2,"login":"alice"}}"#,
            ),
            ("/repos/owner/repo/pulls/7", pr),
            ("/repos/owner/repo", repository),
            (
                "/repos/owner/repo/branches/feature",
                r#"{"name":"feature","protected":true}"#,
            ),
        ];
        let server = tokio::spawn(async move {
            for (path, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 4096];
                let bytes = stream.read(&mut request).await.unwrap();
                assert!(
                    String::from_utf8_lossy(&request[..bytes])
                        .starts_with(&format!("GET {path} HTTP/1.1"))
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let client =
            GitHubClient::with_base_url("test-token".into(), format!("http://{addr}")).unwrap();
        let params = WorkflowParams {
            repo_full_name: "owner/repo".into(),
            pr_number: 7,
            base_branch: "main".into(),
            pr_branch: "feature".into(),
            comment_id: None,
            installation_id: Some(42),
            mode: WorkflowMode::Fix,
            actor: "alice".into(),
            dry_run: false,
            head_sha: Some("initial".into()),
            from_webhook: true,
            messages: vec![],
        };
        let error = WorkflowPublisher::evaluate_and_publish(&sandbox, Some(&client), &params)
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("protected"),
            "publish reached an unsafe path: {error}"
        );
        server.await.unwrap();
    }
}
