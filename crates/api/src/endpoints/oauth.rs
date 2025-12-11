//! OAuth 2.0 endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::{
    AuthorizeInput, AuthorizeResponse, AuthorizedAppResponse, CreateAppInput, OAuthAppResponse,
    OAuthAppWithSecretResponse, TokenExchangeInput, TokenResponse, UpdateAppInput,
};
use serde::Deserialize;

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Request to get an app by client ID.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAppRequest {
    pub client_id: String,
}

/// Request to update an app.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppRequest {
    pub app_id: String,
    #[serde(flatten)]
    pub input: UpdateAppInput,
}

/// Request to delete an app.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAppRequest {
    pub app_id: String,
}

/// Request to revoke authorization.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeAuthorizationRequest {
    pub app_id: String,
}

/// Request to revoke a token.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeTokenRequest {
    pub token: String,
}

// ==================== Application Management ====================

/// Create a new OAuth application.
async fn create_app(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateAppInput>,
) -> AppResult<ApiResponse<OAuthAppWithSecretResponse>> {
    let app = state.oauth_service.create_app(&user.id, input).await?;
    Ok(ApiResponse::ok(app))
}

/// Get an application by client ID (public info).
async fn get_app(
    State(state): State<AppState>,
    Json(req): Json<GetAppRequest>,
) -> AppResult<ApiResponse<OAuthAppResponse>> {
    let app = state
        .oauth_service
        .get_app_by_client_id(&req.client_id)
        .await?;
    Ok(ApiResponse::ok(app))
}

/// List applications created by the current user.
async fn list_my_apps(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<OAuthAppResponse>>> {
    let apps = state.oauth_service.list_apps_by_user(&user.id).await?;
    Ok(ApiResponse::ok(apps))
}

/// Update an OAuth application.
async fn update_app(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateAppRequest>,
) -> AppResult<ApiResponse<OAuthAppResponse>> {
    let app = state
        .oauth_service
        .update_app(&user.id, &req.app_id, req.input)
        .await?;
    Ok(ApiResponse::ok(app))
}

/// Delete an OAuth application.
async fn delete_app(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteAppRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .oauth_service
        .delete_app(&user.id, &req.app_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

// ==================== Authorization ====================

/// Authorize an application (generate authorization code).
async fn authorize(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<AuthorizeInput>,
) -> AppResult<ApiResponse<AuthorizeResponse>> {
    let response = state.oauth_service.authorize(&user.id, input).await?;
    Ok(ApiResponse::ok(response))
}

/// Exchange authorization code or refresh token for access token.
async fn token(
    State(state): State<AppState>,
    Json(input): Json<TokenExchangeInput>,
) -> AppResult<ApiResponse<TokenResponse>> {
    let response = state.oauth_service.exchange_token(input).await?;
    Ok(ApiResponse::ok(response))
}

/// Revoke a token.
async fn revoke_token(
    State(state): State<AppState>,
    Json(req): Json<RevokeTokenRequest>,
) -> AppResult<ApiResponse<()>> {
    state.oauth_service.revoke_token(&req.token).await?;
    Ok(ApiResponse::ok(()))
}

// ==================== User Management ====================

/// List applications authorized by the current user.
async fn list_authorized_apps(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<AuthorizedAppResponse>>> {
    let apps = state.oauth_service.list_authorized_apps(&user.id).await?;
    Ok(ApiResponse::ok(apps))
}

/// Revoke authorization for an application.
async fn revoke_authorization(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RevokeAuthorizationRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .oauth_service
        .revoke_app_authorization(&user.id, &req.app_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Application management
        .route("/apps/create", post(create_app))
        .route("/apps/show", post(get_app))
        .route("/apps/list", post(list_my_apps))
        .route("/apps/update", post(update_app))
        .route("/apps/delete", post(delete_app))
        // Authorization flow
        .route("/authorize", post(authorize))
        .route("/token", post(token))
        .route("/revoke", post(revoke_token))
        // User management
        .route("/authorized-apps", post(list_authorized_apps))
        .route("/revoke-authorization", post(revoke_authorization))
}
