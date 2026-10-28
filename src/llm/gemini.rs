use crate::error::{AppError, Result};
use crate::llm::traits::LlmProvider;
use crate::llm::types::{ChatMessage, LlmResponse, MessageRole, ToolCall, ToolDefinition};
use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};
use tracing::debug;

pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(Self {
            client,
            api_key,
            model,
        })
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(
        &self,
        system: &str,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<LlmResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let mut contents: Vec<Value> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::User => {
                    contents.push(json!({
                        "role": "user",
                        "parts": [{ "text": msg.content }]
                    }));
                }
                MessageRole::Assistant => {
                    let mut parts = Vec::new();
                    if !msg.content.is_empty() {
                        parts.push(json!({ "text": msg.content }));
                    }
                    for tc in &msg.tool_calls {
                        parts.push(json!({
                            "functionCall": {
                                "name": tc.name,
                                "args": tc.arguments
                            }
                        }));
                    }
                    contents.push(json!({
                        "role": "model",
                        "parts": parts
                    }));
                }
                MessageRole::Tool => {
                    contents.push(json!({
                        "role": "function",
                        "parts": [{
                            "functionResponse": {
                                "name": msg.tool_call_id.clone().unwrap_or_else(|| "tool_result".to_string()),
                                "response": { "output": msg.content }
                            }
                        }]
                    }));
                }
                MessageRole::System => {}
            }
        }

        let mut body = json!({
            "contents": contents,
        });

        if !system.is_empty() {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system }]
            });
        }

        if !tools.is_empty() {
            let decls: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    })
                })
                .collect();
            body["tools"] = json!([{ "functionDeclarations": decls }]);
        }

        debug!("Dispatching Google Gemini completion request");

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Llm(format!("Gemini network error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Llm(format!(
                "Gemini API error ({status}): {text}"
            )));
        }

        let json_resp: Value = resp
            .json()
            .await
            .map_err(|e| AppError::Llm(format!("Gemini JSON parse error: {e}")))?;

        let candidate = json_resp["candidates"]
            .get(0)
            .ok_or_else(|| AppError::Llm("No candidate in Gemini response".to_string()))?;

        let mut text_output = String::new();
        let mut tool_calls = Vec::new();

        if let Some(parts) = candidate["content"]["parts"].as_array() {
            for (idx, part) in parts.iter().enumerate() {
                if let Some(t) = part["text"].as_str() {
                    text_output.push_str(t);
                }
                if let Some(fc) = part.get("functionCall") {
                    let name = fc["name"].as_str().unwrap_or_default().to_string();
                    let arguments = fc.get("args").cloned().unwrap_or(json!({}));
                    let id = format!("call_{}_{idx}", name);
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments,
                    });
                }
            }
        }

        let input_tokens = json_resp["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as usize;
        let output_tokens = json_resp["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as usize;

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
