//! API response types.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Standard API response wrapper.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

/// API error response.
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a success response.
    pub const fn ok(data: T) -> Self {
        Self {
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response.
    pub fn err(code: impl Into<String>, message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                id: None,
            }),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = if self.error.is_some() {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::OK
        };
        (status, Json(self)).into_response()
    }
}

/// Empty success response.
#[must_use] 
pub fn ok() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
