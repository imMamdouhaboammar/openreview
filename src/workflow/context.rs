use crate::error::Result;
use crate::github::GitHubClient;
use crate::github::types::ChangedFile;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PullRequestContext {
    pub repo_full_name: String,
    pub pr_number: u64,
    pub title: String,
    pub description: String,
    pub base_branch: String,
    pub base_sha: String,
    pub head_branch: String,
    pub head_sha: String,
    pub is_fork: bool,
    pub diff: String,
    pub changed_files: Vec<ChangedFile>,
}

impl PullRequestContext {
    pub async fn fetch(
        gh_client: &GitHubClient,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Self> {
        let pr = gh_client.get_pull_request(owner, repo, pr_number).await?;
        let diff = gh_client
            .get_pull_request_diff(owner, repo, pr_number)
            .await?;

        let changed_files = gh_client
            .get_pull_request_files(owner, repo, pr_number)
            .await?;
        if pr
            .changed_files
            .is_some_and(|n| n != changed_files.len() as u64)
        {
            return Err(crate::error::AppError::Workflow(
                "PR file count differs from GitHub metadata; refusing partial diff context".into(),
            ));
        }
        let latest_pr = gh_client.get_pull_request(owner, repo, pr_number).await?;
        if latest_pr.head.sha != pr.head.sha {
            return Err(crate::error::AppError::Workflow(
                "PR head moved during context retrieval; retry against a stable revision".into(),
            ));
        }

        let base_repo_name = pr
            .base
            .repo
            .as_ref()
            .map(|r| r.full_name.clone())
            .unwrap_or_else(|| format!("{owner}/{repo}"));
        let head_repo_name = pr
            .head
            .repo
            .as_ref()
            .map(|r| r.full_name.clone())
            .unwrap_or_default();
        let is_fork = head_repo_name.is_empty() || head_repo_name != base_repo_name;

        Ok(Self {
            repo_full_name: format!("{owner}/{repo}"),
            pr_number,
            title: pr.title,
            description: pr.body.unwrap_or_default(),
            base_branch: pr.base.r#ref,
            base_sha: pr.base.sha,
            head_branch: pr.head.r#ref,
            head_sha: pr.head.sha,
            is_fork,
            diff,
            changed_files,
        })
    }

    pub fn to_prompt_context(&self) -> String {
        let mut section = String::new();
        section.push_str(&format!(
            "## Pull Request Context: #{}: {}\n",
            self.pr_number, self.title
        ));
        section.push_str(&format!(
            "- Base branch: `{}` ({})\n",
            self.base_branch, self.base_sha
        ));
        section.push_str(&format!(
            "- Head branch: `{}` ({})\n",
            self.head_branch, self.head_sha
        ));
        if self.is_fork {
            section.push_str("- ⚠️ This PR originates from an external fork.\n");
        }
        if !self.description.is_empty() {
            section.push_str(&format!(
                "\n### Description:\n{}\n",
                self.description.trim()
            ));
        }

        section.push_str("\n### Changed Files:\n");
        for f in &self.changed_files {
            section.push_str(&format!(
                "- `{}` ({}, +{}, -{})\n",
                f.filename, f.status, f.additions, f.deletions
            ));
        }

        if !self.diff.is_empty() {
            // Include diff capped at a reasonable size to avoid blowing context
            let diff_snippet = if self.diff.len() > 30_000 {
                let mut end = 30_000;
                while !self.diff.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...\n(Diff truncated to 30,000 bytes)", &self.diff[..end])
            } else {
                self.diff.clone()
            };
            section.push_str(&format!(
                "\n### Authoritative PR Diff:\n```diff\n{}\n```\n",
                diff_snippet.trim()
            ));
        }

        section
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_context_multibyte_diff_boundary() {
        let context = PullRequestContext {
            repo_full_name: "owner/repo".into(),
            pr_number: 1,
            title: "test".into(),
            description: String::new(),
            base_branch: "main".into(),
            base_sha: "base".into(),
            head_branch: "fix".into(),
            head_sha: "head".into(),
            is_fork: false,
            diff: format!("{}🚀", "x".repeat(29_999)),
            changed_files: vec![],
        };
        let result = context.to_prompt_context();
        assert!(result.contains("Diff truncated"));
        assert!(!result.contains('🚀'));
    }
}
