use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid webhook signature")]
    InvalidWebhookSignature,

    #[error("Missing webhook signature header")]
    MissingWebhookSignature,

    #[error("Unsupported webhook event: {0}")]
    UnsupportedWebhookEvent(String),

    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("LLM provider error: {0}")]
    Llm(String),

    #[error("Skill error: {0}")]
    Skill(String),

    #[error("Workflow error: {0}")]
    Workflow(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Publishing forbidden in review mode")]
    PublishingForbiddenInReviewMode,

    #[error("Missing installation ID in webhook payload")]
    MissingInstallationId,

    #[error("Missing GitHub webhook delivery ID")]
    MissingDeliveryId,

    #[error("Command execution failed ({exit_code}): {output}")]
    CommandFailed { exit_code: i32, output: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::InvalidWebhookSignature => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::MissingWebhookSignature => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::UnsupportedWebhookEvent(_) => (StatusCode::OK, self.to_string()),
            AppError::GitHubApi(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::Auth(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Sandbox(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Llm(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::Skill(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Unauthorized(_) => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::PathTraversal(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::VerificationFailed(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            AppError::PublishingForbiddenInReviewMode => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::MissingInstallationId => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::MissingDeliveryId => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Workflow(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::CommandFailed { .. } => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = json!({
            "error": message,
            "status": status.as_u16()
        });

        (status, axum::Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
