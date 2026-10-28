use crate::llm::types::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ThreadMessage {
    pub content: String,
    pub role: String, // "assistant" | "user"
}

impl From<ThreadMessage> for ChatMessage {
    fn from(msg: ThreadMessage) -> Self {
        if msg.role == "assistant" {
            ChatMessage::assistant(msg.content)
        } else {
            ChatMessage::user(msg.content)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkflowParams {
    pub repo_full_name: String,
    pub pr_number: u64,
    pub base_branch: String,
    pub pr_branch: String,
    pub comment_id: Option<u64>,
    pub messages: Vec<ThreadMessage>,
}
