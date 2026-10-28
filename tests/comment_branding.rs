use axum::{Json, Router, http::StatusCode, routing::post};
use openreview::github::client::GitHubClient;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

#[tokio::test]
async fn posted_pr_comment_has_one_logo_and_keeps_the_actual_finding() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (sender, receiver) = oneshot::channel::<Value>();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let app = Router::new().route(
        "/repos/owner/repo/issues/42/comments",
        post(move |Json(body): Json<Value>| {
            let sender = sender.clone();
            async move {
                if let Some(tx) = sender.lock().await.take() {
                    let _ = tx.send(body);
                }
                (StatusCode::CREATED, Json(json!({"id": 123})))
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let github =
        GitHubClient::with_base_url("test-only-token".into(), format!("http://{addr}")).unwrap();
    github
        .post_comment(
            "owner",
            "repo",
            42,
            "P1 · `src/lib.rs:12` can panic.\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*",
        )
        .await
        .unwrap();
    let body = receiver.await.unwrap()["body"]
        .as_str()
        .unwrap()
        .to_string();
    server.abort();

    assert!(body.contains("P1 · `src/lib.rs:12` can panic."));
    assert!(body.contains("assets/openreview-mark.svg"));
    assert!(body.contains("https://github.com/imMamdouhaboammar/openreview)"));
    assert!(!body.contains("openreview-rs"));
    assert_eq!(body.matches("OpenReview logo").count(), 1);
}

#[tokio::test]
async fn empty_pr_comment_is_rejected_without_a_network_call() {
    let github =
        GitHubClient::with_base_url("test-only-token".into(), "http://127.0.0.1:1".into()).unwrap();
    let err = github
        .post_comment("owner", "repo", 42, "   \n")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("empty PR comment"));
}
