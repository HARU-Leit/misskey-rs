//! Error types for misskey-rs.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Application result type.
pub type AppResult<T> = Result<T, AppError>;

/// Application error type.
#[derive(Debug, Error)]
pub enum AppError {
    // === Client Errors ===
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Note not found: {0}")]
    NoteNotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limited")]
    RateLimited,

    // === Server Errors ===
    #[error("Database error: {0}")]
    Database(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Federation error: {0}")]
    Federation(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Returns the HTTP status code for this error.
    #[must_use] 
    pub const fn status_code(&self) -> StatusCode {
        match self {
            // 4xx Client Errors
            Self::NotFound(_) | Self::UserNotFound(_) | Self::NoteNotFound(_) => {
                StatusCode::NOT_FOUND
            }
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::BadRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,

            // 5xx Server Errors
            Self::Database(_)
            | Self::Redis(_)
            | Self::Federation(_)
            | Self::Queue(_)
            | Self::Config(_)
            | Self::ExternalService(_)
            | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Returns the error code for API responses.
    #[must_use] 
    pub const fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "NOT_FOUND",
            Self::UserNotFound(_) => "USER_NOT_FOUND",
            Self::NoteNotFound(_) => "NOTE_NOT_FOUND",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::Conflict(_) => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Redis(_) => "REDIS_ERROR",
            Self::Federation(_) => "FEDERATION_ERROR",
            Self::Queue(_) => "QUEUE_ERROR",
            Self::Config(_) => "CONFIG_ERROR",
            Self::ExternalService(_) => "EXTERNAL_SERVICE_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }

    /// Returns whether this error should be logged at error level.
    #[must_use] 
    pub fn is_server_error(&self) -> bool {
        self.status_code().is_server_error()
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code();

        // Log server errors
        if self.is_server_error() {
            tracing::error!(error = %self, code = code, "Server error occurred");
        } else {
            tracing::debug!(error = %self, code = code, "Client error occurred");
        }

        let body = Json(json!({
            "error": {
                "code": code,
                "message": self.to_string(),
            }
        }));

        (status, body).into_response()
    }
}

// === From implementations ===

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        Self::Validation(err.to_string())
    }
}

impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        Self::Config(err.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}
