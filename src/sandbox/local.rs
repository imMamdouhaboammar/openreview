use crate::error::{AppError, Result};
use crate::sandbox::detect::detect_install_command;
use crate::sandbox::traits::{CommandOutput, SandboxDriver, SandboxInstance};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

pub struct LocalSandbox {
    _temp_dir: TempDir,
    work_path: PathBuf,
}

impl LocalSandbox {
    pub fn new(temp_dir: TempDir) -> Self {
        let work_path = temp_dir.path().to_path_buf();
        Self {
            _temp_dir: temp_dir,
            work_path,
        }
    }

    fn resolve_path(&self, relative_path: &str) -> PathBuf {
        let trimmed = relative_path.trim_start_matches('/');
        self.work_path.join(trimmed)
    }
}

#[async_trait]
impl SandboxInstance for LocalSandbox {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput> {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .current_dir(&self.work_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = command
            .spawn()
            .map_err(|e| AppError::Sandbox(format!("Failed to spawn command '{cmd}': {e}")))?;

        let output = timeout(DEFAULT_TIMEOUT, child.wait_with_output())
            .await
            .map_err(|_| AppError::Sandbox(format!("Command '{cmd}' timed out after 60s")))?
            .map_err(|e| AppError::Sandbox(format!("Failed to await command '{cmd}': {e}")))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    async fn run_bash(&self, script: &str) -> Result<CommandOutput> {
        let full_script = format!(
            "export PATH=\"$HOME/.local/bin:$PATH\" && cd \"{}\" && {}",
            self.work_path.display(),
            script
        );
        self.run_command("bash", &["-c", &full_script]).await
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        let target = self.resolve_path(path);
        if !target.exists() {
            return Err(AppError::Sandbox(format!("File not found: {path}")));
        }

        fs::read_to_string(&target)
            .await
            .map_err(|e| AppError::Sandbox(format!("Failed to read file {path}: {e}")))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let target = self.resolve_path(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::Sandbox(format!("Failed to create parent dirs for {path}: {e}"))
            })?;
        }

        fs::write(&target, content)
            .await
            .map_err(|e| AppError::Sandbox(format!("Failed to write file {path}: {e}")))
    }

    async fn has_uncommitted_changes(&self) -> Result<bool> {
        let res = self.run_command("git", &["diff", "--name-only"]).await?;
        Ok(!res.stdout.trim().is_empty())
    }

    async fn commit_and_push(&self, message: &str, branch: &str) -> Result<()> {
        let add_res = self.run_command("git", &["add", "-A"]).await?;
        if add_res.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "git add failed: {}",
                add_res.stderr
            )));
        }

        let commit_res = self
            .run_command("git", &["commit", "--no-verify", "-m", message])
            .await?;
        if commit_res.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "git commit failed: {}",
                commit_res.stderr
            )));
        }

        let push_res = self.run_command("git", &["push", "origin", branch]).await?;
        if push_res.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "git push failed: {}",
                push_res.stderr
            )));
        }

        info!("Successfully committed and pushed changes to branch {branch}");
        Ok(())
    }

    fn work_dir(&self) -> &Path {
        &self.work_path
    }
}

pub struct LocalSandboxDriver;

#[async_trait]
impl SandboxDriver for LocalSandboxDriver {
    async fn create_sandbox(
        &self,
        repo_full_name: &str,
        branch: &str,
        token: &str,
    ) -> Result<Box<dyn SandboxInstance>> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            AppError::Sandbox(format!("Failed to allocate temporary directory: {e}"))
        })?;
        let work_path = temp_dir.path().to_path_buf();

        info!("Initializing local sandbox in {}", work_path.display());

        let clone_url = format!("https://x-access-token:{token}@github.com/{repo_full_name}.git");
        let clone_cmd = Command::new("git")
            .args(["clone", "--depth=1", "--branch", branch, &clone_url, "."])
            .current_dir(&work_path)
            .output()
            .await
            .map_err(|e| AppError::Sandbox(format!("git clone execution failed: {e}")))?;

        if !clone_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&clone_cmd.stderr);
            return Err(AppError::Sandbox(format!("git clone failed: {stderr}")));
        }

        // Configure git identity & disable hooks
        let git_configs = [
            ("config", vec!["user.name", "openreview[bot]"]),
            (
                "config",
                vec!["user.email", "openreview[bot]@users.noreply.github.com"],
            ),
            ("config", vec!["core.hooksPath", "/dev/null"]),
        ];

        for (action, args) in git_configs {
            let mut cmd = Command::new("git");
            cmd.arg(action).args(&args).current_dir(&work_path);
            let _ = cmd.output().await;
        }

        // Detect and run dependency installation if available
        if let Some(install_cmd) = detect_install_command(&work_path) {
            info!(
                "Detected project package manager: {} {:?}",
                install_cmd.cmd, install_cmd.args
            );
            let mut proc = Command::new(&install_cmd.cmd);
            proc.args(&install_cmd.args).current_dir(&work_path);
            if matches!(proc.output().await, Ok(out) if !out.status.success()) {
                warn!("Dependency installation exited with non-zero status");
            }
        }

        Ok(Box::new(LocalSandbox::new(temp_dir)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_sandbox_file_ops() {
        let temp = tempfile::tempdir().unwrap();
        let sandbox = LocalSandbox::new(temp);

        sandbox
            .write_file("nested/test.txt", "hello openreview")
            .await
            .unwrap();
        let content = sandbox.read_file("nested/test.txt").await.unwrap();
        assert_eq!(content, "hello openreview");

        let bash_out = sandbox.run_bash("cat nested/test.txt").await.unwrap();
        assert_eq!(bash_out.exit_code, 0);
        assert_eq!(bash_out.stdout.trim(), "hello openreview");
    }
}
