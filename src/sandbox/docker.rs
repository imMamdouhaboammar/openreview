use crate::error::{AppError, Result};
use crate::sandbox::path::safe_resolve_path;
use crate::sandbox::traits::{CommandOutput, SandboxDriver, SandboxInstance};
use crate::workflow::command::WorkflowMode;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

pub struct DockerSandbox {
    container_name: String,
    work_path: PathBuf,
    token: String,
    _temp_dir: TempDir,
    // Never bind mount this directory into the untrusted container.
    trusted_git_dir: TempDir,
}

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const VERIFICATION_TIMEOUT: Duration = Duration::from_secs(600);

/// Rootless Docker may accept --memory/--cpus/--pids-limit while leaving the
/// cgroup controller unset. Never execute untrusted code in that state.
fn resource_limits_enforced(memory: &str, pids: &str, cpu: &str) -> bool {
    let mem = memory.trim().parse::<u64>().ok();
    let pids = pids.trim().parse::<u64>().ok();
    let mut cpu_parts = cpu.split_whitespace();
    let quota = cpu_parts.next().and_then(|v| v.parse::<u64>().ok());
    let period = cpu_parts.next().and_then(|v| v.parse::<u64>().ok());
    matches!(mem, Some(n) if n > 0 && n <= 2 * 1024 * 1024 * 1024)
        && matches!(pids, Some(n) if n > 0 && n <= 100)
        && matches!((quota, period), (Some(q), Some(p)) if q > 0 && p > 0 && p.checked_mul(2).is_some_and(|max| q <= max))
        && cpu_parts.next().is_none()
}

async fn verify_container_limits(container: &str) -> Result<()> {
    let mut values = Vec::with_capacity(3);
    for file in ["memory.max", "pids.max", "cpu.max"] {
        let path = format!("/sys/fs/cgroup/{file}");
        let output = timeout(
            COMMAND_TIMEOUT,
            Command::new("docker")
                .args(["exec", container, "cat", &path])
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| AppError::Sandbox("Container resource check timed out".into()))?
        .map_err(|e| AppError::Sandbox(format!("Container resource check failed: {e}")))?;
        if !output.status.success() {
            return Err(AppError::Sandbox(format!(
                "Cannot inspect isolated container {file}: cgroup constraints unproven"
            )));
        }
        values.push(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    if !resource_limits_enforced(&values[0], &values[1], &values[2]) {
        return Err(AppError::Sandbox(
            "Isolated Docker did not enforce configured memory, CPU and PIDs limits".into(),
        ));
    }
    Ok(())
}

fn sandbox_mount(work_path: &Path, mode: WorkflowMode) -> String {
    let mut mount = format!("type=bind,source={},target=/workspace", work_path.display());
    if mode == WorkflowMode::Review {
        mount.push_str(",readonly");
    }
    mount
}

fn remove_container(name: &str) {
    let result = std::process::Command::new("docker")
        .args(["rm", "-f", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    if matches!(result, Ok(status) if status.success()) {
        info!("Removed Docker sandbox container {name}");
    } else {
        warn!("Could not confirm removal of Docker sandbox container {name}");
    }
}

struct ContainerCreationGuard {
    name: String,
    armed: bool,
}

impl Drop for ContainerCreationGuard {
    fn drop(&mut self) {
        if self.armed {
            remove_container(&self.name);
        }
    }
}

impl DockerSandbox {
    async fn run_command_with_limit(
        &self,
        cmd: &str,
        args: &[&str],
        limit: Duration,
    ) -> Result<CommandOutput> {
        let mut docker_args = vec!["exec", "-w", "/workspace", &self.container_name, cmd];
        docker_args.extend_from_slice(args);

        let output = timeout(
            limit,
            Command::new("docker")
                .args(&docker_args)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| {
            // Dropping the Docker CLI process does not stop its exec process.
            // Destroy the sandbox container so timed-out code cannot keep running.
            remove_container(&self.container_name);
            AppError::Sandbox(format!("Docker command timed out after {limit:?}"))
        })?
        .map_err(|e| AppError::Sandbox(format!("Docker exec failed: {e}")))?;

        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn trusted_git(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command
            .arg(format!(
                "--git-dir={}",
                self.trusted_git_dir.path().join(".git").display()
            ))
            .arg(format!("--work-tree={}", self.work_path.display()))
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.attributesFile=/dev/null",
                "-c",
                "user.name=OpenReview Agent",
                "-c",
                "user.email=openreview-agent@users.noreply.github.com",
            ])
            .args(args)
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("HOME", "/nonexistent")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command
    }

    async fn trusted_git_checked(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = timeout(COMMAND_TIMEOUT, self.trusted_git(args).output())
            .await
            .map_err(|_| AppError::Sandbox("Trusted git command timed out".into()))?
            .map_err(|e| AppError::Sandbox(format!("Trusted git command failed to start: {e}")))?;
        let result = CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        if result.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "Trusted git failed ({}): {}",
                result.exit_code, result.stderr
            )));
        }
        Ok(result)
    }
}

