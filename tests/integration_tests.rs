use axum::body::Body;
use axum::http::{Request, StatusCode};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use sha2::digest::KeyInit;
use std::sync::Arc;
use tower::ServiceExt;

use openreview::agent::runner::{AgentRunner, AgentTerminationReason};
use openreview::config::Config;
use openreview::github::auth::TokenManager;
use openreview::github::client::is_permission_authorized_for_fix;
use openreview::jobs::JobManager;
use openreview::llm::traits::LlmProvider;
use openreview::llm::types::{ChatMessage, LlmResponse, ToolCall, ToolDefinition};
use openreview::sandbox::traits::SandboxInstance;
use openreview::sandbox::{LocalSandbox, LocalSandboxDriver, SandboxDriver};
use openreview::server::{AppState, Metrics, create_router};
use openreview::skills::discover_skills;
use openreview::workflow::command::{WorkflowMode, parse_bot_command};
use openreview::workflow::context::PullRequestContext;
use openreview::workflow::publisher::WorkflowPublisher;
use openreview::workflow::verification::{ProjectVerifier, VerificationCheck, VerificationReport};
use openreview::workflow::{WorkflowCoordinator, WorkflowParams};

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
        jobs: Arc::new(JobManager::default()),
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
async fn test_manifest_endpoint() {
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
        jobs: Arc::new(JobManager::default()),
    };

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/manifest")
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
    assert_eq!(json_val["name"], "OpenReview AI Agent");
    assert_eq!(json_val["default_permissions"]["pull_requests"], "write");
    assert_eq!(json_val["default_permissions"]["contents"], "write");
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
        jobs: Arc::new(JobManager::default()),
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
    assert!(text.contains("openreview_fixes_completed_total"));
}

#[tokio::test]
async fn test_webhook_signature_and_deduplication() {
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
        jobs: Arc::new(JobManager::default()),
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

    // 3. Valid delivery with mention -> 202 Accepted
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
            "body": "@openreview review please check security",
            "user": { "id": 1, "login": "developer" },
            "html_url": "https://github.com/test/repo/pull/55#issuecomment-200"
        },
        "repository": {
            "id": 100,
            "name": "repo",
            "full_name": "test/repo",
            "owner": { "id": 1, "login": "test" }
        },
        "installation": {
            "id": 8888
        }
    });

    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let sig = sign_payload(&secret, &payload_bytes);

    let req_valid = Request::builder()
        .method("POST")
        .uri("/api/webhooks/github")
        .header("content-type", "application/json")
        .header("x-github-event", "issue_comment")
        .header("x-github-delivery", "delivery-uuid-12345")
        .header("x-hub-signature-256", sig.clone())
        .body(Body::from(payload_bytes.clone()))
        .unwrap();

    let res_valid = app.clone().oneshot(req_valid).await.unwrap();
    assert_eq!(res_valid.status(), StatusCode::ACCEPTED);

    // 4. Duplicate delivery ID -> 200 OK with duplicate ignored status
    let req_duplicate = Request::builder()
        .method("POST")
        .uri("/api/webhooks/github")
        .header("content-type", "application/json")
        .header("x-github-event", "issue_comment")
        .header("x-github-delivery", "delivery-uuid-12345")
        .header("x-hub-signature-256", sig)
        .body(Body::from(payload_bytes))
        .unwrap();

    let res_dup = app.oneshot(req_duplicate).await.unwrap();
    assert_eq!(res_dup.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res_dup.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_res: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json_res["reason"], "duplicate_delivery");
}

#[tokio::test]
async fn test_agent_react_loop_review_mode_blocks_write_file() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = LocalSandbox::new(temp, "owner/repo".to_string(), String::new());

    sandbox
        .write_file("src/main.rs", "fn main() { println!(\"original\"); }")
        .await
        .unwrap();

    // Adversarial step: LLM tries to invoke writeFile in Review mode
    let write_call = LlmResponse {
        content: None,
        tool_calls: vec![ToolCall {
            id: "call_write_exploit".to_string(),
            name: "writeFile".to_string(),
            arguments: json!({
                "path": "src/main.rs",
                "content": "fn main() { println!(\"overwritten\"); }"
            }),
        }],
        input_tokens: 100,
        output_tokens: 50,
    };

    let reply_call = LlmResponse {
        content: Some("Review finished".to_string()),
        tool_calls: vec![],
        input_tokens: 100,
        output_tokens: 20,
    };

    let mock_llm = MockLlm::new(vec![write_call, reply_call]);

    let runner = AgentRunner::new(
        &sandbox,
        None,
        &mock_llm,
        "owner/repo",
        1,
        WorkflowMode::Review, // Review Mode Active
        true,
        &[],
        None,
    );

    let result = runner.run(vec![]).await.unwrap();
    assert!(result.success);

    // File content MUST remain untouched because writeFile was blocked
    let file_content = sandbox.read_file("src/main.rs").await.unwrap();
    assert_eq!(file_content, "fn main() { println!(\"original\"); }");
}

