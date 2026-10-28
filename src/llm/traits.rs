use crate::error::Result;
use crate::llm::types::{ChatMessage, LlmResponse, ToolDefinition};
use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(
        &self,
        system: &str,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse>;

    fn model_name(&self) -> &str;
}
