//! Messaging endpoints for direct messages.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Create messaging router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_conversations))
        .route("/unread/count", get(get_unread_count))
        .route("/history/{user_id}", get(get_conversation))
        .route("/history/{user_id}", post(send_message))
        .route("/history/{user_id}/read", post(mark_as_read))
        .route("/message/{message_id}", delete(delete_message))
}

/// Message response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponse {
    pub id: String,
    pub user_id: String,
    pub recipient_id: Option<String>,
    pub text: Option<String>,
    pub file_id: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

impl From<misskey_db::entities::messaging_message::Model> for MessageResponse {
    fn from(msg: misskey_db::entities::messaging_message::Model) -> Self {
        Self {
            id: msg.id,
            user_id: msg.user_id,
            recipient_id: msg.recipient_id,
            text: msg.text,
            file_id: msg.file_id,
            is_read: msg.is_read,
            created_at: msg.created_at.into(),
        }
    }
}

/// Conversation summary response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationResponse {
    pub partner_id: String,
    pub partner_username: String,
    pub partner_avatar_url: Option<String>,
    pub last_message: Option<MessageResponse>,
    pub unread_count: u64,
}

/// List conversations response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationsListResponse {
    pub conversations: Vec<ConversationResponse>,
}

/// List conversations query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationsQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
}

const fn default_limit() -> u64 {
    20
}

/// List conversations for the authenticated user.
async fn list_conversations(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListConversationsQuery>,
) -> AppResult<ApiResponse<ConversationsListResponse>> {
    let summaries = state
        .messaging_service
        .get_conversations(&user.id, query.limit)
        .await?;

    let conversations: Vec<ConversationResponse> = summaries
        .into_iter()
        .map(|s| ConversationResponse {
            partner_id: s.partner_id,
            partner_username: s.partner_username,
            partner_avatar_url: s.partner_avatar_url,
            last_message: s.last_message.map(MessageResponse::from),
            unread_count: s.unread_count,
        })
        .collect();

    Ok(ApiResponse::ok(ConversationsListResponse { conversations }))
}

/// Get conversation query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConversationQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

/// Message list response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageListResponse {
    pub messages: Vec<MessageResponse>,
}

/// Get messages in a conversation with another user.
async fn get_conversation(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(partner_id): Path<String>,
    Query(query): Query<GetConversationQuery>,
) -> AppResult<ApiResponse<MessageListResponse>> {
    let messages = state
        .messaging_service
        .get_conversation(
            &user.id,
            &partner_id,
            query.limit,
            query.until_id.as_deref(),
        )
        .await?;

    let messages: Vec<MessageResponse> = messages.into_iter().map(MessageResponse::from).collect();

    Ok(ApiResponse::ok(MessageListResponse { messages }))
}

/// Send message request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub text: Option<String>,
    pub file_id: Option<String>,
}

/// Send a message to another user.
async fn send_message(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(recipient_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> AppResult<ApiResponse<MessageResponse>> {
    info!(
        sender = %user.id,
        recipient = %recipient_id,
        "Sending message"
    );

    let input = misskey_core::CreateMessageInput {
        text: req.text,
        file_id: req.file_id,
    };

    let message = state
        .messaging_service
        .send_message(&user.id, &recipient_id, input)
        .await?;

    Ok(ApiResponse::ok(MessageResponse::from(message)))
}

/// Mark messages from a user as read.
async fn mark_as_read(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(partner_id): Path<String>,
) -> AppResult<ApiResponse<MarkAsReadResponse>> {
    info!(
        user = %user.id,
        partner = %partner_id,
        "Marking messages as read"
    );

    let count = state
        .messaging_service
        .mark_as_read(&user.id, &partner_id)
        .await?;

    Ok(ApiResponse::ok(MarkAsReadResponse { read_count: count }))
}

/// Mark as read response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkAsReadResponse {
    pub read_count: u64,
}

/// Delete a message.
async fn delete_message(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(message_id): Path<String>,
) -> AppResult<ApiResponse<()>> {
    info!(
        user = %user.id,
        message = %message_id,
        "Deleting message"
    );

    state
        .messaging_service
        .delete_message(&user.id, &message_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Unread count response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadCountResponse {
    pub count: u64,
}

/// Get unread message count.
async fn get_unread_count(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<UnreadCountResponse>> {
    let count = state.messaging_service.get_unread_count(&user.id).await?;

    Ok(ApiResponse::ok(UnreadCountResponse { count }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_message_response_serialization() {
        let response = MessageResponse {
            id: "123".to_string(),
            user_id: "user1".to_string(),
            recipient_id: Some("user2".to_string()),
            text: Some("Hello!".to_string()),
            file_id: None,
            is_read: false,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"text\":\"Hello!\""));
        assert!(json.contains("\"isRead\":false"));
    }
}
