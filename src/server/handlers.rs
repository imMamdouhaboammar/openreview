use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use sha2::digest::KeyInit;
use std::sync::atomic::Ordering;
use tracing::{error, info, warn};

use crate::error::AppError;
use crate::github::types::{IssueCommentPayload, PullRequestReviewCommentPayload};
use crate::github::verify_signature;
use crate::jobs::JobStatus;
use crate::server::state::AppState;
use crate::workflow::WorkflowParams;
use crate::workflow::command::{WorkflowMode, parse_bot_command};

pub async fn webhook_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    state
        .metrics
        .webhooks_received
        .fetch_add(1, Ordering::Relaxed);

    // 1. Signature Verification (Constant-Time HMAC SHA-256)
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

    let event_type = headers
        .get("x-github-event")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    if matches!(event_type, "issue_comment" | "pull_request_review_comment") {
        let envelope: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| AppError::Internal(format!("Invalid webhook envelope: {e}")))?;
        if envelope
            .pointer("/installation/id")
            .and_then(|v| v.as_u64())
            .is_none()
        {
            return Err(AppError::MissingInstallationId);
        }
    }

    // 2. Delivery Idempotency and Deduplication
    let delivery_id = headers
        .get("x-github-delivery")
        .and_then(|h| h.to_str().ok())
        .filter(|s| !s.is_empty())
        .ok_or(AppError::MissingDeliveryId)?
        .to_string();

    // 3. Identify Event Type

    match event_type {
        "issue_comment" => {
            let payload: IssueCommentPayload = serde_json::from_slice(&body)
                .map_err(|e| AppError::Internal(format!("Failed to parse issue_comment: {e}")))?;

            if payload.action == "created"
                && !is_bot_actor(&payload.comment.user, &state.config.bot_username)
                && payload.issue.pull_request.is_some()
                && let Some(cmd) =
                    parse_bot_command(&payload.comment.body, &state.config.bot_username)
            {
                let installation_id = payload.installation.map(|i| i.id);
                let repo_full_name = payload.repository.full_name.clone();
                let pr_number = payload.issue.number;
                let comment_id = payload.comment.id;
                let actor = payload.comment.user.login.clone();
                let comment_body = payload.comment.body.clone();

                info!(
                    "Bot commanded ({:?}) by @{} in comment #{} on repo {}",
                    cmd.mode, actor, pr_number, repo_full_name
                );

                let job = state
                    .jobs
                    .create_job(
                        &delivery_id,
                        "issue_comment",
                        &repo_full_name,
                        pr_number,
                        installation_id,
                        &actor,
                        cmd.mode,
                    )
                    .await?;
                let Some(job) = job else {
                    return Ok(duplicate_delivery_response(&delivery_id));
                };

                let coordinator = state.coordinator.clone();
                let metrics = state.metrics.clone();
                let jobs = state.jobs.clone();
                let job_id = job.id.clone();
                let mode = cmd.mode;

                metrics.active_workflows.fetch_add(1, Ordering::Relaxed);

                tokio::spawn(async move {
                    let sem = jobs.semaphore();
                    let _permit = sem.acquire().await;
                    if !persist_job_status(&jobs, &job_id, JobStatus::Running, None).await {
                        metrics.active_workflows.fetch_sub(1, Ordering::Relaxed);
                        return;
                    }

                    let params = WorkflowParams {
                        repo_full_name: repo_full_name.clone(),
                        pr_number,
                        base_branch: "main".to_string(),
                        pr_branch: "HEAD".to_string(),
                        comment_id: Some(comment_id),
                        installation_id,
                        mode,
                        actor: actor.clone(),
                        dry_run: false,
                        head_sha: None,
                        from_webhook: true,
                        messages: vec![crate::workflow::ThreadMessage {
                            role: "user".to_string(),
                            content: if !cmd.prompt.is_empty() {
                                cmd.prompt
                            } else {
                                comment_body
                            },
                        }],
                    };

                    match coordinator.execute_bot_workflow(params).await {
                        Ok(res) => {
                            if res.success {
                                persist_job_status(&jobs, &job_id, JobStatus::Completed, None)
                                    .await;
                                if mode == WorkflowMode::Fix {
                                    metrics.fixes_completed.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    metrics.reviews_completed.fetch_add(1, Ordering::Relaxed);
                                }
                                info!(
                                    "Job {} completed successfully for PR #{}",
                                    job_id, pr_number
                                );
                            } else {
                                let err = res.error_message.unwrap_or_else(|| {
                                    "Agent loop terminated with error".to_string()
                                });
                                persist_job_status(
                                    &jobs,
                                    &job_id,
                                    JobStatus::Failed,
                                    Some(err.clone()),
                                )
                                .await;
                                if mode == WorkflowMode::Fix {
                                    metrics.fixes_failed.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    metrics.reviews_failed.fetch_add(1, Ordering::Relaxed);
                                }
                                warn!("Job {} finished with failure: {}", job_id, err);
                            }
                        }
                        Err(e) => {
                            persist_job_status(
                                &jobs,
                                &job_id,
                                JobStatus::Failed,
                                Some(e.to_string()),
                            )
                            .await;
                            if mode == WorkflowMode::Fix {
                                metrics.fixes_failed.fetch_add(1, Ordering::Relaxed);
                            } else {
                                metrics.reviews_failed.fetch_add(1, Ordering::Relaxed);
                            }
                            error!("Job {} encountered critical error: {}", job_id, e);
                        }
                    }
                    metrics.active_workflows.fetch_sub(1, Ordering::Relaxed);
                });

                return Ok((
                    StatusCode::ACCEPTED,
                    Json(json!({
                        "status": "accepted",
                        "job_id": job.id,
                        "delivery_id": delivery_id,
                        "mode": mode.to_string()
                    })),
                )
                    .into_response());
            }
        }
        "pull_request_review_comment" => {
            let payload: PullRequestReviewCommentPayload = serde_json::from_slice(&body)
                .map_err(|e| AppError::Internal(format!("Failed to parse review_comment: {e}")))?;

            if payload.action == "created"
                && !is_bot_actor(&payload.comment.user, &state.config.bot_username)
                && let Some(cmd) =
                    parse_bot_command(&payload.comment.body, &state.config.bot_username)
            {
                let installation_id = payload.installation.map(|i| i.id);
                let repo_full_name = payload.repository.full_name.clone();
                let pr_number = payload.pull_request.number;
                let comment_id = payload.comment.id;
                let actor = payload.comment.user.login.clone();
                let base_branch = payload.pull_request.base.r#ref.clone();
                let pr_branch = payload.pull_request.head.r#ref.clone();
                let head_sha = payload.pull_request.head.sha.clone();
                let comment_body = payload.comment.body.clone();

                let job = state
                    .jobs
                    .create_job(
                        &delivery_id,
                        "pull_request_review_comment",
                        &repo_full_name,
                        pr_number,
                        installation_id,
                        &actor,
                        cmd.mode,
                    )
                    .await?;
                let Some(job) = job else {
                    return Ok(duplicate_delivery_response(&delivery_id));
                };

                let coordinator = state.coordinator.clone();
                let metrics = state.metrics.clone();
                let jobs = state.jobs.clone();
                let job_id = job.id.clone();
                let mode = cmd.mode;

                metrics.active_workflows.fetch_add(1, Ordering::Relaxed);

                tokio::spawn(async move {
                    let sem = jobs.semaphore();
                    let _permit = sem.acquire().await;
                    if !persist_job_status(&jobs, &job_id, JobStatus::Running, None).await {
                        metrics.active_workflows.fetch_sub(1, Ordering::Relaxed);
                        return;
                    }

                    let params = WorkflowParams {
                        repo_full_name,
                        pr_number,
                        base_branch,
                        pr_branch,
                        comment_id: Some(comment_id),
                        installation_id,
                        mode,
                        actor,
                        dry_run: false,
                        head_sha: Some(head_sha),
                        from_webhook: true,
                        messages: vec![crate::workflow::ThreadMessage {
                            role: "user".to_string(),
                            content: if !cmd.prompt.is_empty() {
                                cmd.prompt
                            } else {
                                comment_body
                            },
                        }],
                    };

                    match coordinator.execute_bot_workflow(params).await {
                        Ok(res) => {
                            if res.success {
                                persist_job_status(&jobs, &job_id, JobStatus::Completed, None)
                                    .await;
                                if mode == WorkflowMode::Fix {
                                    metrics.fixes_completed.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    metrics.reviews_completed.fetch_add(1, Ordering::Relaxed);
                                }
                            } else {
                                let err = res.error_message.unwrap_or_else(|| {
                                    "Agent loop terminated with error".to_string()
                                });
                                persist_job_status(&jobs, &job_id, JobStatus::Failed, Some(err))
                                    .await;
                                if mode == WorkflowMode::Fix {
                                    metrics.fixes_failed.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    metrics.reviews_failed.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        Err(e) => {
                            persist_job_status(
                                &jobs,
                                &job_id,
                                JobStatus::Failed,
                                Some(e.to_string()),
                            )
                            .await;
                            if mode == WorkflowMode::Fix {
                                metrics.fixes_failed.fetch_add(1, Ordering::Relaxed);
                            } else {
                                metrics.reviews_failed.fetch_add(1, Ordering::Relaxed);
                            }
                            error!("Review workflow failed: {e}");
                        }
                    }
                    metrics.active_workflows.fetch_sub(1, Ordering::Relaxed);
                });

                return Ok((
                    StatusCode::ACCEPTED,
                    Json(json!({
                        "status": "accepted",
                        "job_id": job.id,
                        "delivery_id": delivery_id,
                        "mode": mode.to_string()
                    })),
                )
                    .into_response());
            }
        }
        "installation" | "installation_repositories" => {
            // Revoke cached credentials when installation membership or access changes.
            if let Ok(envelope) = serde_json::from_slice::<serde_json::Value>(&body)
                && let Some(id) = envelope
                    .pointer("/installation/id")
                    .and_then(|v| v.as_u64())
            {
                state.coordinator.token_manager.invalidate(id).await;
            }
            info!("Received GitHub App installation lifecycle event: {event_type}");
            return Ok((
                StatusCode::OK,
                Json(json!({
                    "status": "lifecycle_acknowledged",
                    "event": event_type
                })),
            )
                .into_response());
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

fn duplicate_delivery_response(delivery_id: &str) -> Response {
    info!("Duplicate webhook delivery {} ignored", delivery_id);
    (
        StatusCode::OK,
        Json(json!({
            "status": "ignored", "reason": "duplicate_delivery", "delivery_id": delivery_id
        })),
    )
        .into_response()
}

async fn persist_job_status(
    manager: &crate::jobs::JobManager,
    id: &str,
    status: JobStatus,
    error: Option<String>,
) -> bool {
    match manager.update_status(id, status, error).await {
        Ok(()) => true,
        Err(e) => {
            error!("Cannot persist job {id} status: {e}");
            false
        }
    }
}

fn is_bot_actor(user: &crate::github::types::User, bot_username: &str) -> bool {
    user.r#type.eq_ignore_ascii_case("bot")
        || user
            .login
            .eq_ignore_ascii_case(&format!("{bot_username}[bot]"))
}

pub async fn get_job_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<crate::jobs::JobRecord>, StatusCode> {
    authorize_job_api(&headers, &state.config)?;
    match state.jobs.get_job(&id).await {
        Some(job) => Ok(Json(job)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn list_jobs_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::jobs::JobRecord>>, StatusCode> {
    authorize_job_api(&headers, &state.config)?;
    Ok(Json(state.jobs.list_jobs().await))
}

fn authorize_job_api(
    headers: &HeaderMap,
    config: &crate::config::Config,
) -> Result<(), StatusCode> {
    let configured = config
        .job_api_token
        .as_deref()
        .filter(|token| token.len() >= 32)
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    // Compare keyed digests without revealing a matching token prefix through timing.
    let mut mac = Hmac::<Sha256>::new_from_slice(configured.as_bytes())
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    mac.update(b"openreview-job-api-auth-v1");
    let mut expected = Hmac::<Sha256>::new_from_slice(provided.as_bytes())
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    expected.update(b"openreview-job-api-auth-v1");
    mac.verify_slice(&expected.finalize().into_bytes())
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

pub async fn manifest_handler() -> Json<serde_json::Value> {
    Json(json!({
        "name": "OpenReview AI Agent",
        "url": "https://github.com/imMamdouhaboammar/openreview",
        "hook_attributes": {
            "url": "https://YOUR_DOMAIN/api/webhooks/github"
        },
        "redirect_url": "https://YOUR_DOMAIN/",
        "public": false,
        "default_permissions": {
            "pull_requests": "write",
            "issues": "write",
            "contents": "write",
            "metadata": "read"
        },
        "default_events": [
            "issue_comment",
            "pull_request",
            "pull_request_review_comment"
        ]
    }))
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
    let rev_completed = state.metrics.reviews_completed.load(Ordering::Relaxed);
    let rev_failed = state.metrics.reviews_failed.load(Ordering::Relaxed);
    let fix_completed = state.metrics.fixes_completed.load(Ordering::Relaxed);
    let fix_failed = state.metrics.fixes_failed.load(Ordering::Relaxed);
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
         # HELP openreview_fixes_completed_total Total successful code fixes\n\
         # TYPE openreview_fixes_completed_total counter\n\
         openreview_fixes_completed_total {}\n\
         # HELP openreview_fixes_failed_total Total failed code fixes\n\
         # TYPE openreview_fixes_failed_total counter\n\
         openreview_fixes_failed_total {}\n\
         # HELP openreview_active_workflows Current active workflows\n\
         # TYPE openreview_active_workflows gauge\n\
         openreview_active_workflows {}\n",
        uptime, webhooks, rev_completed, rev_failed, fix_completed, fix_failed, active
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
        <p>Production-ready self-hosted AI code review and fix agent in Rust.</p>
        <ul>
            <li><span class="label">LLM Provider</span> <code>{}</code></li>
            <li><span class="label">Active Model</span> <code>{}</code></li>
            <li><span class="label">Sandbox Driver</span> <code>{}</code></li>
            <li><span class="label">Webhook Path</span> <code>/api/webhooks/github</code></li>
            <li><span class="label">App Manifest</span> <a href="/api/manifest" style="color:#3b82f6">/api/manifest</a></li>
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
