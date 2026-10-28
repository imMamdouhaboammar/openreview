use crate::error::{AppError, Result};
use crate::sandbox::traits::{CommandOutput, SandboxDriver, SandboxInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::process::Command;
use tracing::info;

pub struct DockerSandbox {
    container_name: String,
    work_path: PathBuf,
    _temp_dir: TempDir,
}

#[async_trait]
impl SandboxInstance for DockerSandbox {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput> {
        let mut docker_args = vec!["exec", "-w", "/workspace", &self.container_name, cmd];
        docker_args.extend_from_slice(args);

        let output = Command::new("docker")
            .args(&docker_args)
            .output()
            .await
            .map_err(|e| AppError::Sandbox(format!("Docker exec failed: {e}")))?;

        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn run_bash(&self, script: &str) -> Result<CommandOutput> {
        self.run_command("bash", &["-c", script]).await
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        let target = self.work_path.join(path.trim_start_matches('/'));
        tokio::fs::read_to_string(&target)
            .await
            .map_err(|e| AppError::Sandbox(format!("Failed to read file in container: {e}")))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let target = self.work_path.join(path.trim_start_matches('/'));
        if let Some(parent) = target.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(&target, content)
            .await
            .map_err(|e| AppError::Sandbox(format!("Failed to write file in container: {e}")))
    }

    async fn has_uncommitted_changes(&self) -> Result<bool> {
        let out = self.run_command("git", &["diff", "--name-only"]).await?;
        Ok(!out.stdout.trim().is_empty())
    }

    async fn commit_and_push(&self, message: &str, branch: &str) -> Result<()> {
        let _ = self.run_command("git", &["add", "-A"]).await?;
        let _ = self
            .run_command("git", &["commit", "--no-verify", "-m", message])
            .await?;
        let _ = self.run_command("git", &["push", "origin", branch]).await?;
        Ok(())
    }

    fn work_dir(&self) -> &Path {
        &self.work_path
    }
}

pub struct DockerSandboxDriver {
    pub base_image: String,
}

impl Default for DockerSandboxDriver {
    fn default() -> Self {
        Self {
            base_image: "ghcr.io/astral-sh/uv:debian-slim".to_string(),
        }
    }
}

#[async_trait]
impl SandboxDriver for DockerSandboxDriver {
    async fn create_sandbox(
        &self,
        repo_full_name: &str,
        branch: &str,
        token: &str,
    ) -> Result<Box<dyn SandboxInstance>> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| AppError::Sandbox(format!("Temp dir allocation failed: {e}")))?;
        let work_path = temp_dir.path().to_path_buf();
        let container_name = format!("openreview-sb-{}", &uuid_v4()[..8]);

        info!(
            "Starting Docker container {container_name} with mount {}",
            work_path.display()
        );

        // Clone repository locally first into mount directory
        let clone_url = format!("https://x-access-token:{token}@github.com/{repo_full_name}.git");
        let clone_status = Command::new("git")
            .args(["clone", "--depth=1", "--branch", branch, &clone_url, "."])
            .current_dir(&work_path)
            .status()
            .await
            .map_err(|e| AppError::Sandbox(format!("git clone execution failed: {e}")))?;

        if !clone_status.success() {
            return Err(AppError::Sandbox(
                "git clone failed for docker sandbox".to_string(),
            ));
        }

        // Start background container with mounted volume
        let run_status = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                &container_name,
                "-v",
                &format!("{}:/workspace", work_path.display()),
                "-w",
                "/workspace",
                &self.base_image,
                "sleep",
                "1800",
            ])
            .status()
            .await
            .map_err(|e| AppError::Sandbox(format!("docker run failed: {e}")))?;

        if !run_status.success() {
            return Err(AppError::Sandbox(
                "Failed to launch docker sandbox container".to_string(),
            ));
        }

        Ok(Box::new(DockerSandbox {
            container_name,
            work_path,
            _temp_dir: temp_dir,
        }))
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{now:x}")
}
