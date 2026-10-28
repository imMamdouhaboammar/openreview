use clap::Parser;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sha2::digest::KeyInit;
use std::sync::Arc;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

use openreview::cli::{Cli, Commands};
use openreview::config::Config;
use openreview::error::Result;
use openreview::llm::create_llm_provider;
use openreview::sandbox::{DockerSandboxDriver, LocalSandboxDriver, SandboxDriver};
use openreview::server::{AppState, Metrics, create_router};
use openreview::workflow::{WorkflowCoordinator, WorkflowParams};

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let cli = Cli::parse();
    let config = Config::from_env()?;

    match cli.command {
        Commands::Serve { host, port } => {
            info!("Starting OpenReview-RS daemon on {}:{}", host, port);

            let llm = Arc::from(create_llm_provider(&config)?);
            let sandbox_driver: Arc<dyn SandboxDriver> = match config.sandbox_driver.as_str() {
                "docker" => Arc::new(DockerSandboxDriver::default()),
                _ => Arc::new(LocalSandboxDriver),
            };

            let coordinator = Arc::new(WorkflowCoordinator::new(
                config.clone(),
                llm,
                sandbox_driver,
            ));

            let state = AppState {
                config,
                coordinator,
                metrics: Arc::new(Metrics::new()),
                jobs: Arc::new(openreview::jobs::JobManager::default()),
            };

            let app = create_router(state);
            let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
                .await
                .map_err(|e| {
                    openreview::error::AppError::Internal(format!(
                        "Failed to bind to {host}:{port}: {e}"
                    ))
                })?;

            info!("OpenReview-RS listening on http://{host}:{port}");
            axum::serve(listener, app)
                .await
                .map_err(|e| openreview::error::AppError::Internal(format!("Server error: {e}")))?;
        }
        Commands::Review {
            repo,
            pr,
            branch,
            dry_run,
        } => {
            info!("Running standalone review on {repo} #{pr} (dry_run: {dry_run})");

            let llm = Arc::from(create_llm_provider(&config)?);
            let sandbox_driver: Arc<dyn SandboxDriver> = Arc::new(LocalSandboxDriver);
            let coordinator = WorkflowCoordinator::new(config, llm, sandbox_driver);

            let pr_branch = branch.unwrap_or_else(|| "main".to_string());
            let params = WorkflowParams {
                repo_full_name: repo,
                pr_number: pr,
                base_branch: "main".to_string(),
                pr_branch,
                comment_id: None,
                installation_id: None,
                mode: openreview::workflow::WorkflowMode::Review,
                actor: "cli".to_string(),
                dry_run,
                head_sha: None,
                messages: vec![openreview::workflow::ThreadMessage {
                    role: "user".to_string(),
                    content: format!("Please review pull request #{pr}"),
                }],
            };

            let outcome = coordinator.execute_bot_workflow(params).await?;
            info!(
                "Review completed: {} steps taken, {} comments posted",
                outcome.steps_taken, outcome.replies_posted
            );
        }
        Commands::VerifyConfig => {
            println!("Validating OpenReview-RS configuration...");
            println!("  Bot Username:    {}", config.bot_username);
            println!("  LLM Provider:    {}", config.llm_provider);
            println!("  Active Model:    {}", config.model_name);
            println!("  Sandbox Driver:  {}", config.sandbox_driver);
            println!("  Server Address:  {}:{}", config.host, config.port);

            match config.validate_github_credentials() {
                Ok(_) => println!("  GitHub App Keys: [OK] Present and valid"),
                Err(e) => println!("  GitHub App Keys: [WARN] {}", e),
            }

            match config.validate_llm_credentials() {
                Ok(_) => println!(
                    "  LLM Credentials: [OK] Present and valid for {}",
                    config.llm_provider
                ),
                Err(e) => println!("  LLM Credentials: [WARN] {}", e),
            }

            println!("\nConfiguration check complete.");
        }
        Commands::TestWebhook { url, pr, repo } => {
            info!("Sending test signed webhook to {url}");
            let secret = if config.github_app_webhook_secret.is_empty() {
                "test-secret-123"
            } else {
                &config.github_app_webhook_secret
            };

            let payload = serde_json::json!({
                "action": "created",
                "issue": {
                    "id": 1,
                    "number": pr,
                    "title": "Test PR",
                    "pull_request": {
                        "url": format!("https://api.github.com/repos/{repo}/pulls/{pr}"),
                        "html_url": format!("https://github.com/{repo}/pull/{pr}")
                    }
                },
                "comment": {
                    "id": 1001,
                    "body": format!("@{} please review this PR", config.bot_username),
                    "user": {
                        "id": 42,
                        "login": "developer"
                    },
                    "html_url": format!("https://github.com/{repo}/pull/{pr}#issuecomment-1001")
                },
                "repository": {
                    "id": 999,
                    "name": "repo",
                    "full_name": repo,
                    "owner": {
                        "id": 1,
                        "login": "example"
                    }
                }
            });

            let body_bytes = serde_json::to_vec(&payload).unwrap();
            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
            mac.update(&body_bytes);
            let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

            let client = reqwest::Client::new();
            let resp = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-GitHub-Event", "issue_comment")
                .header("X-Hub-Signature-256", signature)
                .body(body_bytes)
                .send()
                .await
                .map_err(|e| {
                    openreview::error::AppError::Internal(format!("Request failed: {e}"))
                })?;

            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            println!("Response ({}): {}", status, text);
        }
    }

    Ok(())
}
