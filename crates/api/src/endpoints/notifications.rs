//! Notifications endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_db::entities::notification::{Model as NotificationModel, NotificationType};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Notification type filter enum.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationTypeFilter {
    Follow,
    Mention,
    Reply,
    Renote,
    Quote,
    Reaction,
    PollEnded,
    ReceiveFollowRequest,
    FollowRequestAccepted,
    App,
}

impl NotificationTypeFilter {
    const fn to_notification_type(&self) -> NotificationType {
        match self {
            Self::Follow => NotificationType::Follow,
            Self::Mention => NotificationType::Mention,
            Self::Reply => NotificationType::Reply,
            Self::Renote => NotificationType::Renote,
            Self::Quote => NotificationType::Quote,
            Self::Reaction => NotificationType::Reaction,
            Self::PollEnded => NotificationType::PollEnded,
            Self::ReceiveFollowRequest => NotificationType::ReceiveFollowRequest,
            Self::FollowRequestAccepted => NotificationType::FollowRequestAccepted,
            Self::App => NotificationType::App,
        }
    }
}

/// List notifications request with advanced filtering.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationsRequest {
    /// Maximum results (default: 10, max: 100)
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// Cursor for pagination (before this ID)
    pub until_id: Option<String>,
    /// Cursor for forward pagination (after this ID)
    pub since_id: Option<String>,
    /// Only unread notifications
    #[serde(default)]
    pub unread_only: bool,

    // === Advanced filters (上位互換) ===
    /// Include only these notification types
    pub include_types: Option<Vec<NotificationTypeFilter>>,
    /// Exclude these notification types
    pub exclude_types: Option<Vec<NotificationTypeFilter>>,
    /// Include unread count in response metadata
    #[serde(default)]
    pub with_unread_count: bool,
}

const fn default_limit() -> u64 {
    10
}

/// Notifications response with optional metadata.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationsListResponse {
    pub notifications: Vec<NotificationResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread_count: Option<u64>,
}

/// Notification response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationResponse {
    pub id: String,
    pub created_at: String,
    pub is_read: bool,
    #[serde(rename = "type")]
    pub notification_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction: Option<String>,
}

impl From<NotificationModel> for NotificationResponse {
    fn from(n: NotificationModel) -> Self {
        Self {
            id: n.id,
            created_at: n.created_at.to_rfc3339(),
            is_read: n.is_read,
            notification_type: notification_type_to_string(&n.notification_type),
            user_id: n.notifier_id,
            note_id: n.note_id,
            reaction: n.reaction,
        }
    }
}

fn notification_type_to_string(t: &NotificationType) -> String {
    match t {
        NotificationType::Follow => "follow".to_string(),
        NotificationType::Mention => "mention".to_string(),
        NotificationType::Reply => "reply".to_string(),
        NotificationType::Renote => "renote".to_string(),
        NotificationType::Quote => "quote".to_string(),
        NotificationType::Reaction => "reaction".to_string(),
        NotificationType::PollEnded => "pollEnded".to_string(),
        NotificationType::ReceiveFollowRequest => "receiveFollowRequest".to_string(),
        NotificationType::FollowRequestAccepted => "followRequestAccepted".to_string(),
        NotificationType::MessagingMessage => "messagingMessage".to_string(),
        NotificationType::App => "app".to_string(),
    }
}

/// Get notifications for the authenticated user.
async fn get_notifications(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListNotificationsRequest>,
) -> AppResult<ApiResponse<NotificationsListResponse>> {
    let limit = req.limit.min(100);

    // Get notifications with basic filters
    let mut notifications = state
        .notification_service
        .get_notifications(&user.id, limit, req.until_id.as_deref(), req.unread_only)
        .await?;

    // Apply type filters
    if let Some(include_types) = &req.include_types {
        let include_set: Vec<NotificationType> = include_types
            .iter()
            .map(NotificationTypeFilter::to_notification_type)
            .collect();
        notifications.retain(|n| include_set.contains(&n.notification_type));
    }

    if let Some(exclude_types) = &req.exclude_types {
        let exclude_set: Vec<NotificationType> = exclude_types
            .iter()
            .map(NotificationTypeFilter::to_notification_type)
            .collect();
        notifications.retain(|n| !exclude_set.contains(&n.notification_type));
    }

    // Get unread count if requested
    let unread_count = if req.with_unread_count {
        Some(state.notification_service.count_unread(&user.id).await?)
    } else {
        None
    };

    Ok(ApiResponse::ok(NotificationsListResponse {
        notifications: notifications.into_iter().map(Into::into).collect(),
        unread_count,
    }))
}

/// Mark notification as read request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkAsReadRequest {
    pub notification_id: String,
}

/// Mark a notification as read.
async fn mark_as_read(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<MarkAsReadRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .notification_service
        .mark_as_read(&user.id, &req.notification_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Mark all notifications as read.
async fn mark_all_as_read(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<MarkAllAsReadResponse>> {
    let count = state
        .notification_service
        .mark_all_as_read(&user.id)
        .await?;
    Ok(ApiResponse::ok(MarkAllAsReadResponse { count }))
}

/// Mark all as read response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkAllAsReadResponse {
    pub count: u64,
}

/// Unread count response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadCountResponse {
    pub count: u64,
}

/// Get unread notification count.
async fn unread_count(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<UnreadCountResponse>> {
    let count = state.notification_service.count_unread(&user.id).await?;
    Ok(ApiResponse::ok(UnreadCountResponse { count }))
}

/// Delete notification request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNotificationRequest {
    pub notification_id: String,
}

/// Delete a notification.
async fn delete_notification(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteNotificationRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .notification_service
        .delete(&user.id, &req.notification_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Delete all notifications.
async fn delete_all_notifications(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<DeleteAllResponse>> {
    let count = state.notification_service.delete_all(&user.id).await?;
    Ok(ApiResponse::ok(DeleteAllResponse { count }))
}

/// Delete all response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAllResponse {
    pub count: u64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(get_notifications))
        .route("/mark-as-read", post(mark_as_read))
        .route("/mark-all-as-read", post(mark_all_as_read))
        .route("/unread-count", post(unread_count))
        .route("/delete", post(delete_notification))
        .route("/delete-all", post(delete_all_notifications))
}
