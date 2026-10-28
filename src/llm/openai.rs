use crate::error::{AppError, Result};
use crate::llm::traits::LlmProvider;
use crate::llm::types::{ChatMessage, LlmResponse, MessageRole, ToolCall, ToolDefinition};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::debug;

pub struct OpenAiProvider {
    client: reqwest::Client,
    _api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if !api_key.is_empty() {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {api_key}"))
                    .map_err(|e| AppError::Config(e.to_string()))?,
            );
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let url = base_url.unwrap_or_else(|| "https://api.openai.com".to_string());

        Ok(Self {
            client,
            _api_key: api_key,
            model,
            base_url: url.trim_end_matches('/').to_string(),
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(
        &self,
        system: &str,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut oai_messages: Vec<Value> = Vec::new();
        if !system.is_empty() {
            oai_messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        for msg in messages {
            match msg.role {
                MessageRole::User => {
                    oai_messages.push(json!({
                        "role": "user",
                        "content": msg.content
                    }));
                }
                MessageRole::Assistant => {
                    let mut obj = json!({
                        "role": "assistant",
                        "content": if msg.content.is_empty() { Value::Null } else { json!(msg.content) }
                    });

                    if !msg.tool_calls.is_empty() {
                        let tcs: Vec<Value> = msg
                            .tool_calls
                            .iter()
                            .map(|tc| {
                                json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.name,
                                        "arguments": tc.arguments.to_string()
                                    }
                                })
                            })
                            .collect();
                        obj["tool_calls"] = json!(tcs);
                    }
                    oai_messages.push(obj);
                }
                MessageRole::Tool => {
                    oai_messages.push(json!({
                        "role": "tool",
                        "tool_call_id": msg.tool_call_id.clone().unwrap_or_default(),
                        "content": msg.content
                    }));
                }
                MessageRole::System => {
                    oai_messages.push(json!({
                        "role": "system",
                        "content": msg.content
                    }));
                }
            }
        }

        let mut body = json!({
            "model": self.model,
            "messages": oai_messages,
        });

        if !tools.is_empty() {
            let oai_tools: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        }
                    })
                })
                .collect();
            body["tools"] = json!(oai_tools);
        }

        debug!(
            "Dispatching OpenAI-compatible completion request to {}",
            url
        );

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("OpenAI network error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!(
                "OpenAI API error ({status}): {text}"
            )));
        }

        let json_resp: Value = resp
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("OpenAI JSON parse error: {e}")))?;

        let choice = json_resp["choices"]
            .get(0)
            .ok_or_else(|| AppError::Llm("No choices in OpenAI response".to_string()))?;

        let message = &choice["message"];
        let content = message["content"].as_str().map(|s| s.to_string());

        let mut tool_calls = Vec::new();
        if let Some(tcs) = message["tool_calls"].as_array() {
            for tc in tcs {
                let id = tc["id"].as_str().unwrap_or_default().to_string();
                let name = tc["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                let arguments: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments,
                });
            }
        }

        let input_tokens = json_resp["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = json_resp["usage"]["completion_tokens"]
            .as_u64()
            .unwrap_or(0) as usize;

        Ok(LlmResponse {
            content,
            tool_calls,
            input_tokens,
            output_tokens,
        })
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
