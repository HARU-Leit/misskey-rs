//! Webhook endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::{
    CreateWebhookInput, UpdateWebhookInput, WebhookResponse, WebhookWithSecretResponse,
};
use serde::Deserialize;

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Request to get a webhook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWebhookRequest {
    pub webhook_id: String,
}

/// Request to update a webhook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookRequest {
    pub webhook_id: String,
    #[serde(flatten)]
    pub input: UpdateWebhookInput,
}

/// Request to delete a webhook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWebhookRequest {
    pub webhook_id: String,
}

/// Request to regenerate a webhook secret.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateSecretRequest {
    pub webhook_id: String,
}

/// Request to test a webhook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWebhookRequest {
    pub webhook_id: String,
}

/// Test webhook response.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWebhookResponse {
    pub success: bool,
}

/// Create a new webhook.
async fn create_webhook(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateWebhookInput>,
) -> AppResult<ApiResponse<WebhookWithSecretResponse>> {
    let webhook = state.webhook_service.create(&user.id, input).await?;
    Ok(ApiResponse::ok(webhook))
}

/// List webhooks for the current user.
async fn list_webhooks(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<WebhookResponse>>> {
    let webhooks = state.webhook_service.list(&user.id).await?;
    Ok(ApiResponse::ok(webhooks))
}

/// Get a webhook by ID.
async fn get_webhook(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetWebhookRequest>,
) -> AppResult<ApiResponse<WebhookResponse>> {
    let webhook = state.webhook_service.get(&user.id, &req.webhook_id).await?;
    Ok(ApiResponse::ok(webhook))
}

/// Update a webhook.
async fn update_webhook(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateWebhookRequest>,
) -> AppResult<ApiResponse<WebhookResponse>> {
    let webhook = state
        .webhook_service
        .update(&user.id, &req.webhook_id, req.input)
        .await?;
    Ok(ApiResponse::ok(webhook))
}

/// Delete a webhook.
async fn delete_webhook(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteWebhookRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .webhook_service
        .delete(&user.id, &req.webhook_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Regenerate the secret for a webhook.
async fn regenerate_secret(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RegenerateSecretRequest>,
) -> AppResult<ApiResponse<WebhookWithSecretResponse>> {
    let webhook = state
        .webhook_service
        .regenerate_secret(&user.id, &req.webhook_id)
        .await?;
    Ok(ApiResponse::ok(webhook))
}

/// Test a webhook.
async fn test_webhook(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<TestWebhookRequest>,
) -> AppResult<ApiResponse<TestWebhookResponse>> {
    let success = state
        .webhook_service
        .test(&user.id, &req.webhook_id)
        .await?;
    Ok(ApiResponse::ok(TestWebhookResponse { success }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create_webhook))
        .route("/list", post(list_webhooks))
        .route("/show", post(get_webhook))
        .route("/update", post(update_webhook))
        .route("/delete", post(delete_webhook))
        .route("/regenerate-secret", post(regenerate_secret))
        .route("/test", post(test_webhook))
}