impl Drop for DockerSandbox {
    fn drop(&mut self) {
        remove_container(&self.container_name);
    }
}

#[async_trait]
impl SandboxInstance for DockerSandbox {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput> {
        if cmd == "git" && args == ["rev-parse", "HEAD"] {
            return self.trusted_git_checked(args).await;
        }
        self.run_command_with_limit(cmd, args, COMMAND_TIMEOUT)
            .await
    }

    async fn run_bash(&self, script: &str) -> Result<CommandOutput> {
        self.run_command("bash", &["-c", script]).await
    }

    async fn run_verification(&self, script: &str) -> Result<CommandOutput> {
        self.run_command_with_limit("bash", &["-c", script], VERIFICATION_TIMEOUT)
            .await
    }

    async fn read_file(&self, path: &str) -> Result<String> {
        let target = safe_resolve_path(&self.work_path, path)?;
        if !target.exists() {
            return Err(AppError::Sandbox(format!("File not found: {path}")));
        }
        let relative = target
            .strip_prefix(&self.work_path)
            .map_err(|_| AppError::PathTraversal("Path outside workspace".into()))?;
        let container_path = format!("/workspace/{}", relative.display());
        let output = self.run_command("cat", &["--", &container_path]).await?;
        if output.exit_code != 0 {
            return Err(AppError::Sandbox(format!(
                "Container read failed: {}",
                output.stderr
            )));
        }
        Ok(output.stdout)
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let target = safe_resolve_path(&self.work_path, path)?;
        let relative = target
            .strip_prefix(&self.work_path)
            .map_err(|_| AppError::PathTraversal("Path outside workspace".into()))?;
        let container_path = format!("/workspace/{}", relative.display());
        let parent = Path::new(&container_path)
            .parent()
            .ok_or_else(|| AppError::Sandbox("Invalid target parent".into()))?
            .to_string_lossy()
            .into_owned();
        let mut child = Command::new("docker")
            .args([
                "exec",
                "-i",
                "-w",
                "/workspace",
                &self.container_name,
                "sh",
                "-c",
                "mkdir -p -- \"$1\" && cat > \"$2\"",
                "--",
                &parent,
                &container_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::Sandbox(format!("Docker write command failed to start: {e}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(content.as_bytes())
                .await
                .map_err(|e| AppError::Sandbox(format!("Container write failed: {e}")))?;
        }
        let output = timeout(COMMAND_TIMEOUT, child.wait_with_output())
            .await
            .map_err(|_| AppError::Sandbox("Docker write timed out".into()))?
            .map_err(|e| AppError::Sandbox(format!("Docker write failed: {e}")))?;
        if !output.status.success() {
            return Err(AppError::Sandbox(format!(
                "Container write failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    async fn has_uncommitted_changes(&self) -> Result<bool> {
        let out = self.trusted_git_checked(&["status", "--porcelain"]).await?;
        Ok(!out.stdout.trim().is_empty())
    }

    async fn commit_and_push(&self, message: &str, branch: &str) -> Result<()> {
        self.trusted_git_checked(&["add", "-A"]).await?;
        self.trusted_git_checked(&["commit", "--no-verify", "-m", message])
            .await?;

        // Credential helper is outside the mount, and the token is never in a URL or argv.
        let askpass = self.trusted_git_dir.path().join("askpass.sh");
        std::fs::write(&askpass, "#!/bin/sh\ncase \"$1\" in\n*Username*) printf '%s\\n' x-access-token;;\n*Password*) printf '%s\\n' \"$OPENREVIEW_GIT_TOKEN\";;\nesac\n")
            .map_err(|e| AppError::Sandbox(format!("Failed to create trusted credential helper: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&askpass, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| AppError::Sandbox(format!("Credential helper permissions: {e}")))?;
        }
        let mut command = self.trusted_git(&[
            "-c",
            "credential.helper=",
            "push",
            "--no-verify",
            "origin",
            &format!("HEAD:refs/heads/{branch}"),
        ]);
        command
            .env("GIT_ASKPASS", &askpass)
            .env("OPENREVIEW_GIT_TOKEN", &self.token);
        let output = timeout(COMMAND_TIMEOUT, command.output())
            .await
            .map_err(|_| AppError::Sandbox("Git push timed out".into()))?
            .map_err(|e| AppError::Sandbox(format!("Git push failed to start: {e}")))?;
        if !output.status.success() {
            return Err(AppError::Sandbox(format!(
                "Git push rejected: {}",
                String::from_utf8_lossy(&output.stderr)
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
    pub workspace_root: PathBuf,
}

impl Default for DockerSandboxDriver {
    fn default() -> Self {
        Self {
            base_image: "openreview-sandbox:latest".to_string(),
            workspace_root: std::env::var("SANDBOX_WORKSPACE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/srv/openreview/workspaces")),
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
        mode: WorkflowMode,
    ) -> Result<Box<dyn SandboxInstance>> {
        let temp_dir = tempfile::Builder::new()
            .prefix("openreview-job-")
            .tempdir_in(&self.workspace_root)
            .map_err(|e| {
                AppError::Sandbox(format!(
                    "Shared workspace allocation failed in {}: {e}",
                    self.workspace_root.display()
                ))
            })?;
        let work_path = temp_dir.path().to_path_buf();
        let trusted_git_dir = tempfile::tempdir().map_err(|e| {
            AppError::Sandbox(format!("Trusted git temp dir allocation failed: {e}"))
        })?;
        let container_name = format!("openreview-sb-{}", &Uuid::new_v4().to_string()[..8]);
        let mut cleanup_guard = ContainerCreationGuard {
            name: container_name.clone(),
            armed: true,
        };

        info!(
            "Starting Docker container {container_name} with mount {}",
            work_path.display()
        );

        // The clone runs on the trusted orchestration host. Its credentials must
        // never appear in process arguments, a repository remote, or the mount.
        let clone_url = format!("https://github.com/{repo_full_name}.git");
        let askpass = trusted_git_dir.path().join("clone-askpass.sh");
        std::fs::write(
            &askpass,
            "#!/bin/sh\ncase \"$1\" in\n*Username*) printf '%s\\n' x-access-token;;\n*Password*) printf '%s\\n' \"$OPENREVIEW_GIT_TOKEN\";;\nesac\n",
        )
        .map_err(|e| AppError::Sandbox(format!("Failed to create clone credential helper: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&askpass, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| AppError::Sandbox(format!("Clone credential permissions: {e}")))?;
        }

        let clone_status = timeout(
            Duration::from_secs(120),
            Command::new("git")
                .args([
                    "-c",
                    "core.hooksPath=/dev/null",
                    "clone",
                    "--depth=1",
                    "--branch",
                    branch,
                    &clone_url,
                    ".",
                ])
                .env_clear()
                .env("PATH", "/usr/local/bin:/usr/bin:/bin")
                .env("HOME", "/nonexistent")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_TERMINAL_PROMPT", "0")
                .env("GIT_ASKPASS", &askpass)
                .env("OPENREVIEW_GIT_TOKEN", token)
                .current_dir(&work_path)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| AppError::Sandbox("Git clone timed out".into()))?
        .map_err(|e| AppError::Sandbox(format!("git clone execution failed: {e}")))?;

        if !clone_status.status.success() {
            return Err(AppError::Sandbox(
                "git clone failed for docker sandbox".to_string(),
            ));
        }

        // CRITICAL SECURITY FIX: Sanitize origin remote URL immediately so that the token
        // is NEVER stored on disk where container execution can read it
        let clean_origin = format!("https://github.com/{repo_full_name}.git");
        let sanitized = Command::new("git")
            .args(["remote", "set-url", "origin", &clean_origin])
            .current_dir(&work_path)
            .output()
            .await
            .map_err(|e| AppError::Sandbox(format!("Git remote sanitization failed: {e}")))?;
        if !sanitized.status.success() {
            return Err(AppError::Sandbox("Git remote sanitization rejected".into()));
        }

        // The untrusted workload must never see .git/config, hooks or the trusted
        // publishing state. Keep them on the host outside the container bind mount.
        std::fs::rename(work_path.join(".git"), trusted_git_dir.path().join(".git")).map_err(
            |e| AppError::Sandbox(format!("Failed to isolate trusted git metadata: {e}")),
        )?;

        // Start background container with mounted volume and security restrictions:
        // - Memory limit: 2GB
        // - CPU limit: 2 cores
        // - PIDs limit: 100
        // - Drop all Linux capabilities
        // - No new privileges
        let run_status = timeout(
            Duration::from_secs(60),
            Command::new("docker")
                .args([
                    "run",
                    "-d",
                    "--rm",
                    "--name",
                    &container_name,
                    "--memory=2g",
                    "--cpus=2.0",
                    "--pids-limit=100",
                    "--cap-drop=ALL",
                    "--security-opt=no-new-privileges:true",
                    "--network=none",
                    "--read-only",
                    "--tmpfs=/tmp:rw,nosuid,size=1g",
                    // Runs against a dedicated rootless daemon: container UID 0 maps
                    // to the daemon's unprivileged host UID, not to host root.
                    "--user=0:0",
                    "--mount",
                    &sandbox_mount(&work_path, mode),
                    "-w",
                    "/workspace",
                    &self.base_image,
                    "sleep",
                    "1800",
                ])
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| AppError::Sandbox("Docker run timed out".into()))?
        .map_err(|e| AppError::Sandbox(format!("docker run failed: {e}")))?;

        if !run_status.status.success() {
            return Err(AppError::Sandbox(
                "Failed to launch docker sandbox container".to_string(),
            ));
        }

        // The launch guard removes the container even when this check fails.
        // No untrusted tools run until all three controllers are verified.
        verify_container_limits(&container_name).await?;

        cleanup_guard.armed = false;

        Ok(Box::new(DockerSandbox {
            container_name,
            work_path,
            token: token.to_string(),
            _temp_dir: temp_dir,
            trusted_git_dir,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_mount_is_read_only_but_authorized_fix_mount_is_writable() {
        let workspace = Path::new("/tmp/example-workspace");
        assert!(sandbox_mount(workspace, WorkflowMode::Review).ends_with(",readonly"));
        assert!(!sandbox_mount(workspace, WorkflowMode::Fix).contains("readonly"));
    }

    #[test]
    fn container_limits_must_be_enforced_not_silently_ignored() {
        assert!(resource_limits_enforced(
            "2147483648\n",
            "100\n",
            "200000 100000\n"
        ));
        assert!(!resource_limits_enforced("max", "100", "200000 100000"));
        assert!(!resource_limits_enforced(
            "2147483648",
            "max",
            "200000 100000"
        ));
        assert!(!resource_limits_enforced("2147483648", "100", "max 100000"));
        assert!(!resource_limits_enforced(
            "2147483648",
            "100",
            "300000 100000"
        ));
    }

    #[tokio::test]
    async fn trusted_git_metadata_is_kept_outside_untrusted_mount() {
        let work = tempfile::tempdir().unwrap();
        let trusted = tempfile::tempdir().unwrap();
        let exec = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(work.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        exec(&["init", "-q"]);
        exec(&["config", "user.name", "OpenReview Test"]);
        exec(&["config", "user.email", "openreview@example.org"]);
        std::fs::write(work.path().join("source.txt"), "initial").unwrap();
        exec(&["add", "source.txt"]);
        exec(&["commit", "-q", "-m", "initial"]);
        std::fs::rename(work.path().join(".git"), trusted.path().join(".git")).unwrap();

        let sandbox = DockerSandbox {
            container_name: "nonexistent-openreview-test".into(),
            work_path: work.path().to_path_buf(),
            token: String::new(),
            _temp_dir: work,
            trusted_git_dir: trusted,
        };
        assert!(!sandbox.work_dir().join(".git").exists());
        assert!(!sandbox.has_uncommitted_changes().await.unwrap());
        std::fs::write(sandbox.work_dir().join("source.txt"), "edited").unwrap();
        assert!(sandbox.has_uncommitted_changes().await.unwrap());
    }
}
