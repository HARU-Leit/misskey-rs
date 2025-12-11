//! Users endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::{AppError, AppResult};
use misskey_core::UpdateUserInput;
use misskey_db::entities::user;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// User response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: String,
    pub created_at: String,
    pub username: String,
    pub host: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub is_bot: bool,
    pub is_cat: bool,
    pub is_locked: bool,
    pub followers_count: i32,
    pub following_count: i32,
    pub notes_count: i32,
}

impl From<user::Model> for UserResponse {
    fn from(user: user::Model) -> Self {
        Self {
            id: user.id,
            created_at: user.created_at.to_rfc3339(),
            username: user.username,
            host: user.host,
            name: user.name,
            description: user.description,
            avatar_url: user.avatar_url,
            banner_url: user.banner_url,
            is_bot: user.is_bot,
            is_cat: user.is_cat,
            is_locked: user.is_locked,
            followers_count: user.followers_count,
            following_count: user.following_count,
            notes_count: user.notes_count,
        }
    }
}

/// Get current user.
async fn me(AuthUser(user): AuthUser) -> ApiResponse<UserResponse> {
    ApiResponse::ok(user.into())
}

/// Show user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowUserRequest {
    #[serde(alias = "userId")]
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub host: Option<String>,
}

/// Get a user by ID or username.
async fn show(
    State(state): State<AppState>,
    Json(req): Json<ShowUserRequest>,
) -> AppResult<ApiResponse<UserResponse>> {
    let user = if let Some(user_id) = req.user_id {
        state.user_service.get(&user_id).await?
    } else if let Some(username) = req.username {
        state
            .user_service
            .get_by_username(&username, req.host.as_deref())
            .await?
    } else {
        return Err(AppError::BadRequest(
            "Either userId or username is required".to_string(),
        ));
    };

    Ok(ApiResponse::ok(user.into()))
}

/// Update user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar_id: Option<String>,
    pub banner_id: Option<String>,
    pub is_bot: Option<bool>,
    pub is_cat: Option<bool>,
    pub is_locked: Option<bool>,
    /// User pronouns (e.g., "they/them", "she/her", "he/him")
    pub pronouns: Option<String>,
}

impl UpdateUserRequest {
    /// Convert request to input, optionally resolving file IDs to URLs.
    pub fn into_input(self, avatar_url: Option<String>, banner_url: Option<String>) -> UpdateUserInput {
        UpdateUserInput {
            name: self.name,
            description: self.description,
            avatar_id: self.avatar_id,
            banner_id: self.banner_id,
            avatar_url,
            banner_url,
            is_bot: self.is_bot,
            is_cat: self.is_cat,
            is_locked: self.is_locked,
            pronouns: self.pronouns,
        }
    }
}

/// Update current user.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateUserRequest>,
) -> AppResult<ApiResponse<UserResponse>> {
    // Resolve avatar file ID to URL if provided
    let avatar_url = if let Some(ref avatar_id) = req.avatar_id {
        let file = state.drive_service.get_file(avatar_id).await?;
        // Verify the file belongs to the user
        if file.user_id != user.id {
            return Err(AppError::Forbidden("Avatar file does not belong to you".to_string()));
        }
        Some(file.url)
    } else {
        None
    };

    // Resolve banner file ID to URL if provided
    let banner_url = if let Some(ref banner_id) = req.banner_id {
        let file = state.drive_service.get_file(banner_id).await?;
        // Verify the file belongs to the user
        if file.user_id != user.id {
            return Err(AppError::Forbidden("Banner file does not belong to you".to_string()));
        }
        Some(file.url)
    } else {
        None
    };

    let updated_user = state.user_service.update(&user.id, req.into_input(avatar_url, banner_url)).await?;
    Ok(ApiResponse::ok(updated_user.into()))
}

// ==================== Pin Note Endpoints ====================

/// Pin note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinNoteRequest {
    pub note_id: String,
}

/// Unpin note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpinNoteRequest {
    pub note_id: String,
}

/// Reorder pinned notes request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderPinnedNotesRequest {
    pub note_ids: Vec<String>,
}

/// Pinned notes response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedNotesResponse {
    pub pinned_note_ids: Vec<String>,
}

/// Pin a note to the user's profile.
async fn pin_note(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<PinNoteRequest>,
) -> AppResult<ApiResponse<PinnedNotesResponse>> {
    let pinned_note_ids = state.user_service.pin_note(&user.id, &req.note_id).await?;
    Ok(ApiResponse::ok(PinnedNotesResponse { pinned_note_ids }))
}

/// Unpin a note from the user's profile.
async fn unpin_note(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UnpinNoteRequest>,
) -> AppResult<ApiResponse<PinnedNotesResponse>> {
    let pinned_note_ids = state.user_service.unpin_note(&user.id, &req.note_id).await?;
    Ok(ApiResponse::ok(PinnedNotesResponse { pinned_note_ids }))
}

/// Get pinned note IDs for the current user.
async fn get_pinned_notes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<PinnedNotesResponse>> {
    let pinned_note_ids = state.user_service.get_pinned_note_ids(&user.id).await?;
    Ok(ApiResponse::ok(PinnedNotesResponse { pinned_note_ids }))
}

/// Reorder pinned notes.
async fn reorder_pinned_notes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ReorderPinnedNotesRequest>,
) -> AppResult<ApiResponse<()>> {
    state.user_service.reorder_pinned_notes(&user.id, req.note_ids).await?;
    Ok(ApiResponse::ok(()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", post(me))
        .route("/show", post(show))
        .route("/update", post(update))
        .route("/pin", post(pin_note))
        .route("/unpin", post(unpin_note))
        .route("/pinned-notes", post(get_pinned_notes))
        .route("/reorder-pinned-notes", post(reorder_pinned_notes))
}
