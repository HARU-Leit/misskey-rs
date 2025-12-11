//! Authentication endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Signup request.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    #[validate(length(min = 1, max = 100))]
    pub username: String,

    #[validate(length(min = 8, max = 128))]
    pub password: String,

    pub name: Option<String>,
}

/// Signup response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupResponse {
    pub id: String,
    pub username: String,
    pub token: String,
}

/// Create a new user account.
async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> AppResult<ApiResponse<SignupResponse>> {
    req.validate()?;

    let input = misskey_core::user::CreateUserInput {
        username: req.username,
        password: req.password,
        name: req.name,
    };

    let user = state.user_service.create(input).await?;

    Ok(ApiResponse::ok(SignupResponse {
        id: user.id.clone(),
        username: user.username,
        token: user.token.unwrap_or_default(),
    }))
}

/// Signin request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigninRequest {
    pub username: String,
    pub password: String,
    /// Optional 2FA token for users with 2FA enabled.
    pub token: Option<String>,
}

/// Signin response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigninResponse {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// True if 2FA is required but not provided.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub requires_two_factor: bool,
    /// True if `WebAuthn` is available.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub webauthn_available: bool,
    /// Challenge ID for `WebAuthn` (if starting `WebAuthn` flow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webauthn_challenge_id: Option<String>,
    /// `WebAuthn` options (if starting `WebAuthn` flow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webauthn_options: Option<serde_json::Value>,
}

/// Sign in to an existing account.
async fn signin(
    State(state): State<AppState>,
    Json(req): Json<SigninRequest>,
) -> AppResult<ApiResponse<SigninResponse>> {
    // Authenticate with username/password first
    let user = state
        .user_service
        .authenticate(&req.username, &req.password)
        .await?;

    // Check if 2FA is enabled
    let two_factor_enabled = state.two_factor_service.is_enabled(&user.id).await?;
    let has_security_keys = state.webauthn_service.has_security_keys(&user.id).await?;

    // If 2FA is enabled, verify token if provided
    if two_factor_enabled {
        if let Some(ref token) = req.token {
            // Verify 2FA token
            let is_valid = state.two_factor_service.verify(&user.id, token).await?;
            if !is_valid {
                return Err(misskey_common::AppError::Validation(
                    "Invalid two-factor authentication code".to_string(),
                ));
            }
        } else {
            // 2FA required but not provided
            return Ok(ApiResponse::ok(SigninResponse {
                id: user.id.clone(),
                username: user.username,
                token: None,
                requires_two_factor: true,
                webauthn_available: has_security_keys,
                webauthn_challenge_id: None,
                webauthn_options: None,
            }));
        }
    }

    // Successful login
    Ok(ApiResponse::ok(SigninResponse {
        id: user.id.clone(),
        username: user.username,
        token: user.token,
        requires_two_factor: false,
        webauthn_available: false,
        webauthn_challenge_id: None,
        webauthn_options: None,
    }))
}

/// Signin with `WebAuthn` request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigninWebAuthnBeginRequest {
    pub username: String,
    pub password: String,
}

/// Begin `WebAuthn` authentication for signin.
async fn signin_webauthn_begin(
    State(state): State<AppState>,
    Json(req): Json<SigninWebAuthnBeginRequest>,
) -> AppResult<ApiResponse<SigninResponse>> {
    // Authenticate with username/password first
    let user = state
        .user_service
        .authenticate(&req.username, &req.password)
        .await?;

    // Check if user has security keys
    let has_security_keys = state.webauthn_service.has_security_keys(&user.id).await?;
    if !has_security_keys {
        return Err(misskey_common::AppError::Validation(
            "No security keys registered".to_string(),
        ));
    }

    // Begin WebAuthn authentication
    let auth_response = state
        .webauthn_service
        .begin_authentication(&user.id)
        .await?;

    Ok(ApiResponse::ok(SigninResponse {
        id: user.id.clone(),
        username: user.username,
        token: None,
        requires_two_factor: true,
        webauthn_available: true,
        webauthn_challenge_id: Some(auth_response.challenge_id),
        webauthn_options: Some(auth_response.options),
    }))
}

/// Complete `WebAuthn` signin request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigninWebAuthnCompleteRequest {
    pub username: String,
    pub password: String,
    pub challenge_id: String,
    pub credential: serde_json::Value,
}

/// Complete `WebAuthn` authentication for signin.
async fn signin_webauthn_complete(
    State(state): State<AppState>,
    Json(req): Json<SigninWebAuthnCompleteRequest>,
) -> AppResult<ApiResponse<SigninResponse>> {
    // Authenticate with username/password first
    let user = state
        .user_service
        .authenticate(&req.username, &req.password)
        .await?;

    // Complete WebAuthn authentication
    let input = misskey_core::webauthn::CompleteAuthenticationInput {
        challenge_id: req.challenge_id,
        credential: req.credential,
    };

    let is_valid = state
        .webauthn_service
        .complete_authentication(&user.id, input)
        .await?;

    if !is_valid {
        return Err(misskey_common::AppError::Validation(
            "WebAuthn authentication failed".to_string(),
        ));
    }

    Ok(ApiResponse::ok(SigninResponse {
        id: user.id.clone(),
        username: user.username,
        token: user.token,
        requires_two_factor: false,
        webauthn_available: false,
        webauthn_challenge_id: None,
        webauthn_options: None,
    }))
}

/// Regenerate token response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateTokenResponse {
    pub token: String,
}

/// Regenerate the authentication token.
async fn regenerate_token(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<RegenerateTokenResponse>> {
    let new_token = state.user_service.regenerate_token(&user.id).await?;

    Ok(ApiResponse::ok(RegenerateTokenResponse {
        token: new_token,
    }))
}

/// Signout response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignoutResponse {
    pub ok: bool,
}

/// Sign out (invalidate current token by regenerating).
async fn signout(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<SignoutResponse>> {
    // Regenerate token to invalidate the current one
    state.user_service.regenerate_token(&user.id).await?;

    Ok(ApiResponse::ok(SignoutResponse { ok: true }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/signup", post(signup))
        .route("/signin", post(signin))
        .route("/signin/webauthn/begin", post(signin_webauthn_begin))
        .route("/signin/webauthn/complete", post(signin_webauthn_complete))
        .route("/signout", post(signout))
        .route("/regenerate-token", post(regenerate_token))
}
