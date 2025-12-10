//! Custom emoji endpoints.

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    extractors::AuthUser,
    middleware::AppState,
    response::ApiResponse,
};

/// Create emoji router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_emojis))
        .route("/", post(create_emoji))
        .route("/search", get(search_emojis))
        .route("/categories", get(list_categories))
        .route("/{id}", get(get_emoji))
        .route("/{id}", put(update_emoji))
        .route("/{id}", delete(delete_emoji))
}

/// Emoji response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiResponse {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub url: String,
    pub static_url: Option<String>,
    pub aliases: Vec<String>,
    pub is_sensitive: bool,
    pub local_only: bool,
    pub license: Option<String>,
}

impl From<misskey_db::entities::emoji::Model> for EmojiResponse {
    fn from(emoji: misskey_db::entities::emoji::Model) -> Self {
        let aliases: Vec<String> = serde_json::from_value(emoji.aliases).unwrap_or_default();
        Self {
            id: emoji.id,
            name: emoji.name,
            category: emoji.category,
            url: emoji.original_url,
            static_url: emoji.static_url,
            aliases,
            is_sensitive: emoji.is_sensitive,
            local_only: emoji.local_only,
            license: emoji.license,
        }
    }
}

/// List emojis response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiListResponse {
    pub emojis: Vec<EmojiResponse>,
    pub total: u64,
}

/// List emojis query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEmojisQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
    pub category: Option<String>,
}

const fn default_limit() -> u64 {
    100
}

/// List emojis.
async fn list_emojis(
    State(state): State<AppState>,
    Query(query): Query<ListEmojisQuery>,
) -> AppResult<ApiResponse<EmojiListResponse>> {
    let emojis = if let Some(category) = query.category {
        state.emoji_service.list_by_category(&category).await?
    } else {
        state
            .emoji_service
            .list_local_paginated(query.limit, query.offset)
            .await?
    };

    let total = state.emoji_service.count().await?;

    Ok(ApiResponse::ok(EmojiListResponse {
        emojis: emojis.into_iter().map(EmojiResponse::from).collect(),
        total,
    }))
}

/// Search emojis query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEmojisQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Search emojis.
async fn search_emojis(
    State(state): State<AppState>,
    Query(query): Query<SearchEmojisQuery>,
) -> AppResult<ApiResponse<EmojiListResponse>> {
    let emojis = state
        .emoji_service
        .search(&query.q, query.limit, query.offset)
        .await?;

    let count = emojis.len() as u64;

    Ok(ApiResponse::ok(EmojiListResponse {
        emojis: emojis.into_iter().map(EmojiResponse::from).collect(),
        total: count,
    }))
}

/// Categories response.
#[derive(Debug, Serialize)]
pub struct CategoriesResponse {
    pub categories: Vec<String>,
}

/// List categories.
async fn list_categories(
    State(state): State<AppState>,
) -> AppResult<ApiResponse<CategoriesResponse>> {
    let categories = state.emoji_service.list_categories().await?;
    Ok(ApiResponse::ok(CategoriesResponse { categories }))
}

/// Get emoji by ID.
async fn get_emoji(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<ApiResponse<EmojiResponse>> {
    let emoji = state
        .emoji_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| misskey_common::AppError::NotFound(format!("Emoji not found: {id}")))?;

    Ok(ApiResponse::ok(EmojiResponse::from(emoji)))
}

/// Create emoji request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmojiRequest {
    pub name: String,
    pub url: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    pub category: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub is_sensitive: bool,
    #[serde(default)]
    pub local_only: bool,
    pub license: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

fn default_content_type() -> String {
    "image/png".to_string()
}

/// Create emoji (admin only).
async fn create_emoji(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateEmojiRequest>,
) -> AppResult<ApiResponse<EmojiResponse>> {
    // Check if user is admin or moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only admins and moderators can create emojis".to_string(),
        ));
    }

    info!(user_id = %user.id, emoji_name = %req.name, "Creating emoji");

    let emoji = state
        .emoji_service
        .create(
            req.name,
            req.url,
            req.content_type,
            req.category,
            req.aliases,
            req.is_sensitive,
            req.local_only,
            req.license,
            req.width,
            req.height,
        )
        .await?;

    Ok(ApiResponse::ok(EmojiResponse::from(emoji)))
}

/// Update emoji request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEmojiRequest {
    pub name: Option<String>,
    pub category: Option<Option<String>>,
    pub aliases: Option<Vec<String>>,
    pub is_sensitive: Option<bool>,
    pub local_only: Option<bool>,
    pub license: Option<Option<String>>,
}

/// Update emoji (admin only).
async fn update_emoji(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateEmojiRequest>,
) -> AppResult<ApiResponse<EmojiResponse>> {
    // Check if user is admin or moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only admins and moderators can update emojis".to_string(),
        ));
    }

    info!(user_id = %user.id, emoji_id = %id, "Updating emoji");

    let emoji = state
        .emoji_service
        .update(
            &id,
            req.name,
            req.category,
            req.aliases,
            req.is_sensitive,
            req.local_only,
            req.license,
        )
        .await?;

    Ok(ApiResponse::ok(EmojiResponse::from(emoji)))
}

/// Delete emoji (admin only).
async fn delete_emoji(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<ApiResponse<()>> {
    // Check if user is admin or moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only admins and moderators can delete emojis".to_string(),
        ));
    }

    info!(user_id = %user.id, emoji_id = %id, "Deleting emoji");

    state.emoji_service.delete(&id).await?;

    Ok(ApiResponse::ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_response_serialization() {
        let response = EmojiResponse {
            id: "123".to_string(),
            name: "blobcat".to_string(),
            category: Some("Animals".to_string()),
            url: "https://example.com/blobcat.png".to_string(),
            static_url: None,
            aliases: vec!["cat".to_string(), "blob".to_string()],
            is_sensitive: false,
            local_only: false,
            license: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"blobcat\""));
        assert!(json.contains("\"category\":\"Animals\""));
    }
}
