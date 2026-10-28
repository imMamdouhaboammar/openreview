use crate::error::{AppError, Result};
use crate::github::types::{BranchProtection, PullRequest, PushAccessResult, Repository};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::json;
use tracing::{debug, warn};

#[derive(Clone, Debug)]
pub struct GitHubClient {
    client: reqwest::Client,
    token: String,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("openreview-rs/0.1.0"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| AppError::Auth(e.to_string()))?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(Self {
            client,
            token,
            base_url: "https://api.github.com".to_string(),
        })
    }

    pub fn with_base_url(token: String, base_url: String) -> Result<Self> {
        let mut client = Self::new(token)?;
        client.base_url = base_url;
        Ok(client)
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!("{}/repos/{owner}/{repo}", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            AppError::GitHubApi(format!("Failed to fetch repository {owner}/{repo}: {e}"))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "Repository query failed ({status}): {body}"
            )));
        }

        resp.json::<Repository>()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to parse repository payload: {e}")))
    }

    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest> {
        let url = format!("{}/repos/{owner}/{repo}/pulls/{pr_number}", self.base_url);
        let resp =
            self.client.get(&url).send().await.map_err(|e| {
                AppError::GitHubApi(format!("Failed to fetch PR #{pr_number}: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "PR fetch failed ({status}): {body}"
            )));
        }

        resp.json::<PullRequest>()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to parse PR payload: {e}")))
    }

    pub async fn check_push_access(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        app_slug: &str,
    ) -> Result<PushAccessResult> {
        let repo_data = self.get_repository(owner, repo).await?;
        if repo_data.archived {
            return Ok(PushAccessResult {
                can_push: false,
                reason: Some("Repository is archived and cannot be modified".to_string()),
            });
        }

        // Check branch protection rules
        let protection_url = format!(
            "{}/repos/{owner}/{repo}/branches/{branch}/protection",
            self.base_url
        );
        let protection_resp = self.client.get(&protection_url).send().await;

        if let Ok(resp) = protection_resp {
            let status = resp.status();
            if status.is_success() {
                if let Ok(protection) = resp.json::<BranchProtection>().await
                    && let Some(restrictions) =
                        protection.restrictions.filter(|r| !r.apps.is_empty())
                {
                    let allowed = restrictions.apps.iter().any(|app| {
                        app.slug
                            .as_deref()
                            .map(|s| s.eq_ignore_ascii_case(app_slug))
                            .unwrap_or(false)
                    });
                    if !allowed {
                        return Ok(PushAccessResult {
                            can_push: false,
                            reason: Some(format!(
                                "Branch \"{branch}\" has push restrictions that do not include the {app_slug} app"
                            )),
                        });
                    }
                }
            } else if status.as_u16() != 404 && status.as_u16() != 403 {
                debug!("Branch protection query returned status {status}");
            }
        }

        Ok(PushAccessResult {
            can_push: true,
            reason: None,
        })
    }

    pub async fn add_reaction_to_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
        content: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/comments/{comment_id}/reactions",
            self.base_url
        );
        let resp = self
            .client
            .post(&url)
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to post reaction: {e}")))?;

        if !resp.status().is_success()
            && resp.status().as_u16() != 200
            && resp.status().as_u16() != 201
        {
            warn!("Add reaction returned status {}", resp.status());
        }
        Ok(())
    }

    pub async fn add_reaction_to_review_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
        content: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/comments/{comment_id}/reactions",
            self.base_url
        );
        let resp = self
            .client
            .post(&url)
            .json(&json!({ "content": content }))
            .send()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to post reaction: {e}")))?;

        if !resp.status().is_success()
            && resp.status().as_u16() != 200
            && resp.status().as_u16() != 201
        {
            warn!("Add reaction returned status {}", resp.status());
        }
        Ok(())
    }

    pub async fn post_comment(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        body: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/{pr_number}/comments",
            self.base_url
        );
        let resp = self
            .client
            .post(&url)
            .json(&json!({ "body": body }))
            .send()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to post PR comment: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "Post comment failed ({status}): {text}"
            )));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn post_inline_review_comment(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        commit_id: &str,
        path: &str,
        line: u64,
        body: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/{pr_number}/comments",
            self.base_url
        );
        let resp = self
            .client
            .post(&url)
            .json(&json!({
                "body": body,
                "commit_id": commit_id,
                "path": path,
                "line": line,
                "side": "RIGHT"
            }))
            .send()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to post review comment: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "Inline review comment failed ({status}): {text}"
            )));
        }
        Ok(())
    }

    pub async fn get_collaborator_permission(
        &self,
        owner: &str,
        repo: &str,
        username: &str,
    ) -> Result<String> {
        let url = format!(
            "{}/repos/{owner}/{repo}/collaborators/{username}/permission",
            self.base_url
        );
        let resp = self.client.get(&url).send().await.map_err(|e| {
            AppError::GitHubApi(format!("Failed to query collaborator permission: {e}"))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "Collaborator permission query failed ({status}): {body}"
            )));
        }

        let payload: crate::github::types::CollaboratorPermissionResponse =
            resp.json().await.map_err(|e| {
                AppError::GitHubApi(format!(
                    "Failed to parse collaborator permission payload: {e}"
                ))
            })?;

        Ok(payload.permission)
    }

    pub async fn get_pull_request_diff(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<String> {
        let url = format!("{}/repos/{owner}/{repo}/pulls/{pr_number}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header(ACCEPT, "application/vnd.github.v3.diff")
            .send()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to fetch PR diff: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "PR diff fetch failed ({status}): {text}"
            )));
        }

        resp.text()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to read PR diff body: {e}")))
    }

    pub async fn get_pull_request_files(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<Vec<crate::github::types::ChangedFile>> {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls/{pr_number}/files",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to fetch PR files: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "PR files fetch failed ({status}): {text}"
            )));
        }

        resp.json::<Vec<crate::github::types::ChangedFile>>()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to parse PR files: {e}")))
    }

    pub async fn get_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        comment_id: u64,
    ) -> Result<crate::github::types::IssueCommentDetail> {
        let url = format!(
            "{}/repos/{owner}/{repo}/issues/comments/{comment_id}",
            self.base_url
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to fetch issue comment: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::GitHubApi(format!(
                "Issue comment query failed ({status}): {text}"
            )));
        }

        resp.json::<crate::github::types::IssueCommentDetail>()
            .await
            .map_err(|e| AppError::GitHubApi(format!("Failed to parse issue comment: {e}")))
    }
}

pub fn is_permission_authorized_for_fix(permission: &str) -> bool {
    matches!(
        permission.to_lowercase().as_str(),
        "admin" | "write" | "maintain"
    )
}

pub fn extract_number_from_url(url: &str) -> Option<u64> {
    url.rsplit('/').next()?.parse::<u64>().ok()
}
