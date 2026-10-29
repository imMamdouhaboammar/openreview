use crate::error::Result;
use crate::workflow::command::WorkflowMode;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait SandboxInstance: Send + Sync {
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<CommandOutput>;
    async fn run_bash(&self, script: &str) -> Result<CommandOutput>;
    /// Verification has a longer bounded budget than untrusted agent tools.
    /// Drivers without a separate limit can reuse their existing command path.
    async fn run_verification(&self, script: &str) -> Result<CommandOutput> {
        self.run_bash(script).await
    }
    async fn read_file(&self, path: &str) -> Result<String>;
    async fn write_file(&self, path: &str, content: &str) -> Result<()>;
    async fn has_uncommitted_changes(&self) -> Result<bool>;
    async fn commit_and_push(&self, message: &str, branch: &str) -> Result<()>;
    fn work_dir(&self) -> &Path;
}

#[async_trait]
pub trait SandboxDriver: Send + Sync {
    async fn create_sandbox(
        &self,
        repo_full_name: &str,
        branch: &str,
        token: &str,
        mode: WorkflowMode,
    ) -> Result<Box<dyn SandboxInstance>>;
}
