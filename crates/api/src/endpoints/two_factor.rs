//! Two-factor authentication endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::services::two_factor::{
    ConfirmTwoFactorInput, DisableTwoFactorInput, TwoFactorConfirmResponse, TwoFactorSetupResponse,
};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

// ==================== Request/Response Types ====================

/// Password confirmation request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordRequest {
    pub password: String,
}

/// Token verification request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTokenRequest {
    pub token: String,
}

/// 2FA status response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorStatusResponse {
    pub enabled: bool,
    pub has_backup_codes: bool,
}

/// Regenerate backup codes response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateBackupCodesResponse {
    pub backup_codes: Vec<String>,
}

// ==================== Handlers ====================

/// Get 2FA status.
async fn status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<TwoFactorStatusResponse>> {
    let enabled = state.two_factor_service.is_enabled(&user.id).await?;

    Ok(ApiResponse::ok(TwoFactorStatusResponse {
        enabled,
        has_backup_codes: enabled, // If 2FA is enabled, backup codes exist
    }))
}

/// Begin 2FA setup.
async fn begin_setup(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<TwoFactorSetupResponse>> {
    // Get issuer from config or use default
    let issuer = "Misskey";

    let response = state
        .two_factor_service
        .begin_setup(&user.id, &user.username, issuer)
        .await?;

    Ok(ApiResponse::ok(response))
}

/// Confirm 2FA setup.
async fn confirm_setup(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<ConfirmTwoFactorInput>,
) -> AppResult<ApiResponse<TwoFactorConfirmResponse>> {
    let response = state
        .two_factor_service
        .confirm_setup(&user.id, input)
        .await?;

    Ok(ApiResponse::ok(response))
}

/// Cancel pending 2FA setup.
async fn cancel_setup(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<()>> {
    state.two_factor_service.cancel_setup(&user.id).await?;

    Ok(ApiResponse::ok(()))
}

/// Disable 2FA.
async fn disable(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<DisableTwoFactorInput>,
) -> AppResult<ApiResponse<()>> {
    state.two_factor_service.disable(&user.id, input).await?;

    Ok(ApiResponse::ok(()))
}

/// Verify 2FA token (for login).
async fn verify(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<VerifyTokenRequest>,
) -> AppResult<ApiResponse<bool>> {
    let valid = state
        .two_factor_service
        .verify(&user.id, &req.token)
        .await?;

    Ok(ApiResponse::ok(valid))
}

/// Regenerate backup codes.
async fn regenerate_backup_codes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<PasswordRequest>,
) -> AppResult<ApiResponse<RegenerateBackupCodesResponse>> {
    let codes = state
        .two_factor_service
        .regenerate_backup_codes(&user.id, &req.password)
        .await?;

    Ok(ApiResponse::ok(RegenerateBackupCodesResponse {
        backup_codes: codes,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", post(status))
        .route("/register", post(begin_setup))
        .route("/confirm", post(confirm_setup))
        .route("/cancel", post(cancel_setup))
        .route("/disable", post(disable))
        .route("/verify", post(verify))
        .route("/regenerate-backup-codes", post(regenerate_backup_codes))
}
