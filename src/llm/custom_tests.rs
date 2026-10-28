use super::*;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn mock_once<F>(check: F, response: &'static str) -> (String, tokio::task::JoinHandle<()>)
where
    F: FnOnce(&str, &Value) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut data = Vec::new();
        loop {
            let mut chunk = [0u8; 8192];
            let len = stream.read(&mut chunk).await.unwrap();
            assert!(len > 0, "HTTP request ended before its body");
            data.extend_from_slice(&chunk[..len]);
            if let Some(index) = data.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&data[..index]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                if data.len() >= index + 4 + length {
                    break;
                }
            }
        }
        let request = String::from_utf8(data).unwrap();
        let (headers, body) = request.split_once("\r\n\r\n").unwrap();
        let body: Value = serde_json::from_str(body).unwrap();
        check(headers, &body);
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len()
        );
        stream.write_all(reply.as_bytes()).await.unwrap();
    });
    (format!("http://{address}"), server)
}

fn scoped_header(headers: &str, name: &str, value: &str) {
    assert!(
        headers
            .to_ascii_lowercase()
            .contains(&format!("{name}: {value}").to_ascii_lowercase()),
        "expected a scoped {name} header"
    );
    assert!(!headers.contains("private-secret-in-path"));
}

#[tokio::test]
async fn custom_gemini_uses_native_path_bearer_and_valid_tool_result_role() {
    let (base, server) = mock_once(
        |headers, body| {
            assert!(headers.starts_with("POST /models/gemini-3.8-flash:generateContent HTTP/1.1"));
            scoped_header(headers, "authorization", "Bearer private-gemini-key");
            assert_eq!(body["contents"][0]["role"], "user");
            assert_eq!(body["contents"][1]["role"], "model");
            assert_eq!(body["contents"][2]["role"], "user");
            assert_eq!(
                body["contents"][2]["parts"][0]["functionResponse"]["name"],
                "readFile"
            );
        },
        r#"{"candidates":[{"content":{"parts":[{"text":"OK"}]}}],"usageMetadata":{"promptTokenCount":4,"candidatesTokenCount":1}}"#,
    )
    .await;
    let provider = GeminiProvider::new("private-gemini-key".into(), "gemini-3.8-flash".into())
        .unwrap()
        .with_opencode_custom(base)
        .unwrap();
    let messages = vec![
        ChatMessage::user("inspect file"),
        ChatMessage::assistant_with_tools(
            None,
            vec![ToolCall {
                id: "call_readFile_0".into(),
                name: "readFile".into(),
                arguments: json!({"path":"a.rs"}),
            }],
        ),
        ChatMessage::tool_result("call_readFile_0", "ok"),
    ];
    let result = provider.complete("", &messages, &[]).await.unwrap();
    assert_eq!(result.content.as_deref(), Some("OK"));
    server.await.unwrap();
}

#[tokio::test]
async fn custom_chat_uses_the_unversioned_connection_path() {
    let (base, server) = mock_once(
        |headers, body| {
            assert!(headers.starts_with("POST /chat/completions HTTP/1.1"));
            scoped_header(headers, "authorization", "Bearer private-chat-key");
            assert_eq!(body["model"], "custom-chat");
            assert_eq!(body["messages"][0]["content"], "test");
        },
        r#"{"choices":[{"message":{"content":"chat-works"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#,
    )
    .await;
    let provider = OpenAiProvider::new(
        "private-chat-key".into(),
        "custom-chat".into(),
        Some(base.clone()),
    )
    .unwrap()
    .with_chat_endpoint(format!("{base}/chat/completions"))
    .unwrap();
    let result = provider
        .complete("", &[ChatMessage::user("test")], &[])
        .await
        .unwrap();
    assert_eq!(result.content.as_deref(), Some("chat-works"));
    server.await.unwrap();
}

