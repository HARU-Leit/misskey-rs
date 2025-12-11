//! Search endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_db::entities::{note::Model as NoteModel, user::Model as UserModel};
use serde::{Deserialize, Serialize};

use crate::{middleware::AppState, response::ApiResponse};

/// Note search result.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSearchResult {
    pub id: String,
    pub created_at: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cw: Option<String>,
    pub visibility: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renote_id: Option<String>,
    pub replies_count: i32,
    pub renote_count: i32,
    pub reaction_count: i32,
}

impl From<NoteModel> for NoteSearchResult {
    fn from(n: NoteModel) -> Self {
        Self {
            id: n.id,
            created_at: n.created_at.to_rfc3339(),
            user_id: n.user_id,
            text: n.text,
            cw: n.cw,
            visibility: format!("{:?}", n.visibility).to_lowercase(),
            reply_id: n.reply_id,
            renote_id: n.renote_id,
            replies_count: n.replies_count,
            renote_count: n.renote_count,
            reaction_count: n.reaction_count,
        }
    }
}

/// User search result.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchResult {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub followers_count: i32,
    pub following_count: i32,
    pub notes_count: i32,
    pub is_bot: bool,
    pub is_cat: bool,
}

impl From<UserModel> for UserSearchResult {
    fn from(u: UserModel) -> Self {
        Self {
            id: u.id,
            username: u.username,
            host: u.host,
            name: u.name,
            description: u.description,
            avatar_url: u.avatar_url,
            followers_count: u.followers_count,
            following_count: u.following_count,
            notes_count: u.notes_count,
            is_bot: u.is_bot,
            is_cat: u.is_cat,
        }
    }
}

/// Note visibility for filtering.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchVisibility {
    Public,
    Home,
    Followers,
    Specified,
}

/// Search notes request with advanced filters.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchNotesRequest {
    /// Search query text
    pub query: String,
    /// Maximum results to return (default: 10, max: 100)
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// Cursor for pagination (notes before this ID)
    pub until_id: Option<String>,
    /// Cursor for forward pagination (notes after this ID)
    pub since_id: Option<String>,
    /// Filter by specific user
    pub user_id: Option<String>,
    /// Filter by host (empty string = local only)
    pub host: Option<String>,

    // === Advanced filters (上位互換) ===
    /// Filter by visibility types
    pub visibility: Option<Vec<SearchVisibility>>,
    /// Filter notes created after this date
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter notes created before this date
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
    /// Minimum reaction count (for trending)
    pub min_reactions: Option<i32>,
    /// Minimum renote count
    pub min_renotes: Option<i32>,
    /// Only notes with media attachments
    pub has_media: Option<bool>,
    /// Only notes that are replies to this note ID
    pub in_reply_to: Option<String>,
    /// Notes mentioning these user IDs
    pub mentions: Option<Vec<String>>,
    /// Notes with these hashtags
    pub hashtags: Option<Vec<String>>,
}

const fn default_limit() -> u64 {
    10
}

/// Trending notes request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendingNotesRequest {
    /// Maximum results (default: 10, max: 100)
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// Minimum reaction count (default: 5)
    #[serde(default = "default_min_reactions")]
    pub min_reactions: i32,
    /// Time window in hours (default: 24)
    #[serde(default = "default_hours")]
    pub hours: i64,
}

const fn default_min_reactions() -> i32 {
    5
}

const fn default_hours() -> i64 {
    24
}

/// Search notes.
async fn search_notes(
    State(state): State<AppState>,
    Json(req): Json<SearchNotesRequest>,
) -> AppResult<ApiResponse<Vec<NoteSearchResult>>> {
    let query = req.query.trim();
    if query.is_empty() {
        return Ok(ApiResponse::ok(vec![]));
    }
    if query.len() < 2 {
        return Err(misskey_common::AppError::BadRequest(
            "Search query must be at least 2 characters".to_string(),
        ));
    }

    let limit = req.limit.min(100);
    let notes = state
        .note_service
        .search_notes(
            query,
            limit,
            req.until_id.as_deref(),
            req.user_id.as_deref(),
            req.host.as_deref(),
        )
        .await?;

    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

/// Search by hashtag request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchByTagRequest {
    pub tag: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

/// Search notes by hashtag.
async fn search_by_tag(
    State(state): State<AppState>,
    Json(req): Json<SearchByTagRequest>,
) -> AppResult<ApiResponse<Vec<NoteSearchResult>>> {
    let tag = req.tag.trim().trim_start_matches('#');
    if tag.is_empty() {
        return Ok(ApiResponse::ok(vec![]));
    }

    let limit = req.limit.min(100);
    let notes = state
        .note_service
        .search_by_tag(tag, limit, req.until_id.as_deref())
        .await?;

    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

/// Search users request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchUsersRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
    #[serde(default)]
    pub local_only: bool,
}

/// Search users.
async fn search_users(
    State(state): State<AppState>,
    Json(req): Json<SearchUsersRequest>,
) -> AppResult<ApiResponse<Vec<UserSearchResult>>> {
    let query = req.query.trim();
    if query.is_empty() {
        return Ok(ApiResponse::ok(vec![]));
    }
    if query.is_empty() {
        return Err(misskey_common::AppError::BadRequest(
            "Search query is required".to_string(),
        ));
    }

    let limit = req.limit.min(100);
    let users = state
        .user_service
        .search_users(query, limit, req.offset, req.local_only)
        .await?;

    Ok(ApiResponse::ok(users.into_iter().map(Into::into).collect()))
}

/// Get trending notes.
async fn trending_notes(
    State(state): State<AppState>,
    Json(req): Json<TrendingNotesRequest>,
) -> AppResult<ApiResponse<Vec<NoteSearchResult>>> {
    let limit = req.limit.min(100);
    let min_reactions = req.min_reactions.max(1);
    let hours = req.hours.clamp(1, 168); // 1 hour to 1 week

    let notes = state
        .note_service
        .find_trending(limit, min_reactions, hours)
        .await?;

    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notes", post(search_notes))
        .route("/notes/by-tag", post(search_by_tag))
        .route("/notes/trending", post(trending_notes))
        .route("/users", post(search_users))
}
