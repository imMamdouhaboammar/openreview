use crate::error::{AppError, Result};
use crate::sandbox::path::safe_resolve_path;
use crate::sandbox::traits::{CommandOutput, SandboxDriver, SandboxInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::process::Command;
use tracing::info;
use uuid::Uuid;

pub struct DockerSandbox {
    container_name: String,
    work_path: PathBuf,
    repo_full_name: String,
    token: String,
    _temp_dir: TempDir,
}

impl Drop for DockerSandbox {
    fn drop(&mut self) {
        let name = self.container_name.clone();
        // Synchronously initiate forced container destruction on teardown to prevent orphan containers
        let _ = std::process::Command::new("docker")
            .args(["rm", "-f", &name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        info!("Torn down and cleaned up Docker sandbox container {name}");
    }
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
        let target = safe_resolve_path(&self.work_path, path)?;
        if !target.exists() {
            return Err(AppError::Sandbox(format!("File not found: {path}")));
        }
        tokio::fs::read_to_string(&target)
            .await
            .map_err(|e| AppError::Sandbox(format!("Failed to read file in container: {e}")))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let target = safe_resolve_path(&self.work_path, path)?;
        if let Some(parent) = target.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        tokio::fs::write(&target, content)
            .await
            .map_err(|e| AppError::Sandbox(format!("Failed to write file in container: {e}")))
    }

    async fn has_uncommitted_changes(&self) -> Result<bool> {
        let out = self.run_command("git", &["status", "--porcelain"]).await?;
        Ok(!out.stdout.trim().is_empty())
    }

    async fn commit_and_push(&self, message: &str, branch: &str) -> Result<()> {
        let add_out = self.run_command("git", &["add", "-A"]).await?;
        if add_out.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "git add failed: {}",
                add_out.stderr
            )));
        }

        let commit_out = self
            .run_command("git", &["commit", "--no-verify", "-m", message])
            .await?;
        if commit_out.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "git commit failed: {}",
                commit_out.stderr
            )));
        }

        let push_target = if !self.token.is_empty() {
            format!(
                "https://x-access-token:{}@github.com/{}.git",
                self.token, self.repo_full_name
            )
        } else {
            "origin".to_string()
        };

        let push_out = self
            .run_command("git", &["push", &push_target, &format!("HEAD:{branch}")])
            .await?;
        if push_out.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "git push failed: {}",
                push_out.stderr
            )));
        }

        info!("Successfully committed and pushed changes to branch {branch}");
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
        let container_name = format!("openreview-sb-{}", &Uuid::new_v4().to_string()[..8]);

        info!(
            "Starting Docker container {container_name} with mount {}",
            work_path.display()
        );

        // Clone repository locally first into mount directory
        let clone_url = if !token.is_empty() {
            format!("https://x-access-token:{token}@github.com/{repo_full_name}.git")
        } else {
            format!("https://github.com/{repo_full_name}.git")
        };

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

        // CRITICAL SECURITY FIX: Sanitize origin remote URL immediately so that the token
        // is NEVER stored on disk where container execution can read it
        let clean_origin = format!("https://github.com/{repo_full_name}.git");
        let _ = Command::new("git")
            .args(["remote", "set-url", "origin", &clean_origin])
            .current_dir(&work_path)
            .output()
            .await;

        // Start background container with mounted volume and security restrictions:
        // - Memory limit: 2GB
        // - CPU limit: 2 cores
        // - PIDs limit: 100
        // - Drop all Linux capabilities
        // - No new privileges
        let run_status = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                &container_name,
                "--memory=2g",
                "--cpus=2.0",
                "--pids-limit=100",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges:true",
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
            repo_full_name: repo_full_name.to_string(),
            token: token.to_string(),
            _temp_dir: temp_dir,
        }))
    }
}