#[tokio::test]
async fn test_agent_react_loop_fix_mode_with_tools() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = LocalSandbox::new(temp, "owner/repo".to_string(), String::new());

    sandbox
        .write_file("src/main.rs", "fn main() { println!(\"bad\"); }")
        .await
        .unwrap();

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

    let step3_resp = LlmResponse {
        content: Some("Done!".to_string()),
        tool_calls: vec![],
        input_tokens: 100,
        output_tokens: 10,
    };

    let mock_llm = MockLlm::new(vec![step1_resp, step2_resp, step3_resp]);

    let runner = AgentRunner::new(
        &sandbox,
        None,
        &mock_llm,
        "test/repo",
        1,
        WorkflowMode::Fix, // Fix Mode Active
        true,
        &[],
        None,
    );

    let result = runner.run(vec![]).await.unwrap();

    assert!(result.success);
    assert_eq!(result.replies_posted, 1);
    assert_eq!(result.steps_taken, 3);
    assert_eq!(result.termination_reason, AgentTerminationReason::Completed);

    let updated_content = sandbox.read_file("src/main.rs").await.unwrap();
    assert_eq!(
        updated_content,
        "fn main() {\n    println!(\"hello clean code\");\n}\n"
    );
}

#[tokio::test]
async fn test_agent_step_limit_exceeded() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = LocalSandbox::new(temp, "owner/repo".to_string(), String::new());

    // Continuous tool calls that exceed 20 steps
    let mut responses = Vec::new();
    for i in 1..=25 {
        responses.push(LlmResponse {
            content: None,
            tool_calls: vec![ToolCall {
                id: format!("call_{i}"),
                name: "bash".to_string(),
                arguments: json!({ "command": "echo loop" }),
            }],
            input_tokens: 50,
            output_tokens: 20,
        });
    }

    let mock_llm = MockLlm::new(responses);

    let runner = AgentRunner::new(
        &sandbox,
        None,
        &mock_llm,
        "test/repo",
        1,
        WorkflowMode::Review,
        true,
        &[],
        None,
    );

    let result = runner.run(vec![]).await.unwrap();
    // Must NOT be marked success when max steps exceeded!
    assert!(!result.success);
    assert_eq!(result.steps_taken, 20);
    assert_eq!(
        result.termination_reason,
        AgentTerminationReason::StepLimitExceeded
    );
    assert!(result.error_message.unwrap().contains("20 steps"));
}

#[test]
fn test_permission_authorization_model() {
    assert!(is_permission_authorized_for_fix("admin"));
    assert!(is_permission_authorized_for_fix("write"));
    assert!(is_permission_authorized_for_fix("maintain"));
    assert!(!is_permission_authorized_for_fix("triage"));
    assert!(!is_permission_authorized_for_fix("read"));
    assert!(!is_permission_authorized_for_fix("none"));
}

#[tokio::test]
async fn test_publishing_prevented_in_review_mode() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = LocalSandbox::new(temp, "owner/repo".to_string(), String::new());

    sandbox.write_file("file.txt", "content").await.unwrap();

    let params = WorkflowParams {
        repo_full_name: "owner/repo".to_string(),
        pr_number: 42,
        base_branch: "main".to_string(),
        pr_branch: "feature".to_string(),
        comment_id: None,
        installation_id: Some(123),
        mode: WorkflowMode::Review, // Review Mode
        actor: "alice".to_string(),
        dry_run: false,
        head_sha: None,
        messages: vec![],
    };

    let err = WorkflowPublisher::evaluate_and_publish(&sandbox, None, &params)
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        openreview::error::AppError::PublishingForbiddenInReviewMode
    ));
}

