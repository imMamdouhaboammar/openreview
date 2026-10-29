use crate::agent::runner::{AgentRunResult, AgentRunner, AgentTerminationReason};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::github::auth::TokenManager;
use crate::github::client::{GitHubClient, is_permission_authorized_for_fix};
use crate::llm::traits::LlmProvider;
use crate::llm::types::ChatMessage;
use crate::sandbox::traits::SandboxDriver;
use crate::skills::discover_skills;
use crate::workflow::command::WorkflowMode;
use crate::workflow::context::PullRequestContext;
use crate::workflow::params::WorkflowParams;
use crate::workflow::publisher::WorkflowPublisher;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct WorkflowCoordinator {
    pub config: Config,
    pub llm: Arc<dyn LlmProvider>,
    pub sandbox_driver: Arc<dyn SandboxDriver>,
    pub http_client: reqwest::Client,
    pub token_manager: TokenManager,
}

impl WorkflowCoordinator {
    pub fn new(
        config: Config,
        llm: Arc<dyn LlmProvider>,
        sandbox_driver: Arc<dyn SandboxDriver>,
    ) -> Self {
        Self {
            config,
            llm,
            sandbox_driver,
            http_client: reqwest::Client::new(),
            token_manager: TokenManager::new(),
        }
    }

    pub async fn execute_bot_workflow(&self, mut params: WorkflowParams) -> Result<AgentRunResult> {
        let [owner, repo] = match params.repo_full_name.split_once('/') {
            Some((o, r)) => [o, r],
            None => {
                return Err(AppError::Config(format!(
                    "Invalid repo: {}",
                    params.repo_full_name
                )));
            }
        };

        // 1. Acquire Installation Token (Strict multi-installation scoping)
        if params.from_webhook {
            let installation_id = params
                .installation_id
                .ok_or(AppError::MissingInstallationId)?;
            if !self
                .config
                .allowed_installation_ids
                .contains(&installation_id)
            {
                return Err(AppError::Unauthorized(
                    "GitHub App installation has not been approved by this self-hosted operator"
                        .into(),
                ));
            }
        }
        let token = if let Some(installation_id) = params.installation_id {
            info!(
                "Acquiring installation token for installation {} on {}",
                installation_id, params.repo_full_name
            );
            self.token_manager
                .get_token(
                    &self.http_client,
                    &self.config.github_app_id,
                    &self.config.github_app_private_key,
                    installation_id,
                )
                .await?
        } else if let Some(fallback_id) = self.config.github_app_installation_id {
            // Standalone legacy single-installation or CLI fallback only
            info!("Using configured fallback installation ID {}", fallback_id);
            self.token_manager
                .get_token(
                    &self.http_client,
                    &self.config.github_app_id,
                    &self.config.github_app_private_key,
                    fallback_id,
                )
                .await?
        } else {
            // Standalone or personal access token fallback if configured
            std::env::var("GITHUB_TOKEN").unwrap_or_default()
        };

        let gh_client = if !token.is_empty() {
            Some(GitHubClient::new(token.clone())?)
        } else {
            None
        };

        if params.mode == WorkflowMode::Fix && gh_client.is_none() {
            return Err(AppError::Unauthorized(
                "Fix mode requires an authenticated GitHub App installation".into(),
            ));
        }

        // 2. Validate Repository Belongs to Installation
        if let Some(gh) = &gh_client {
            let repo_data = gh.get_repository(owner, repo).await.map_err(|e| {
                AppError::Unauthorized(format!(
                    "Repository {}/{} is not accessible with installation token: {}",
                    owner, repo, e
                ))
            })?;

            if repo_data.archived && params.mode == WorkflowMode::Fix {
                return Err(AppError::Workflow(
                    "Repository is archived; modifications are prohibited".to_string(),
                ));
            }
        }

        // 3. Resolve the real PR before any authorization or push decision. A guessed
        // branch such as HEAD must never become a publishing target.
        let pr_context = if let Some(gh) = &gh_client {
            let ctx = PullRequestContext::fetch(gh, owner, repo, params.pr_number).await?;
            if params.mode == WorkflowMode::Fix && ctx.is_fork {
                return Err(AppError::Workflow(
                    "Fix publishing to fork pull requests is not supported; no base-repository branch will be modified".into(),
                ));
            }
            if params.mode == WorkflowMode::Fix
                && params
                    .head_sha
                    .as_ref()
                    .is_some_and(|sha| sha != &ctx.head_sha)
            {
                return Err(AppError::Workflow(
                    "Pull request head changed since authorization".into(),
                ));
            }
            params.head_sha = Some(ctx.head_sha.clone());
            params.pr_branch = ctx.head_branch.clone();
            params.base_branch = ctx.base_branch.clone();
            Some(ctx)
        } else {
            None
        };

        // 4. Authorization Gate: Check caller permission for Fix mode
        if params.mode == WorkflowMode::Fix
            && let Some(gh) = &gh_client
        {
            if params.actor.trim().is_empty() {
                return Err(AppError::Unauthorized(
                    "Fix requests must identify an actor".into(),
                ));
            }
            let permission = match gh
                .get_collaborator_permission(owner, repo, &params.actor)
                .await
            {
                Ok(perm) => perm,
                Err(e) => {
                    warn!(
                        "Failed to fetch collaborator permission for {}: {}",
                        params.actor, e
                    );
                    "none".to_string()
                }
            };

            if !is_permission_authorized_for_fix(&permission) {
                let reject_msg = format!(
                    "## ⛔ Fix Request Denied\n\nUser `@{}` has `{}` permission on repository `{}/{}`. Autonomous code fixing requires `write`, `maintain`, or `admin` access.\n\nTo request changes, please ask a repository collaborator with write access to approve or comment `@{} fix`.\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*",
                    params.actor, permission, owner, repo, self.config.bot_username
                );
                if !params.dry_run {
                    let _ = gh
                        .post_comment(owner, repo, params.pr_number, &reject_msg)
                        .await;
                }
                return Err(AppError::Unauthorized(format!(
                    "User '{}' does not have write access to repository (has '{}')",
                    params.actor, permission
                )));
            }
            info!(
                "User '{}' authorized for Fix mode (permission: '{}')",
                params.actor, permission
            );
        }

        // 5. Check Push Access Permissions for the actual PR head branch.
        if params.mode == WorkflowMode::Fix
            && !params.dry_run
            && let Some(gh) = &gh_client
        {
            let push_check = gh
                .check_push_access(owner, repo, &params.pr_branch, &self.config.bot_username)
                .await?;
            if !push_check.can_push {
                let reason = push_check
                    .reason
                    .unwrap_or_else(|| "Push access denied".to_string());
                let message = format!(
                    "## Skipped\n\nUnable to push to this branch: {reason}\n\nPlease ensure the OpenReview app has write access to this repository and branch.\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*"
                );
                if !params.dry_run {
                    let _ = gh
                        .post_comment(owner, repo, params.pr_number, &message)
                        .await;
                }
                return Err(AppError::Workflow(format!("Push access denied: {reason}")));
            }
        }

        // 6. Create Ephemeral Sandbox & Clone Branch
        let sandbox = self
            .sandbox_driver
            .create_sandbox(
                &params.repo_full_name,
                &params.pr_branch,
                &token,
                params.mode,
            )
            .await?;
        let work_dir = sandbox.work_dir().to_path_buf();

        // 7. Discover Skills
        let skill_paths = [
            work_dir.join(".agents").join("skills"),
            work_dir.join(".skills"),
            std::path::PathBuf::from(".agents/skills"),
        ];
        let search_refs: Vec<&std::path::Path> = skill_paths.iter().map(|p| p.as_path()).collect();
        let skills = discover_skills(&search_refs).await;
        info!(
            "Discovered {} applicable review skills for repo {}",
            skills.len(),
            params.repo_full_name
        );

        // 8. Run ReAct Agent with Mode Restrictions
        let initial_messages: Vec<ChatMessage> = params
            .messages
            .clone()
            .into_iter()
            .map(Into::into)
            .collect();
        let runner = AgentRunner::new(
            sandbox.as_ref(),
            gh_client.as_ref(),
            self.llm.as_ref(),
            &params.repo_full_name,
            params.pr_number,
            params.mode,
            params.dry_run,
            &skills,
            pr_context.as_ref(),
        );

        let mut agent_result = match runner.run(initial_messages).await {
            Ok(res) => res,
            Err(e) => {
                error!("Agent run failed: {e}");
                if let Some(gh) = &gh_client
                    && !params.dry_run
                {
                    let err_msg = format!(
                        "## Error\n\nAn error occurred while processing your request:\n\n```\n{e}\n```\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*"
                    );
                    let _ = gh
                        .post_comment(owner, repo, params.pr_number, &err_msg)
                        .await;
                }
                return Err(e);
            }
        };

        // 9. Safe Git Publishing Gate
        if params.mode == WorkflowMode::Fix && !agent_result.success {
            warn!("Fix agent did not finish successfully; publishing is forbidden");
        } else if params.mode == WorkflowMode::Fix {
            match WorkflowPublisher::evaluate_and_publish(
                sandbox.as_ref(),
                gh_client.as_ref(),
                &params,
            )
            .await
            {
                Ok(audit) => {
                    info!(
                        "Publish evaluation completed: action={}, success={}",
                        audit.action_taken, audit.success
                    );
                    if !audit.success {
                        agent_result.success = false;
                        agent_result.termination_reason =
                            AgentTerminationReason::VerificationFailure(
                                audit
                                    .reason
                                    .clone()
                                    .unwrap_or_else(|| audit.action_taken.clone()),
                            );
                        agent_result.error_message = audit.reason;
                    }
                }
                Err(e) => {
                    error!("Publish evaluation encountered an error: {e}");
                    let reason = e.to_string();
                    agent_result.success = false;
                    agent_result.termination_reason =
                        AgentTerminationReason::PublishingFailure(reason.clone());
                    agent_result.error_message = Some(reason.clone());
                    if let Some(gh) = &gh_client
                        && !params.dry_run
                    {
                        let _ = gh.post_comment(owner, repo, params.pr_number,
                            &format!("## Fix publishing failed\n\nThe changes were not confirmed as published. Error: `{reason}`")
                        ).await;
                    }
                }
            }
        } else {
            debug!("Review mode: skipping all git publishing evaluation");
        }

        Ok(agent_result)
    }
}
