//! Security keys (WebAuthn/Passkey) endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::{
    BeginAuthenticationResponse, BeginRegistrationResponse, CompleteAuthenticationInput,
    CompleteRegistrationInput, SecurityKeyResponse,
};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Request to rename a security key.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameKeyRequest {
    pub key_id: String,
    pub name: String,
}

/// Request to delete a security key.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteKeyRequest {
    pub key_id: String,
}

/// Response for checking security key status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityKeyStatusResponse {
    pub has_security_keys: bool,
    pub count: u64,
}

/// Begin registration - start the process of adding a new security key.
async fn begin_registration(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<BeginRegistrationResponse>> {
    let response = state
        .webauthn_service
        .begin_registration(&user.id)
        .await?;

    Ok(ApiResponse::ok(response))
}

/// Complete registration - finish adding a new security key.
async fn complete_registration(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CompleteRegistrationInput>,
) -> AppResult<ApiResponse<SecurityKeyResponse>> {
    let response = state
        .webauthn_service
        .complete_registration(&user.id, input)
        .await?;

    Ok(ApiResponse::ok(response))
}

/// Begin authentication - start the process of authenticating with a security key.
async fn begin_authentication(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<BeginAuthenticationResponse>> {
    let response = state
        .webauthn_service
        .begin_authentication(&user.id)
        .await?;

    Ok(ApiResponse::ok(response))
}

/// Complete authentication - finish authenticating with a security key.
async fn complete_authentication(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CompleteAuthenticationInput>,
) -> AppResult<ApiResponse<bool>> {
    let success = state
        .webauthn_service
        .complete_authentication(&user.id, input)
        .await?;

    Ok(ApiResponse::ok(success))
}

/// List all security keys for the current user.
async fn list_keys(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<SecurityKeyResponse>>> {
    let keys = state.webauthn_service.list_keys(&user.id).await?;
    Ok(ApiResponse::ok(keys))
}

/// Rename a security key.
async fn rename_key(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RenameKeyRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .webauthn_service
        .rename_key(&user.id, &req.key_id, &req.name)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Delete a security key.
async fn delete_key(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteKeyRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .webauthn_service
        .delete_key(&user.id, &req.key_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Get security key status for the current user.
async fn status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<SecurityKeyStatusResponse>> {
    let has_keys = state.webauthn_service.has_security_keys(&user.id).await?;
    let keys = state.webauthn_service.list_keys(&user.id).await?;

    Ok(ApiResponse::ok(SecurityKeyStatusResponse {
        has_security_keys: has_keys,
        count: keys.len() as u64,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(begin_registration))
        .route("/register/complete", post(complete_registration))
        .route("/authenticate", post(begin_authentication))
        .route("/authenticate/complete", post(complete_authentication))
        .route("/list", post(list_keys))
        .route("/rename", post(rename_key))
        .route("/delete", post(delete_key))
        .route("/status", post(status))
}