#[tokio::test]
async fn test_publishing_dry_run_suppresses_push() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = LocalSandbox::new(temp, "owner/repo".to_string(), String::new());

    let params = WorkflowParams {
        repo_full_name: "owner/repo".to_string(),
        pr_number: 42,
        base_branch: "main".to_string(),
        pr_branch: "feature".to_string(),
        comment_id: None,
        installation_id: Some(123),
        mode: WorkflowMode::Fix,
        actor: "alice".to_string(),
        dry_run: true, // Dry Run Active
        head_sha: None,
        messages: vec![],
    };

    let record = WorkflowPublisher::evaluate_and_publish(&sandbox, None, &params)
        .await
        .unwrap();

    assert_eq!(record.action_taken, "dry_run_simulated");
    assert!(record.success);
    assert_eq!(record.published_sha, None);
}

#[tokio::test]
async fn test_progressive_skill_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let skill_dir = root.join(".agents").join("skills").join("next-practices");
    tokio::fs::create_dir_all(&skill_dir).await.unwrap();
    let skill_md = r#"---
name: next-practices
description: Best practices for Next.js 16
---

# Next.js 16 Best Practices
- Use Server Components
"#;
    tokio::fs::write(skill_dir.join("SKILL.md"), skill_md)
        .await
        .unwrap();

    let search_paths = [root.join(".agents").join("skills")];
    let search_refs: Vec<&std::path::Path> = search_paths.iter().map(|p| p.as_path()).collect();
    let discovered = discover_skills(&search_refs).await;

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].name, "next-practices");
    assert!(discovered[0].content.contains("Use Server Components"));
}

#[tokio::test]
async fn test_pr_context_formatting() {
    let ctx = PullRequestContext {
        repo_full_name: "owner/repo".to_string(),
        pr_number: 101,
        title: "Security patch".to_string(),
        description: "Closes CVE-1234".to_string(),
        base_branch: "main".to_string(),
        base_sha: "abc1234".to_string(),
        head_branch: "patch-1".to_string(),
        head_sha: "def5678".to_string(),
        is_fork: true,
        diff: "diff --git a/src/lib.rs b/src/lib.rs\n+ safe_code();".to_string(),
        changed_files: vec![openreview::github::types::ChangedFile {
            filename: "src/lib.rs".to_string(),
            status: "modified".to_string(),
            additions: 1,
            deletions: 0,
            changes: 1,
            patch: Some("+ safe_code();".to_string()),
        }],
    };

    let prompt = ctx.to_prompt_context();
    assert!(prompt.contains("Pull Request Context: #101: Security patch"));
    assert!(prompt.contains("This PR originates from an external fork"));
    assert!(prompt.contains("src/lib.rs"));
    assert!(prompt.contains("safe_code()"));
}

#[tokio::test]
async fn test_multi_installation_token_isolation() {
    let token_mgr = TokenManager::new();
    // Initially, invalidating a nonexistent token does not crash
    token_mgr.invalidate(1001).await;
    token_mgr.invalidate(2002).await;
}

#[test]
fn test_bot_command_matrix() {
    let fix_cmd = parse_bot_command("@openreview fix please format code", "openreview").unwrap();
    assert_eq!(fix_cmd.mode, WorkflowMode::Fix);
    assert_eq!(fix_cmd.prompt, "please format code");

    let review_cmd =
        parse_bot_command("@openreview review check performance", "openreview").unwrap();
    assert_eq!(review_cmd.mode, WorkflowMode::Review);
    assert_eq!(review_cmd.prompt, "check performance");

    let suggest_cmd = parse_bot_command("@openreview suggest refactoring", "openreview").unwrap();
    assert_eq!(suggest_cmd.mode, WorkflowMode::Review);
    assert_eq!(suggest_cmd.prompt, "refactoring");

    let unqualified = parse_bot_command("@openreview any concerns?", "openreview").unwrap();
    assert_eq!(unqualified.mode, WorkflowMode::Review);
}

#[test]
fn test_verification_report_markdown_generation() {
    let report = VerificationReport {
        all_passed: false,
        checks: vec![VerificationCheck {
            name: "Linter".to_string(),
            command: "cargo clippy".to_string(),
            passed: false,
            exit_code: 1,
            output_snippet: "error: aborting due to 1 previous error".to_string(),
        }],
        skipped_reason: None,
    };

    let md = report.to_markdown();
    assert!(md.contains("❌ **Linter**"));
    assert!(md.contains("Automatic publishing blocked"));
}

#[tokio::test]
async fn test_project_verifier_detects_clean_empty_dir() {
    let temp = tempfile::tempdir().unwrap();
    let sandbox = LocalSandbox::new(temp, "owner/repo".to_string(), String::new());
    let report = ProjectVerifier::run_verification(&sandbox).await.unwrap();
    assert!(report.all_passed);
    assert!(report.checks.is_empty());
}
