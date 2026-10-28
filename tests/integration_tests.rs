use axum::body::Body;
use axum::http::{Request, StatusCode};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use sha2::digest::KeyInit;
use std::sync::Arc;
use tower::ServiceExt;

use openreview::agent::runner::AgentRunner;
use openreview::config::Config;
use openreview::llm::traits::LlmProvider;
use openreview::llm::types::{ChatMessage, LlmResponse, ToolCall, ToolDefinition};
use openreview::sandbox::traits::SandboxInstance;
use openreview::sandbox::{LocalSandbox, LocalSandboxDriver, SandboxDriver};
use openreview::server::{AppState, Metrics, create_router};
use openreview::skills::{SkillMetadata, discover_skills};
use openreview::workflow::WorkflowCoordinator;

type HmacSha256 = Hmac<Sha256>;

fn create_test_config() -> Config {
    Config {
        github_app_id: "12345".to_string(),
        github_app_private_key: "dummy-key".to_string(),
        github_app_webhook_secret: "test-webhook-secret-xyz".to_string(),
        github_app_installation_id: Some(999),
        bot_username: "openreview".to_string(),
        llm_provider: "anthropic".to_string(),
        model_name: "claude-3-7-sonnet-20250219".to_string(),
        anthropic_api_key: Some("dummy-key".to_string()),
        openai_api_key: None,
        openai_base_url: None,
        gemini_api_key: None,
        ollama_base_url: None,
        sandbox_driver: "local".to_string(),
        host: "127.0.0.1".to_string(),
        port: 3000,
    }
}

fn sign_payload(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

#[tokio::test]
async fn test_health_endpoint() {
    let config = create_test_config();
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new(vec![]));
    let sandbox_driver: Arc<dyn SandboxDriver> = Arc::new(LocalSandboxDriver);
    let coordinator = Arc::new(WorkflowCoordinator::new(
        config.clone(),
        llm,
        sandbox_driver,
    ));
    let state = AppState {
        config,
        coordinator,
        metrics: Arc::new(Metrics::new()),
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json_val["status"], "ok");
    assert_eq!(json_val["service"], "openreview-rs");
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let config = create_test_config();
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new(vec![]));
    let sandbox_driver: Arc<dyn SandboxDriver> = Arc::new(LocalSandboxDriver);
    let coordinator = Arc::new(WorkflowCoordinator::new(
        config.clone(),
        llm,
        sandbox_driver,
    ));
    let state = AppState {
        config,
        coordinator,
        metrics: Arc::new(Metrics::new()),
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("openreview_uptime_seconds"));
    assert!(text.contains("openreview_webhooks_total"));
}

#[tokio::test]
async fn test_webhook_signature_validation() {
    let config = create_test_config();
    let secret = config.github_app_webhook_secret.clone();
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new(vec![]));
    let sandbox_driver: Arc<dyn SandboxDriver> = Arc::new(LocalSandboxDriver);
    let coordinator = Arc::new(WorkflowCoordinator::new(
        config.clone(),
        llm,
        sandbox_driver,
    ));
    let state = AppState {
        config,
        coordinator,
        metrics: Arc::new(Metrics::new()),
    };

    let app = create_router(state);

    // 1. Missing signature header -> 400 Bad Request
    let req_missing = Request::builder()
        .method("POST")
        .uri("/api/webhooks/github")
        .header("content-type", "application/json")
        .body(Body::from(b"{}".as_slice()))
        .unwrap();
    let res_missing = app.clone().oneshot(req_missing).await.unwrap();
    assert_eq!(res_missing.status(), StatusCode::BAD_REQUEST);

    // 2. Invalid signature header -> 401 Unauthorized
    let req_invalid = Request::builder()
        .method("POST")
        .uri("/api/webhooks/github")
        .header("content-type", "application/json")
        .header(
            "x-hub-signature-256",
            "sha256=badbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbadbad",
        )
        .body(Body::from(b"{}".as_slice()))
        .unwrap();
    let res_invalid = app.clone().oneshot(req_invalid).await.unwrap();
    assert_eq!(res_invalid.status(), StatusCode::UNAUTHORIZED);

    // 3. Valid signature with mention -> 202 Accepted
    let payload = json!({
        "action": "created",
        "issue": {
            "id": 10,
            "number": 55,
            "title": "Add feature",
            "pull_request": {
                "url": "https://api.github.com/repos/test/repo/pulls/55",
                "html_url": "https://github.com/test/repo/pull/55"
            }
        },
        "comment": {
            "id": 200,
            "body": "@openreview please check the new diff",
            "user": { "id": 1, "login": "developer" },
            "html_url": "https://github.com/test/repo/pull/55#issuecomment-200"
        },
        "repository": {
            "id": 100,
            "name": "repo",
            "full_name": "test/repo",
            "owner": { "id": 1, "login": "test" }
        }
    });

    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let sig = sign_payload(&secret, &payload_bytes);

    let req_valid = Request::builder()
        .method("POST")
        .uri("/api/webhooks/github")
        .header("content-type", "application/json")
        .header("x-github-event", "issue_comment")
        .header("x-hub-signature-256", sig)
        .body(Body::from(payload_bytes))
        .unwrap();

    let res_valid = app.oneshot(req_valid).await.unwrap();
    assert_eq!(res_valid.status(), StatusCode::ACCEPTED);
}

