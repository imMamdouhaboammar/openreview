use crate::error::{AppError, Result};
use crate::llm::traits::LlmProvider;
use crate::llm::types::{ChatMessage, LlmResponse, MessageRole, ToolCall, ToolDefinition};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};

/// OpenCode's custom Responses API. Keep a stable session for the agent run,
/// and never send its Console credential to a redirect destination.
pub struct OpenAiResponsesProvider {
    client: reqwest::Client,
    _api_key: String,
    model: String,
    endpoint: String,
    session_id: HeaderValue,
}

impl OpenAiResponsesProvider {
    pub fn new(api_key: String, model: String, endpoint: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|error| AppError::Config(error.to_string()))?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let session_id = HeaderValue::from_str(&format!("ses_{}", uuid::Uuid::new_v4().simple()))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(Self {
            client,
            _api_key: api_key,
            model,
            endpoint,
            session_id,
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiResponsesProvider {
    async fn complete(
        &self,
        system: &str,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        let mut input: Vec<Value> = Vec::new();
        for message in messages {
            match message.role {
                MessageRole::System => {
                    input.push(json!({"role": "system", "content": message.content}));
                }
                MessageRole::User => {
                    input.push(json!({"role": "user", "content": message.content}));
                }
                MessageRole::Assistant => {
                    if !message.content.is_empty() {
                        input.push(json!({"role": "assistant", "content": message.content}));
                    }
                    for call in &message.tool_calls {
                        input.push(json!({
                            "type": "function_call",
                            "call_id": call.id,
                            "name": call.name,
                            "arguments": call.arguments.to_string()
                        }));
                    }
                }
                MessageRole::Tool => {
                    let call_id = message.tool_call_id.as_deref().ok_or_else(|| {
                        AppError::Llm("Responses tool result has no call ID".into())
                    })?;
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": call_id,
                        "output": message.content
                    }));
                }
            }
        }
        let mut body = json!({"model": self.model, "input": input});
        if !system.is_empty() {
            body["instructions"] = json!(system);
        }
        if !tools.is_empty() {
            body["tools"] = json!(
                tools
                    .iter()
                    .map(|tool| json!({
                        "type": "function",
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }))
                    .collect::<Vec<_>>()
            );
        }

        let response = self
            .client
            .post(&self.endpoint)
            .header("x-opencode-session", self.session_id.clone())
            .json(&body)
            .send()
            .await
            .map_err(|error| AppError::Llm(format!("Responses network error: {error}")))?;
        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!(
                "Responses API error ({status}): {detail}"
            )));
        }
        let payload: Value = response
            .json()
            .await
            .map_err(|error| AppError::Llm(format!("Responses JSON error: {error}")))?;

        let output = payload["output"]
            .as_array()
            .ok_or_else(|| AppError::Llm("Responses API returned no output array".into()))?;
        let mut text = String::new();
        let mut tool_calls = Vec::new();
        for item in output {
            match item["type"].as_str() {
                Some("message") => {
                    if let Some(parts) = item["content"].as_array() {
                        for part in parts {
                            if let Some(value) = part["text"].as_str() {
                                text.push_str(value);
                            }
                        }
                    }
                }
                Some("function_call") => {
                    let id = item["call_id"]
                        .as_str()
                        .ok_or_else(|| AppError::Llm("Responses call has no call_id".into()))?;
                    let name = item["name"]
                        .as_str()
                        .ok_or_else(|| AppError::Llm("Responses call has no name".into()))?;
                    let arguments = item["arguments"]
                        .as_str()
                        .ok_or_else(|| AppError::Llm("Responses call has no arguments".into()))?;
                    let arguments: Value = serde_json::from_str(arguments).map_err(|error| {
                        AppError::Llm(format!("Invalid Responses function arguments: {error}"))
                    })?;
                    tool_calls.push(ToolCall {
                        id: id.into(),
                        name: name.into(),
                        arguments,
                    });
                }
                _ => {}
            }
        }
        Ok(LlmResponse {
            content: if text.is_empty() { None } else { Some(text) },
            tool_calls,
            input_tokens: payload["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize,
            output_tokens: payload["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize,
        })
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