#[tokio::test]
async fn custom_anthropic_uses_unversioned_messages_with_scoped_key() {
    let (base, server) = mock_once(
        |headers, body| {
            assert!(headers.starts_with("POST /messages HTTP/1.1"));
            scoped_header(headers, "x-api-key", "private-anthropic-key");
            assert_eq!(body["model"], "custom-anthropic");
            assert_eq!(body["messages"][0]["content"], "test");
        },
        r#"{"content":[{"type":"text","text":"anthropic-works"}],"usage":{"input_tokens":3,"output_tokens":2}}"#,
    )
    .await;
    let provider =
        AnthropicProvider::new("private-anthropic-key".into(), "custom-anthropic".into())
            .unwrap()
            .with_opencode_custom(base)
            .unwrap();
    let result = provider
        .complete("", &[ChatMessage::user("test")], &[])
        .await
        .unwrap();
    assert_eq!(result.content.as_deref(), Some("anthropic-works"));
    server.await.unwrap();
}

#[tokio::test]
async fn custom_responses_round_trips_function_calls() {
    let (base, server) = mock_once(
        |headers, body| {
            assert!(headers.starts_with("POST /responses HTTP/1.1"));
            scoped_header(headers, "authorization", "Bearer private-responses-key");
            assert!(headers.to_ascii_lowercase().contains("x-opencode-session: ses_"));
            assert_eq!(body["model"], "custom-responses");
            assert!(body["input"].as_array().unwrap().iter().any(|item| {
                item["type"] == "function_call_output" && item["call_id"] == "call_a"
            }));
            assert_eq!(body["tools"][0]["name"], "readFile");
        },
        r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"ready"}]},{"type":"function_call","call_id":"call_b","name":"readFile","arguments":"{\"path\":\"b.rs\"}"}],"usage":{"input_tokens":6,"output_tokens":3}}"#,
    )
    .await;
    let provider = OpenAiResponsesProvider::new(
        "private-responses-key".into(),
        "custom-responses".into(),
        format!("{base}/responses"),
    )
    .unwrap();
    let definition = ToolDefinition {
        name: "readFile".into(),
        description: "Read a file".into(),
        parameters: json!({"type":"object","properties":{"path":{"type":"string"}}}),
    };
    let result = provider
        .complete(
            "Help with code",
            &[
                ChatMessage::user("test"),
                ChatMessage::assistant_with_tools(
                    None,
                    vec![ToolCall {
                        id: "call_a".into(),
                        name: "readFile".into(),
                        arguments: json!({"path":"a.rs"}),
                    }],
                ),
                ChatMessage::tool_result("call_a", "done"),
            ],
            &[definition],
        )
        .await
        .unwrap();
    assert_eq!(result.content.as_deref(), Some("ready"));
    assert_eq!(result.tool_calls[0].id, "call_b");
    assert_eq!(result.tool_calls[0].arguments["path"], "b.rs");
    assert_eq!(result.input_tokens, 6);
    server.await.unwrap();
}

#[tokio::test]
async fn responses_reject_malformed_function_arguments() {
    let (base, server) = mock_once(
        |headers, _| assert!(headers.starts_with("POST /responses HTTP/1.1")),
        r#"{"output":[{"type":"function_call","call_id":"call_x","name":"writeFile","arguments":"not-json"}],"usage":{}}"#,
    )
    .await;
    let provider = OpenAiResponsesProvider::new(
        "scoped-test-key".into(),
        "model".into(),
        format!("{base}/responses"),
    )
    .unwrap();
    let error = provider
        .complete("", &[ChatMessage::user("test")], &[])
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Invalid Responses function arguments")
    );
    server.await.unwrap();
}

#[tokio::test]
async fn custom_responses_does_not_follow_redirects_with_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        let size = stream.read(&mut request).await.unwrap();
        assert!(String::from_utf8_lossy(&request[..size]).starts_with("POST /responses HTTP/1.1"));
        stream
            .write_all(b"HTTP/1.1 302 Found\r\nLocation: https://untrusted.example/collect\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
    });
    let provider = OpenAiResponsesProvider::new(
        "scoped-test-key".into(),
        "model".into(),
        format!("http://{address}/responses"),
    )
    .unwrap();
    let error = provider
        .complete("", &[ChatMessage::user("test")], &[])
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("302"),
        "redirect must be rejected before sending bearer credentials elsewhere: {error}"
    );
    server.await.unwrap();
}
