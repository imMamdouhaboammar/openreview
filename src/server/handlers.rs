use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde_json::json;
use std::sync::atomic::Ordering;
use tracing::{error, info, warn};

use crate::error::AppError;
use crate::github::types::{IssueCommentPayload, PullRequestReviewCommentPayload, ReactionPayload};
use crate::github::verify_signature;
use crate::server::state::AppState;
use crate::workflow::WorkflowParams;

pub async fn webhook_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    state
        .metrics
        .webhooks_received
        .fetch_add(1, Ordering::Relaxed);

    // 1. Signature Verification
    let signature_header = headers
        .get("x-hub-signature-256")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::MissingWebhookSignature)?;

    let is_valid = verify_signature(
        &state.config.github_app_webhook_secret,
        &body,
        signature_header,
    )?;

    if !is_valid {
        warn!("Invalid HMAC signature on GitHub webhook");
        return Err(AppError::InvalidWebhookSignature);
    }

    // 2. Identify Event Type
    let event_type = headers
        .get("x-github-event")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();

    let bot_mention = format!("@{}", state.config.bot_username.to_lowercase());

    match event_type {
        "issue_comment" => {
            let payload: IssueCommentPayload = serde_json::from_slice(&body)
                .map_err(|e| AppError::Internal(format!("Failed to parse issue_comment: {e}")))?;

            if payload.action == "created" && payload.issue.pull_request.is_some() {
                let comment_lower = payload.comment.body.to_lowercase();
                if comment_lower.contains(&bot_mention) {
                    info!(
                        "Bot mentioned in issue comment #{} on repo {}",
                        payload.issue.number, payload.repository.full_name
                    );

                    let coordinator = state.coordinator.clone();
                    let metrics = state.metrics.clone();
                    let repo_full_name = payload.repository.full_name.clone();
                    let pr_number = payload.issue.number;
                    let comment_id = payload.comment.id;
                    let comment_body = payload.comment.body.clone();

                    metrics.active_workflows.fetch_add(1, Ordering::Relaxed);

                    tokio::spawn(async move {
                        let params = WorkflowParams {
                            repo_full_name: repo_full_name.clone(),
                            pr_number,
                            base_branch: "main".to_string(), // will resolve dynamically in client
                            pr_branch: "HEAD".to_string(),
                            comment_id: Some(comment_id),
                            messages: vec![crate::workflow::ThreadMessage {
                                role: "user".to_string(),
                                content: comment_body,
                            }],
                        };

                        match coordinator.execute_bot_workflow(params).await {
                            Ok(_) => {
                                metrics.reviews_completed.fetch_add(1, Ordering::Relaxed);
                                info!("Successfully completed review workflow for PR #{pr_number}");
                            }
                            Err(e) => {
                                metrics.reviews_failed.fetch_add(1, Ordering::Relaxed);
                                error!("Review workflow failed for PR #{pr_number}: {e}");
                            }
                        }
                        metrics.active_workflows.fetch_sub(1, Ordering::Relaxed);
                    });

                    return Ok((
                        StatusCode::ACCEPTED,
                        Json(json!({
                            "status": "accepted",
                            "message": "Review workflow enqueued"
                        })),
                    )
                        .into_response());
                }
            }
        }
        "pull_request_review_comment" => {
            let payload: PullRequestReviewCommentPayload = serde_json::from_slice(&body)
                .map_err(|e| AppError::Internal(format!("Failed to parse review_comment: {e}")))?;

            if payload.action == "created" {
                let comment_lower = payload.comment.body.to_lowercase();
                if comment_lower.contains(&bot_mention) {
                    info!(
                        "Bot mentioned in PR review comment on {} #{}",
                        payload.repository.full_name, payload.pull_request.number
                    );

                    let coordinator = state.coordinator.clone();
                    let metrics = state.metrics.clone();
                    let repo_full_name = payload.repository.full_name.clone();
                    let pr_number = payload.pull_request.number;
                    let comment_id = payload.comment.id;
                    let comment_body = payload.comment.body.clone();
                    let base_branch = payload.pull_request.base.r#ref.clone();
                    let pr_branch = payload.pull_request.head.r#ref.clone();

                    metrics.active_workflows.fetch_add(1, Ordering::Relaxed);

                    tokio::spawn(async move {
                        let params = WorkflowParams {
                            repo_full_name,
                            pr_number,
                            base_branch,
                            pr_branch,
                            comment_id: Some(comment_id),
                            messages: vec![crate::workflow::ThreadMessage {
                                role: "user".to_string(),
                                content: comment_body,
                            }],
                        };

                        match coordinator.execute_bot_workflow(params).await {
                            Ok(_) => {
                                metrics.reviews_completed.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(e) => {
                                metrics.reviews_failed.fetch_add(1, Ordering::Relaxed);
                                error!("Review workflow failed: {e}");
                            }
                        }
                        metrics.active_workflows.fetch_sub(1, Ordering::Relaxed);
                    });

                    return Ok((
                        StatusCode::ACCEPTED,
                        Json(json!({
                            "status": "accepted",
                            "message": "Review workflow enqueued"
                        })),
                    )
                        .into_response());
                }
            }
        }
        "reaction" => {
            let payload: ReactionPayload = serde_json::from_slice(&body)
                .map_err(|e| AppError::Internal(format!("Failed to parse reaction: {e}")))?;

            if payload.action == "created"
                && let Some(comment) = payload.comment
            {
                let coordinator = state.coordinator.clone();
                let repo = payload.repository.full_name.clone();
                let reaction = payload.reaction.content.clone();
                let user_login = payload.reaction.user.login.clone();

                tokio::spawn(async move {
                    let _ = coordinator
                        .handle_reaction_event(
                            &repo,
                            0, // will resolve via comment thread
                            comment.id,
                            &reaction,
                            &user_login,
                        )
                        .await;
                });

                return Ok((
                    StatusCode::ACCEPTED,
                    Json(json!({ "status": "reaction_handled" })),
                )
                    .into_response());
            }
        }
        _ => {}
    }

    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "ignored",
            "event": event_type
        })),
    )
        .into_response())
}

