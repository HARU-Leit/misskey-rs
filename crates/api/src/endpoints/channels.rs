//! Channel endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::services::channel::{CreateChannelInput, UpdateChannelInput};
use misskey_db::entities::channel;
use serde::{Deserialize, Serialize};

use crate::{
    endpoints::notes::NoteResponse, extractors::AuthUser, middleware::AppState,
    response::ApiResponse,
};

// ==================== Request/Response Types ====================

/// Channel response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelResponse {
    pub id: String,
    pub created_at: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub banner_id: Option<String>,
    pub color: Option<String>,
    pub is_archived: bool,
    pub is_searchable: bool,
    pub allow_anyone_to_post: bool,
    pub notes_count: i64,
    pub users_count: i64,
    pub last_noted_at: Option<String>,
    pub is_following: Option<bool>,
}

impl From<channel::Model> for ChannelResponse {
    fn from(c: channel::Model) -> Self {
        Self {
            id: c.id,
            created_at: c.created_at.to_rfc3339(),
            user_id: c.user_id,
            name: c.name,
            description: c.description,
            banner_id: c.banner_id,
            color: c.color,
            is_archived: c.is_archived,
            is_searchable: c.is_searchable,
            allow_anyone_to_post: c.allow_anyone_to_post,
            notes_count: c.notes_count,
            users_count: c.users_count,
            last_noted_at: c.last_noted_at.map(|dt| dt.to_rfc3339()),
            is_following: None,
        }
    }
}

/// Show channel request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowChannelRequest {
    pub channel_id: String,
}

/// Delete/Archive channel request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteChannelRequest {
    pub channel_id: String,
}

/// List channels request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListChannelsRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Search channels request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchChannelsRequest {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Follow/Unfollow request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowChannelRequest {
    pub channel_id: String,
}

/// Channel timeline request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelTimelineRequest {
    pub channel_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
    pub since_id: Option<String>,
}

const fn default_limit() -> u64 {
    10
}

// ==================== Handlers ====================

/// Create a new channel.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateChannelInput>,
) -> AppResult<ApiResponse<ChannelResponse>> {
    let channel = state.channel_service.create(&user.id, input).await?;

    Ok(ApiResponse::ok(channel.into()))
}

/// Update a channel.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<UpdateChannelInput>,
) -> AppResult<ApiResponse<ChannelResponse>> {
    let channel = state.channel_service.update(&user.id, input).await?;

    Ok(ApiResponse::ok(channel.into()))
}

/// Archive a channel.
async fn archive(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteChannelRequest>,
) -> AppResult<ApiResponse<ChannelResponse>> {
    let channel = state
        .channel_service
        .archive(&req.channel_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(channel.into()))
}

/// Show a channel.
async fn show(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowChannelRequest>,
) -> AppResult<ApiResponse<ChannelResponse>> {
    let channel = state
        .channel_service
        .get_by_id(&req.channel_id)
        .await?
        .ok_or_else(|| misskey_common::AppError::NotFound("Channel not found".to_string()))?;

    let is_following = state
        .channel_service
        .is_following(&user.id, &req.channel_id)
        .await?;

    let mut response: ChannelResponse = channel.into();
    response.is_following = Some(is_following);

    Ok(ApiResponse::ok(response))
}

/// List owned channels.
async fn owned(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListChannelsRequest>,
) -> AppResult<ApiResponse<Vec<ChannelResponse>>> {
    let limit = req.limit.min(100);
    let channels = state
        .channel_service
        .list_owned(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        channels.into_iter().map(Into::into).collect(),
    ))
}

/// List followed channels.
async fn followed(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListChannelsRequest>,
) -> AppResult<ApiResponse<Vec<ChannelResponse>>> {
    let limit = req.limit.min(100);
    let channels = state
        .channel_service
        .list_followed(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        channels.into_iter().map(Into::into).collect(),
    ))
}

/// List featured channels.
async fn featured(
    State(state): State<AppState>,
    Json(req): Json<ListChannelsRequest>,
) -> AppResult<ApiResponse<Vec<ChannelResponse>>> {
    let limit = req.limit.min(100);
    let channels = state
        .channel_service
        .list_featured(limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        channels.into_iter().map(Into::into).collect(),
    ))
}

/// Search channels.
async fn search(
    State(state): State<AppState>,
    Json(req): Json<SearchChannelsRequest>,
) -> AppResult<ApiResponse<Vec<ChannelResponse>>> {
    let limit = req.limit.min(100);
    let channels = state
        .channel_service
        .search(&req.query, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        channels.into_iter().map(Into::into).collect(),
    ))
}

/// Follow a channel.
async fn follow(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FollowChannelRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .channel_service
        .follow(&user.id, &req.channel_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Unfollow a channel.
async fn unfollow(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FollowChannelRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .channel_service
        .unfollow(&user.id, &req.channel_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Get channel timeline.
async fn timeline(
    State(state): State<AppState>,
    Json(req): Json<ChannelTimelineRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(100);

    // Verify channel exists
    let _channel = state
        .channel_service
        .get_by_id(&req.channel_id)
        .await?
        .ok_or_else(|| misskey_common::AppError::NotFound("Channel not found".to_string()))?;

    // Archived channels can still be viewed
    let notes = state
        .note_service
        .channel_timeline(
            &req.channel_id,
            limit,
            req.until_id.as_deref(),
            req.since_id.as_deref(),
        )
        .await?;

    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/update", post(update))
        .route("/delete", post(archive))
        .route("/show", post(show))
        .route("/owned", post(owned))
        .route("/followed", post(followed))
        .route("/featured", post(featured))
        .route("/search", post(search))
        .route("/follow", post(follow))
        .route("/unfollow", post(unfollow))
        .route("/timeline", post(timeline))
}
