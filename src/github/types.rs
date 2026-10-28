use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: User,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub private: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub id: u64,
    pub login: String,
    #[serde(default)]
    pub r#type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GitRef {
    pub r#ref: String,
    pub sha: String,
    pub repo: Option<Repository>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    #[serde(default)]
    pub changed_files: Option<u64>,
    pub title: String,
    pub body: Option<String>,
    pub head: GitRef,
    pub base: GitRef,
    pub html_url: String,
    pub user: User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Comment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub html_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Installation {
    pub id: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IssueCommentPayload {
    pub action: String,
    pub issue: IssuePayload,
    pub comment: Comment,
    pub repository: Repository,
    pub installation: Option<Installation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IssuePayload {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub pull_request: Option<PullRequestLink>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestLink {
    pub url: String,
    pub html_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestReviewCommentPayload {
    pub action: String,
    pub pull_request: PullRequest,
    pub comment: ReviewComment,
    pub repository: Repository,
    pub installation: Option<Installation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReviewComment {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub path: Option<String>,
    pub line: Option<u64>,
    pub commit_id: Option<String>,
    pub in_reply_to_id: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReactionPayload {
    pub action: String,
    pub reaction: ReactionDetails,
    pub comment: Option<Comment>,
    pub repository: Repository,
    pub installation: Option<Installation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReactionDetails {
    pub id: u64,
    pub content: String,
    pub user: User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallationTokenResponse {
    pub token: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BranchProtection {
    #[serde(default)]
    pub restrictions: Option<BranchRestrictions>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BranchRestrictions {
    #[serde(default)]
    pub apps: Vec<AppRestriction>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppRestriction {
    pub id: Option<u64>,
    pub slug: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PushAccessResult {
    pub can_push: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CollaboratorPermissionResponse {
    pub permission: String,
    #[serde(default)]
    pub role_name: Option<String>,
    pub user: User,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChangedFile {
    pub filename: String,
    pub status: String,
    #[serde(default)]
    pub additions: usize,
    #[serde(default)]
    pub deletions: usize,
    #[serde(default)]
    pub changes: usize,
    #[serde(default)]
    pub patch: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IssueCommentDetail {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub html_url: String,
    pub issue_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestReviewCommentDetail {
    pub id: u64,
    pub body: String,
    pub user: User,
    pub pull_request_url: String,
}
