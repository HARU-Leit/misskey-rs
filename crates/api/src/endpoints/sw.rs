//! Service Worker / Push notification endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::{
    CreateSubscriptionInput, PushConfigResponse, PushSubscriptionResponse, UpdateSubscriptionInput,
};
use serde::Deserialize;

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Request to update a push subscription.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionRequest {
    /// Subscription ID
    pub subscription_id: String,
    #[serde(flatten)]
    pub input: UpdateSubscriptionInput,
}

/// Request to unregister a push subscription.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnregisterRequest {
    /// Subscription ID (optional, use endpoint if not provided)
    pub subscription_id: Option<String>,
    /// Endpoint URL (optional, use `subscription_id` if not provided)
    pub endpoint: Option<String>,
}

/// Request to get a subscription by ID.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSubscriptionRequest {
    /// Subscription ID
    pub subscription_id: String,
}

/// Register a new push subscription.
async fn register(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<CreateSubscriptionInput>,
) -> AppResult<ApiResponse<PushSubscriptionResponse>> {
    let push_service = state.push_notification_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Push notifications not configured".to_string())
    })?;

    // Extract user agent from headers
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let subscription = push_service.register(&user.id, input, user_agent).await?;

    Ok(ApiResponse::ok(subscription))
}

/// Update a push subscription.
async fn update_subscription(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateSubscriptionRequest>,
) -> AppResult<ApiResponse<PushSubscriptionResponse>> {
    let push_service = state.push_notification_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Push notifications not configured".to_string())
    })?;

    let subscription = push_service
        .update(&user.id, &req.subscription_id, req.input)
        .await?;

    Ok(ApiResponse::ok(subscription))
}

/// Unregister a push subscription.
async fn unregister(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UnregisterRequest>,
) -> AppResult<ApiResponse<()>> {
    let push_service = state.push_notification_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Push notifications not configured".to_string())
    })?;

    if let Some(subscription_id) = req.subscription_id {
        push_service.unregister(&user.id, &subscription_id).await?;
    } else if let Some(endpoint) = req.endpoint {
        push_service
            .unregister_by_endpoint(&user.id, &endpoint)
            .await?;
    } else {
        return Err(misskey_common::AppError::Validation(
            "Either subscription_id or endpoint must be provided".to_string(),
        ));
    }

    Ok(ApiResponse::ok(()))
}

/// List all push subscriptions.
async fn list_subscriptions(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<PushSubscriptionResponse>>> {
    let push_service = state.push_notification_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Push notifications not configured".to_string())
    })?;

    let subscriptions = push_service.list(&user.id).await?;
    Ok(ApiResponse::ok(subscriptions))
}

/// Get a push subscription by ID.
async fn get_subscription(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetSubscriptionRequest>,
) -> AppResult<ApiResponse<PushSubscriptionResponse>> {
    let push_service = state.push_notification_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Push notifications not configured".to_string())
    })?;

    let subscription = push_service.get(&user.id, &req.subscription_id).await?;
    Ok(ApiResponse::ok(subscription))
}

/// Get push notification configuration (public key, etc).
async fn get_config(State(state): State<AppState>) -> AppResult<ApiResponse<PushConfigResponse>> {
    let response = if let Some(push_service) = &state.push_notification_service {
        PushConfigResponse {
            available: push_service.is_enabled(),
            public_key: push_service.public_key().map(String::from),
        }
    } else {
        PushConfigResponse {
            available: false,
            public_key: None,
        }
    };
    Ok(ApiResponse::ok(response))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/update", post(update_subscription))
        .route("/unregister", post(unregister))
        .route("/list", post(list_subscriptions))
        .route("/show", post(get_subscription))
        .route("/config", post(get_config))
}
