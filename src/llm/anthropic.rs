use crate::error::{AppError, Result};
use crate::llm::traits::LlmProvider;
use crate::llm::types::{ChatMessage, LlmResponse, MessageRole, ToolCall, ToolDefinition};
use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::debug;

pub struct AnthropicProvider {
    client: reqwest::Client,
    _api_key: String,
    model: String,
    base_url: String,
    messages_endpoint: Option<String>,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&api_key).map_err(|e| AppError::Config(e.to_string()))?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(Self {
            client,
            _api_key: api_key,
            model,
            base_url: "https://api.anthropic.com".to_string(),
            messages_endpoint: None,
        })
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    pub fn with_opencode_custom(mut self, base_url: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        // OpenCode's gateway explicitly accepts x-api-key for provider SDKs.
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self._api_key)
                .map_err(|error| AppError::Config(error.to_string()))?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        self.client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.messages_endpoint = Some(format!("{base_url}/messages"));
        Ok(self)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(
        &self,
        system: &str,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        let url = self
            .messages_endpoint
            .clone()
            .unwrap_or_else(|| format!("{}/v1/messages", self.base_url));

        let anthropic_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            })
            .collect();

        let mut anthropic_messages: Vec<Value> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::User => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": msg.content
                    }));
                }
                MessageRole::Assistant => {
                    let mut content_parts: Vec<Value> = Vec::new();
                    if !msg.content.is_empty() {
                        content_parts.push(json!({
                            "type": "text",
                            "text": msg.content
                        }));
                    }
                    for tc in &msg.tool_calls {
                        content_parts.push(json!({
                            "type": "tool_use",
                            "id": tc.id,
                            "name": tc.name,
                            "input": tc.arguments
                        }));
                    }
                    anthropic_messages.push(json!({
                        "role": "assistant",
                        "content": content_parts
                    }));
                }
                MessageRole::Tool => {
                    if let Some(tool_use_id) = &msg.tool_call_id {
                        anthropic_messages.push(json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": tool_use_id,
                                "content": msg.content
                            }]
                        }));
                    }
                }
                MessageRole::System => {}
            }
        }

        let mut body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": anthropic_messages,
        });

        if !anthropic_tools.is_empty() {
            body["tools"] = json!(anthropic_tools);
        }

        debug!(
            "Dispatching Anthropic completion request with model {}",
            self.model
        );

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("Anthropic network error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!(
                "Anthropic API error ({status}): {text}"
            )));
        }

        let json_resp: Value = resp
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("Anthropic JSON parse error: {e}")))?;

        let mut text_output = String::new();
        let mut tool_calls = Vec::new();

        if let Some(content_array) = json_resp["content"].as_array() {
            for block in content_array {
                match block["type"].as_str() {
                    Some("text") => {
                        if let Some(t) = block["text"].as_str() {
                            text_output.push_str(t);
                        }
                    }
                    Some("tool_use") => {
                        let id = block["id"].as_str().unwrap_or_default().to_string();
                        let name = block["name"].as_str().unwrap_or_default().to_string();
                        let arguments = block["input"].clone();
                        tool_calls.push(ToolCall {
                            id,
                            name,
                            arguments,
                        });
                    }
                    _ => {}
                }
            }
        }

        let input_tokens = json_resp["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = json_resp["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize;

        Ok(LlmResponse {
            content: if text_output.is_empty() {
                None
            } else {
                Some(text_output)
            },
            tool_calls,
            input_tokens,
            output_tokens,
        })
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
