use clap::Parser;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sha2::digest::KeyInit;
use std::sync::Arc;
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

use openreview::cli::{Cli, Commands};
use openreview::config::Config;
use openreview::error::{AppError, Result};
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
    if let Commands::GithubAppUrl {
        domain,
        name,
        org,
        public,
    } = &cli.command
    {
        println!(
            "{}",
            openreview::github::registration::github_app_registration_url(
                domain,
                name,
                org.as_deref(),
                *public,
            )?
        );
        return Ok(());
    }
    let config = Config::from_env()?;

    match cli.command {
        Commands::Serve { host, port } => {
            config.validate_for_serve()?;
            if config.allowed_installation_ids.is_empty() {
                warn!(
                    "No GitHub installations approved; configure ALLOWED_INSTALLATION_IDS to process PR commands"
                );
            }
            validate_isolated_docker_backend(&config)?;
            let max_concurrent_jobs = config.max_concurrent_jobs;
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
                jobs: Arc::new(openreview::jobs::JobManager::persistent(
                    "/data/openreview-jobs.json",
                    max_concurrent_jobs,
                )?),
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
                from_webhook: false,
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
        Commands::Action { dry_run } => {
            let mut config = config;
            if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
                return Err(AppError::Unauthorized(
                    "The action command requires the authenticated GitHub Actions environment"
                        .into(),
                ));
            }
            let event_name = std::env::var("GITHUB_EVENT_NAME")
                .map_err(|_| AppError::Config("GITHUB_EVENT_NAME is required".into()))?;
            let repo = std::env::var("GITHUB_REPOSITORY")
                .map_err(|_| AppError::Config("GITHUB_REPOSITORY is required".into()))?;
            let path = std::env::var("GITHUB_EVENT_PATH")
                .map_err(|_| AppError::Config("GITHUB_EVENT_PATH is required".into()))?;
            let event_data = std::fs::read(&path).map_err(|e| {
                AppError::Config(format!("Unable to read authenticated Actions event: {e}"))
            })?;
            let event: serde_json::Value = serde_json::from_slice(&event_data)
                .map_err(|e| AppError::Config(format!("Invalid GitHub Actions event JSON: {e}")))?;
            let Some(mut params) = openreview::action::parse_action_event(
                &event_name,
                &event,
                &repo,
                &config.bot_username,
            )?
            else {
                info!("No actionable OpenReview command in this GitHub Actions event");
                return Ok(());
            };
            if std::env::var("GITHUB_TOKEN")
                .ok()
                .is_none_or(|token| token.trim().is_empty())
            {
                return Err(AppError::Unauthorized(
                    "GITHUB_TOKEN is required for repository-scoped Actions authentication".into(),
                ));
            }
            // An Actions invocation must use the calling repository's own
            // ephemeral token. Ignore unrelated App-installation defaults
            // that may be present in an operator's runner environment.
            config.github_app_installation_id = None;
            config.validate_llm_credentials()?;
            if config.sandbox_driver != "docker" {
                return Err(AppError::Config(
                    "GitHub Actions execution requires SANDBOX_DRIVER=docker; Local mode cannot isolate untrusted PR code".into(),
                ));
            }
            validate_isolated_docker_backend(&config)?;

            params.dry_run = dry_run;
            let llm = Arc::from(create_llm_provider(&config)?);
            let sandbox: Arc<dyn SandboxDriver> = Arc::new(DockerSandboxDriver::default());
            let coordinator = WorkflowCoordinator::new(config, llm, sandbox);
            let outcome = coordinator.execute_bot_workflow(params).await?;
            if !outcome.success {
                return Err(AppError::Workflow(
                    "GitHub Actions agent did not complete successfully".into(),
                ));
            }
            info!(
                "GitHub Actions agent completed {} steps",
                outcome.steps_taken
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
        Commands::GithubAppUrl { .. } => unreachable!("handled before reading credentials"),
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

fn validate_isolated_docker_backend(config: &Config) -> Result<()> {
    use openreview::error::AppError;
    use std::path::Path;
    let output = std::process::Command::new("docker")
        .args(["info", "--format", "{{json .SecurityOptions}}"])
        .output()
        .map_err(|e| AppError::Config(format!("Docker execution backend is unavailable: {e}")))?;
    let security = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || !security.contains("rootless") {
        return Err(AppError::Config(
            "Webhook execution requires a reachable dedicated rootless Docker daemon (DOCKER_HOST); refusing host/rootful execution".into(),
        ));
    }
    let cgroups = std::process::Command::new("docker")
        .args(["info", "--format", "{{.CgroupDriver}}/{{.CgroupVersion}}"])
        .output()
        .map_err(|e| AppError::Config(format!("Unable to inspect Docker cgroup support: {e}")))?;
    if !cgroups.status.success()
        || !effective_cgroup_driver(&String::from_utf8_lossy(&cgroups.stdout))
    {
        return Err(AppError::Config(
            "Rootless Docker must use systemd cgroup driver and cgroup v2; otherwise memory, CPU and process limits may be silently ignored".into(),
        ));
    }
    let image = std::process::Command::new("docker")
        .args(["image", "inspect", "openreview-sandbox:latest"])
        .output()
        .map_err(|e| AppError::Config(format!("Unable to inspect sandbox image: {e}")))?;
    if !image.status.success() {
        return Err(AppError::Config(
            "Sandbox image missing on isolated daemon; build Dockerfile.sandbox before starting service".into(),
        ));
    }
    // The service runs inside Compose, but the rootless Docker daemon resolves
    // volume paths on its *own host*. A path visible only in the service
    // container will otherwise create an empty or inaccessible sandbox mount.
    let root = Path::new(&config.sandbox_workspace_dir);
    if !root.is_absolute() || !root.is_dir() {
        return Err(AppError::Config(format!(
            "SANDBOX_WORKSPACE_DIR must be an existing absolute directory shared with the rootless Docker host: {}",
            root.display()
        )));
    }
    let probe = tempfile::Builder::new()
        .prefix("openreview-docker-probe-")
        .tempdir_in(root)
        .map_err(|e| AppError::Config(format!("Sandbox workspace is not writable: {e}")))?;
    std::fs::write(probe.path().join("probe"), b"sandbox-mount-ready")
        .map_err(|e| AppError::Config(format!("Cannot initialize sandbox workspace probe: {e}")))?;
    let probe_result = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network=none",
            "--read-only",
            "--cap-drop=ALL",
            "--mount",
            &format!(
                "type=bind,source={},target=/workspace",
                probe.path().display()
            ),
            "openreview-sandbox:latest",
            "sh",
            "-c",
            "cat /workspace/probe && printf writable > /workspace/probe-write",
        ])
        .output()
        .map_err(|e| AppError::Config(format!("Sandbox mount validation could not run: {e}")))?;
    if !probe_result.status.success()
        || probe_result.stdout != b"sandbox-mount-ready"
        || std::fs::read(probe.path().join("probe-write"))
            .ok()
            .as_deref()
            != Some(b"writable")
    {
        return Err(AppError::Config(
            "Rootless Docker daemon cannot read/write shared sandbox workspace; check matching host paths and permissions".into(),
        ));
    }
    Ok(())
}

fn effective_cgroup_driver(info: &str) -> bool {
    info.trim() == "systemd/2"
}

#[cfg(test)]
mod docker_preflight_tests {
    use super::*;

    #[test]
    fn refuse_rootless_daemon_without_enforced_cgroup_driver() {
        assert!(effective_cgroup_driver("systemd/2\n"));
        assert!(!effective_cgroup_driver("none/2"));
        assert!(!effective_cgroup_driver("systemd/1"));
    }
}
