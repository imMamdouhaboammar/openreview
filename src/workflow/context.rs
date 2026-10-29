use crate::error::{AppError, Result};
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
        // GitHub's aggregate diff endpoint can reject very large PRs while
        // the paginated file patches are still available. Never silently
        // proceed with an incomplete changed-file set or missing patch.
        let diff = match gh_client.get_pull_request_diff(owner, repo, pr_number).await {
            Ok(diff) => diff,
            Err(original_error) => reconstruct_diff_from_patches(&changed_files).map_err(|_| {
                AppError::Workflow(format!(
                    "GitHub aggregate PR diff failed ({original_error}) and changed-file patches were incomplete"
                ))
            })?,
        };
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

fn reconstruct_diff_from_patches(files: &[ChangedFile]) -> Result<String> {
    let mut diff = String::new();
    for file in files {
        let patch = file
            .patch
            .as_deref()
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| {
                AppError::Workflow(format!("Missing patch for changed file {}", file.filename))
            })?;
        let previous = match file.status.as_str() {
            "renamed" | "copied" => file.previous_filename.as_deref().ok_or_else(|| {
                AppError::Workflow(format!("Missing original filename for {}", file.filename))
            })?,
            _ => &file.filename,
        };
        let old_label = if file.status == "added" {
            "/dev/null".to_string()
        } else {
            format!("a/{previous}")
        };
        let new_label = if file.status == "removed" {
            "/dev/null".to_string()
        } else {
            format!("b/{}", file.filename)
        };
        diff.push_str(&format!(
            "diff --git a/{previous} b/{}\n--- {old_label}\n+++ {new_label}\n{patch}\n",
            file.filename
        ));
    }
    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstructs_a_complete_renamed_and_deleted_file_diff() {
        let files = vec![
            ChangedFile {
                filename: "new.rs".into(),
                previous_filename: Some("old.rs".into()),
                status: "renamed".into(),
                additions: 1,
                deletions: 1,
                changes: 2,
                patch: Some("@@ -1 +1 @@\n-old\n+new".into()),
            },
            ChangedFile {
                filename: "gone.rs".into(),
                previous_filename: None,
                status: "removed".into(),
                additions: 0,
                deletions: 1,
                changes: 1,
                patch: Some("@@ -1 +0,0 @@\n-gone".into()),
            },
        ];
        let diff = reconstruct_diff_from_patches(&files).unwrap();
        assert!(diff.contains("diff --git a/old.rs b/new.rs"));
        assert!(diff.contains("+++ b/new.rs"));
        assert!(diff.contains("--- a/gone.rs\n+++ /dev/null"));
    }

    #[test]
    fn incomplete_file_patches_must_not_be_misreported_as_full_diff() {
        let files = vec![ChangedFile {
            filename: "missing.rs".into(),
            previous_filename: None,
            status: "modified".into(),
            additions: 3,
            deletions: 1,
            changes: 4,
            patch: None,
        }];
        assert!(reconstruct_diff_from_patches(&files).is_err());
    }

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
