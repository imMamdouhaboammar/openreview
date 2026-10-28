use crate::agent::runner::{AgentRunResult, AgentRunner};
use crate::config::Config;
use crate::error::{AppError, Result};
use crate::github::auth::get_installation_token;
use crate::github::client::GitHubClient;
use crate::llm::traits::LlmProvider;
use crate::llm::types::ChatMessage;
use crate::sandbox::traits::SandboxDriver;
use crate::skills::discover_skills;
use crate::workflow::params::WorkflowParams;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct WorkflowCoordinator {
    pub config: Config,
    pub llm: Arc<dyn LlmProvider>,
    pub sandbox_driver: Arc<dyn SandboxDriver>,
    pub http_client: reqwest::Client,
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
        }
    }

    pub async fn execute_bot_workflow(&self, params: WorkflowParams) -> Result<AgentRunResult> {
        let [owner, repo] = match params.repo_full_name.split_once('/') {
            Some((o, r)) => [o, r],
            None => {
                return Err(AppError::Config(format!(
                    "Invalid repo: {}",
                    params.repo_full_name
                )));
            }
        };

        // 1. Acquire Installation Token
        let token = if let Some(installation_id) = self.config.github_app_installation_id {
            get_installation_token(
                &self.http_client,
                &self.config.github_app_id,
                &self.config.github_app_private_key,
                installation_id,
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

        // 2. Check Push Access Permissions
        if let Some(gh) = &gh_client {
            let push_check = gh
                .check_push_access(owner, repo, &params.pr_branch, &self.config.bot_username)
                .await?;
            if !push_check.can_push {
                let reason = push_check
                    .reason
                    .unwrap_or_else(|| "Push access denied".to_string());
                let message = format!(
                    "## Skipped\n\nUnable to access this branch: {reason}\n\nPlease ensure the OpenReview app has write access to this repository and branch.\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*"
                );
                let _ = gh
                    .post_comment(owner, repo, params.pr_number, &message)
                    .await;
                return Err(AppError::Workflow(format!("Push access denied: {reason}")));
            }
        }

        // 3. Create Sandbox & Clone Branch
        let sandbox = self
            .sandbox_driver
            .create_sandbox(&params.repo_full_name, &params.pr_branch, &token)
            .await?;
        let work_dir = sandbox.work_dir().to_path_buf();

        // 4. Discover Skills (local repo + server defaults)
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

        // 5. Run ReAct Agent
        let initial_messages: Vec<ChatMessage> =
            params.messages.into_iter().map(Into::into).collect();
        let runner = AgentRunner::new(
            sandbox.as_ref(),
            gh_client.as_ref(),
            self.llm.as_ref(),
            &params.repo_full_name,
            params.pr_number,
            &skills,
        );

        let agent_result = match runner.run(initial_messages).await {
            Ok(res) => res,
            Err(e) => {
                error!("Agent run failed: {e}");
                if let Some(gh) = &gh_client {
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

        // 6. Check for uncommitted changes and commit/push
        match sandbox.has_uncommitted_changes().await {
            Ok(true) => {
                info!(
                    "Uncommitted changes detected in sandbox. Committing and pushing to {}",
                    params.pr_branch
                );
                if let Err(e) = sandbox
                    .commit_and_push("openreview: apply changes", &params.pr_branch)
                    .await
                {
                    error!("Failed to commit and push changes: {e}");
                    if let Some(gh) = &gh_client {
                        let _ = gh
                            .post_comment(
                                owner,
                                repo,
                                params.pr_number,
                                &format!(
                                    "⚠️ Failed to push changes to branch `{}`: {}",
                                    params.pr_branch, e
                                ),
                            )
                            .await;
                    }
                }
            }
            Ok(false) => {
                debug!("No file modifications made by agent in sandbox");
            }
            Err(e) => {
                warn!("Could not evaluate git diff in sandbox: {e}");
            }
        }

        Ok(agent_result)
    }

    pub async fn handle_reaction_event(
        &self,
        repo_full_name: &str,
        pr_number: u64,
        comment_id: u64,
        reaction: &str,
        user_login: &str,
    ) -> Result<()> {
        let [owner, repo] = match repo_full_name.split_once('/') {
            Some((o, r)) => [o, r],
            None => return Err(AppError::Config(format!("Invalid repo: {repo_full_name}"))),
        };

        let token = if let Some(installation_id) = self.config.github_app_installation_id {
            get_installation_token(
                &self.http_client,
                &self.config.github_app_id,
                &self.config.github_app_private_key,
                installation_id,
            )
            .await?
        } else {
            std::env::var("GITHUB_TOKEN").unwrap_or_default()
        };

        let gh_client = GitHubClient::new(token)?;

        match reaction {
            "+1" | "heart" => {
                info!(
                    "Reaction {reaction} from {user_login} on comment {comment_id}. Re-triggering apply workflow."
                );
                let pr = gh_client.get_pull_request(owner, repo, pr_number).await?;
                let params = WorkflowParams {
                    repo_full_name: repo_full_name.to_string(),
                    pr_number,
                    base_branch: pr.base.r#ref,
                    pr_branch: pr.head.r#ref,
                    comment_id: Some(comment_id),
                    messages: vec![crate::workflow::params::ThreadMessage {
                        role: "user".to_string(),
                        content: format!(
                            "User {user_login} approved changes on comment {comment_id} with reaction {reaction}. Please apply the suggested edits directly to the repository."
                        ),
                    }],
                };
                self.execute_bot_workflow(params).await?;
            }
            "-1" | "confused" => {
                info!(
                    "Reaction {reaction} from {user_login} on comment {comment_id}. Acknowledging skip."
                );
                let skip_text = "👀 Got it, skipping that. Mention me with feedback if you'd like a different approach.\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*";
                gh_client
                    .post_comment(owner, repo, pr_number, skip_text)
                    .await?;
            }
            other => {
                debug!("Ignoring unsupported reaction '{other}'");
            }
        }

        Ok(())
    }
}