// Mock LLM for integration testing
struct MockLlm {
    responses: std::sync::Mutex<Vec<LlmResponse>>,
}

impl MockLlm {
    fn new(responses: Vec<LlmResponse>) -> Self {
        Self {
            responses: std::sync::Mutex::new(responses),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockLlm {
    async fn complete(
        &self,
        _system: &str,
        _messages: &[ChatMessage],
        _tools: &[ToolDefinition],
    ) -> openreview::error::Result<LlmResponse> {
        let mut queue = self.responses.lock().unwrap();
        if !queue.is_empty() {
            Ok(queue.remove(0))
        } else {
            Ok(LlmResponse {
                content: Some("Review finished: Everything looks clean and correct!".to_string()),
                tool_calls: vec![],
                input_tokens: 150,
                output_tokens: 40,
            })
        }
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }
}

#[tokio::test]
async fn test_agent_react_loop_with_tools() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = LocalSandbox::new(temp);

    // Initial files in sandbox
    sandbox
        .write_file("src/main.rs", "fn main() { println!(\"bad\"); }")
        .await
        .unwrap();

    let skills = vec![SkillMetadata {
        name: "clean-code".to_string(),
        description: "Enforce clean code".to_string(),
        content: "Always format and test".to_string(),
    }];

    // Step 1: Tool call writeFile to fix the code
    let step1_resp = LlmResponse {
        content: None,
        tool_calls: vec![ToolCall {
            id: "call_write_1".to_string(),
            name: "writeFile".to_string(),
            arguments: json!({
                "path": "src/main.rs",
                "content": "fn main() {\n    println!(\"hello clean code\");\n}\n"
            }),
        }],
        input_tokens: 200,
        output_tokens: 50,
    };

    // Step 2: Tool call reply to announce fix
    let step2_resp = LlmResponse {
        content: None,
        tool_calls: vec![ToolCall {
            id: "call_reply_2".to_string(),
            name: "reply".to_string(),
            arguments: json!({
                "body": "Fixed formatting and replaced code."
            }),
        }],
        input_tokens: 250,
        output_tokens: 30,
    };

    // Step 3: Final turn
    let step3_resp = LlmResponse {
        content: Some("Done!".to_string()),
        tool_calls: vec![],
        input_tokens: 100,
        output_tokens: 10,
    };

    let mock_llm = MockLlm::new(vec![step1_resp, step2_resp, step3_resp]);

    let runner = AgentRunner::new(
        &sandbox,
        None, // Dry-run / standalone mode without real GitHub REST client
        &mock_llm,
        "test/repo",
        1,
        &skills,
    );

    let result = runner.run(vec![]).await.unwrap();

    assert!(result.success);
    assert_eq!(result.replies_posted, 1);
    assert_eq!(result.steps_taken, 3);

    // Verify file was actually modified inside sandbox
    let updated_content = sandbox.read_file("src/main.rs").await.unwrap();
    assert_eq!(
        updated_content,
        "fn main() {\n    println!(\"hello clean code\");\n}\n"
    );
}

#[tokio::test]
async fn test_progressive_skill_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    // Create .agents/skills/next-practices/SKILL.md
    let skill_dir = root.join(".agents").join("skills").join("next-practices");
    tokio::fs::create_dir_all(&skill_dir).await.unwrap();
    let skill_md = r#"---
name: next-practices
description: Best practices for Next.js 16
---

# Next.js 16 Best Practices
- Use Server Components
- Use Server Actions
"#;
    tokio::fs::write(skill_dir.join("SKILL.md"), skill_md)
        .await
        .unwrap();

    let search_paths = [root.join(".agents").join("skills")];
    let search_refs: Vec<&std::path::Path> = search_paths.iter().map(|p| p.as_path()).collect();
    let discovered = discover_skills(&search_refs).await;

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].name, "next-practices");
    assert_eq!(discovered[0].description, "Best practices for Next.js 16");
    assert!(discovered[0].content.contains("Use Server Components"));
}
