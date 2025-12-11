//! Error types for misskey-rs.
//!
//! This module provides a unified error type for the entire application,
//! with automatic conversion from common error types using the `#[from]` attribute.
//!
//! # Examples
//!
//! ```
//! use misskey_common::error::{AppError, AppResult};
//!
//! fn example_function() -> AppResult<()> {
//!     // Validation errors are automatically converted
//!     // Config errors are automatically converted
//!     // Database errors are automatically converted
//!     Ok(())
//! }
//! ```

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// Application result type.
pub type AppResult<T> = Result<T, AppError>;

/// Application error type.
///
/// This enum represents all possible errors in the application, categorized into
/// client errors (4xx) and server errors (5xx). It implements automatic conversion
/// from common error types using the `#[from]` attribute.
#[derive(Debug, Error)]
pub enum AppError {
    // === Client Errors (4xx) ===
    /// Generic not found error.
    #[error("Not found: {0}")]
    NotFound(String),

    /// User not found error.
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Note not found error.
    #[error("Note not found: {0}")]
    NoteNotFound(String),

    /// Authentication required.
    #[error("Unauthorized")]
    Unauthorized,

    /// Permission denied.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Invalid request.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Validation failed with structured errors.
    #[error("Validation error: {0}")]
    ValidationErrors(#[from] validator::ValidationErrors),

    /// Validation failed with a message.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Resource conflict.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Rate limit exceeded.
    #[error("Rate limited")]
    RateLimited,

    // === Server Errors (5xx) ===
    /// Database operation failed.
    #[error("Database error: {0}")]
    Database(String),

    /// Redis operation failed.
    #[error("Redis error: {0}")]
    Redis(String),

    /// Federation/ActivityPub error.
    #[error("Federation error: {0}")]
    Federation(String),

    /// Job queue error.
    #[error("Queue error: {0}")]
    Queue(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// External service error.
    #[error("External service error: {0}")]
    ExternalService(String),

    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Returns the HTTP status code for this error.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 4xx Client Errors
            Self::NotFound(_) | Self::UserNotFound(_) | Self::NoteNotFound(_) => {
                StatusCode::NOT_FOUND
            }
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::BadRequest(_) | Self::ValidationErrors(_) | Self::Validation(_) => {
                StatusCode::BAD_REQUEST
            }
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
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "NOT_FOUND",
            Self::UserNotFound(_) => "USER_NOT_FOUND",
            Self::NoteNotFound(_) => "NOTE_NOT_FOUND",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::ValidationErrors(_) | Self::Validation(_) => "VALIDATION_ERROR",
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

// === Additional From implementations ===
// Note: ValidationErrors and ConfigError are handled by #[from] attribute above

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}
