use crate::server::handlers::{
    get_job_handler, health_handler, list_jobs_handler, manifest_handler, metrics_handler,
    root_handler, webhook_handler,
};
use crate::server::state::AppState;
use axum::Router;
use axum::routing::{get, post};
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/api/manifest", get(manifest_handler))
        .route("/api/jobs", get(list_jobs_handler))
        .route("/api/jobs/{id}", get(get_job_handler))
        .route("/api/webhooks/github", post(webhook_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
