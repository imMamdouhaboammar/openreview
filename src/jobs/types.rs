use crate::workflow::command::WorkflowMode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Received,
    Authorized,
    Queued,
    Running,
    Verifying,
    Publishing,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Received => write!(f, "received"),
            Self::Authorized => write!(f, "authorized"),
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::Verifying => write!(f, "verifying"),
            Self::Publishing => write!(f, "publishing"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub delivery_id: String,
    pub event_type: String,
    pub repo_full_name: String,
    pub pr_number: u64,
    pub installation_id: Option<u64>,
    pub actor: String,
    pub mode: WorkflowMode,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
}