pub async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let uptime = state.metrics.start_time.elapsed().as_secs();
    Json(json!({
        "status": "ok",
        "service": "openreview-rs",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime,
        "llm_provider": state.config.llm_provider,
        "model": state.config.model_name,
        "sandbox_driver": state.config.sandbox_driver
    }))
}

pub async fn metrics_handler(State(state): State<AppState>) -> Response {
    let uptime = state.metrics.start_time.elapsed().as_secs();
    let webhooks = state.metrics.webhooks_received.load(Ordering::Relaxed);
    let completed = state.metrics.reviews_completed.load(Ordering::Relaxed);
    let failed = state.metrics.reviews_failed.load(Ordering::Relaxed);
    let active = state.metrics.active_workflows.load(Ordering::Relaxed);

    let metrics_text = format!(
        "# HELP openreview_uptime_seconds Service uptime in seconds\n\
         # TYPE openreview_uptime_seconds gauge\n\
         openreview_uptime_seconds {}\n\
         # HELP openreview_webhooks_total Total GitHub webhooks received\n\
         # TYPE openreview_webhooks_total counter\n\
         openreview_webhooks_total {}\n\
         # HELP openreview_reviews_completed_total Total successful reviews\n\
         # TYPE openreview_reviews_completed_total counter\n\
         openreview_reviews_completed_total {}\n\
         # HELP openreview_reviews_failed_total Total failed reviews\n\
         # TYPE openreview_reviews_failed_total counter\n\
         openreview_reviews_failed_total {}\n\
         # HELP openreview_active_workflows Current active review workflows\n\
         # TYPE openreview_active_workflows gauge\n\
         openreview_active_workflows {}\n",
        uptime, webhooks, completed, failed, active
    );

    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        metrics_text,
    )
        .into_response()
}

pub async fn root_handler(State(state): State<AppState>) -> Html<String> {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OpenReview-RS Microservice</title>
    <style>
        body {{
            background: #09090b;
            color: #fafafa;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            padding: 24px;
        }}
        .card {{
            background: #18181b;
            border: 1px solid #27272a;
            border-radius: 8px;
            max-width: 600px;
            width: 100%;
            padding: 32px;
            box-shadow: 0 4px 20px rgba(0,0,0,0.5);
        }}
        h1 {{ font-size: 1.5rem; margin-top: 0; display: flex; align-items: center; gap: 8px; }}
        .badge {{
            background: #10b98122;
            color: #10b981;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 0.8rem;
            font-weight: 600;
        }}
        p {{ color: #a1a1aa; line-height: 1.5; }}
        ul {{ list-style: none; padding: 0; }}
        li {{ padding: 8px 0; border-bottom: 1px solid #27272a; display: flex; justify-content: space-between; }}
        span.label {{ color: #a1a1aa; }}
        code {{ font-family: monospace; color: #3b82f6; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>OpenReview-RS <span class="badge">Online</span></h1>
        <p>High-performance self-hosted AI code review bot and microservice in Rust.</p>
        <ul>
            <li><span class="label">LLM Provider</span> <code>{}</code></li>
            <li><span class="label">Active Model</span> <code>{}</code></li>
            <li><span class="label">Sandbox Driver</span> <code>{}</code></li>
            <li><span class="label">Webhook Path</span> <code>/api/webhooks/github</code></li>
            <li><span class="label">Health Check</span> <a href="/health" style="color:#3b82f6">/health</a></li>
            <li><span class="label">Metrics</span> <a href="/metrics" style="color:#3b82f6">/metrics</a></li>
        </ul>
    </div>
</body>
</html>"#,
        state.config.llm_provider, state.config.model_name, state.config.sandbox_driver
    );

    Html(html)
}
