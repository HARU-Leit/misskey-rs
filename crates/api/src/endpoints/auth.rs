//! Authentication endpoints.

use axum::{extract::State, routing::post, Json, Router};
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
}

/// Signin response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigninResponse {
    pub id: String,
    pub username: String,
    pub token: String,
}

/// Sign in to an existing account.
async fn signin(
    State(state): State<AppState>,
    Json(req): Json<SigninRequest>,
) -> AppResult<ApiResponse<SigninResponse>> {
    let user = state
        .user_service
        .authenticate(&req.username, &req.password)
        .await?;

    Ok(ApiResponse::ok(SigninResponse {
        id: user.id.clone(),
        username: user.username,
        token: user.token.unwrap_or_default(),
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

    Ok(ApiResponse::ok(RegenerateTokenResponse { token: new_token }))
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
        .route("/signout", post(signout))
        .route("/regenerate-token", post(regenerate_token))
}
