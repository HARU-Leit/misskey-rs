//! Hashtag endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};

use crate::{middleware::AppState, response::ApiResponse};

/// Hashtag response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashtagResponse {
    pub tag: String,
    pub mentioned_users_count: i32,
    pub mentioned_local_users_count: i32,
    pub mentioned_remote_users_count: i32,
    pub attached_users_count: i32,
    pub attached_local_users_count: i32,
    pub attached_remote_users_count: i32,
    pub is_trending: bool,
}

/// Trending hashtags request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendingRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
}

const fn default_limit() -> u64 {
    10
}

/// Get trending hashtags.
async fn trending(
    State(state): State<AppState>,
    Json(req): Json<TrendingRequest>,
) -> AppResult<ApiResponse<Vec<HashtagResponse>>> {
    let limit = req.limit.min(100);
    let tags = state.hashtag_service.get_trending(limit).await?;
    Ok(ApiResponse::ok(
        tags.into_iter()
            .map(|t| HashtagResponse {
                tag: t.name,
                mentioned_users_count: t.users_count,
                mentioned_local_users_count: t.local_notes_count,
                mentioned_remote_users_count: t.remote_notes_count,
                attached_users_count: t.users_count,
                attached_local_users_count: t.local_notes_count,
                attached_remote_users_count: t.remote_notes_count,
                is_trending: t.is_trending,
            })
            .collect(),
    ))
}

/// Search hashtags request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

/// Search hashtags.
async fn search(
    State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> AppResult<ApiResponse<Vec<String>>> {
    let limit = req.limit.min(100);
    let tags = state.hashtag_service.search(&req.query, limit).await?;
    Ok(ApiResponse::ok(tags.into_iter().map(|t| t.name).collect()))
}

/// Show hashtag request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowRequest {
    pub tag: String,
}

/// Get hashtag details.
async fn show(
    State(state): State<AppState>,
    Json(req): Json<ShowRequest>,
) -> AppResult<ApiResponse<HashtagResponse>> {
    let tag = state.hashtag_service.get(&req.tag).await?;
    Ok(ApiResponse::ok(HashtagResponse {
        tag: tag.name,
        mentioned_users_count: tag.users_count,
        mentioned_local_users_count: tag.local_notes_count,
        mentioned_remote_users_count: tag.remote_notes_count,
        attached_users_count: tag.users_count,
        attached_local_users_count: tag.local_notes_count,
        attached_remote_users_count: tag.remote_notes_count,
        is_trending: tag.is_trending,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/trending", post(trending))
        .route("/search", post(search))
        .route("/show", post(show))
}
