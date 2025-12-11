//! Clips endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_db::entities::{clip, note};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

// ==================== Request/Response Types ====================

/// Clip response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipResponse {
    pub id: String,
    pub created_at: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub notes_count: i32,
}

impl From<clip::Model> for ClipResponse {
    fn from(c: clip::Model) -> Self {
        Self {
            id: c.id,
            created_at: c.created_at.to_rfc3339(),
            user_id: c.user_id,
            name: c.name,
            description: c.description,
            is_public: c.is_public,
            notes_count: c.notes_count,
        }
    }
}

/// Clip note response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipNoteResponse {
    pub id: String,
    pub created_at: String,
    pub note_id: String,
    pub comment: Option<String>,
    pub note: Option<NoteResponse>,
}

/// Simple note response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteResponse {
    pub id: String,
    pub created_at: String,
    pub user_id: String,
    pub text: Option<String>,
    pub cw: Option<String>,
    pub visibility: String,
}

impl From<note::Model> for NoteResponse {
    fn from(n: note::Model) -> Self {
        Self {
            id: n.id,
            created_at: n.created_at.to_rfc3339(),
            user_id: n.user_id,
            text: n.text,
            cw: n.cw,
            visibility: format!("{:?}", n.visibility).to_lowercase(),
        }
    }
}

/// Create clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClipRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub is_public: bool,
}

/// Update clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClipRequest {
    pub clip_id: String,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub is_public: Option<bool>,
}

/// Delete clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteClipRequest {
    pub clip_id: String,
}

/// Show clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowClipRequest {
    pub clip_id: String,
}

/// List clips request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListClipsRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// List user clips request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUserClipsRequest {
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Add note to clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddNoteRequest {
    pub clip_id: String,
    pub note_id: String,
    pub comment: Option<String>,
}

/// Remove note from clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveNoteRequest {
    pub clip_id: String,
    pub note_id: String,
}

/// List notes in clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotesRequest {
    pub clip_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Reorder clips request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderClipsRequest {
    pub clip_ids: Vec<String>,
}

/// Reorder notes request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderNotesRequest {
    pub clip_id: String,
    pub clip_note_ids: Vec<String>,
}

/// Find note in clips request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindNoteInClipsRequest {
    pub note_id: String,
}

/// Search notes in clip request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchNotesRequest {
    pub clip_id: String,
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

const fn default_limit() -> u64 {
    10
}

// ==================== Handlers ====================

/// Create a new clip.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateClipRequest>,
) -> AppResult<ApiResponse<ClipResponse>> {
    let clip = state
        .clip_service
        .create(&user.id, req.name, req.description, req.is_public)
        .await?;

    Ok(ApiResponse::ok(clip.into()))
}

/// Update a clip.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateClipRequest>,
) -> AppResult<ApiResponse<ClipResponse>> {
    let clip = state
        .clip_service
        .update(
            &req.clip_id,
            &user.id,
            req.name,
            req.description,
            req.is_public,
        )
        .await?;

    Ok(ApiResponse::ok(clip.into()))
}

/// Delete a clip.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteClipRequest>,
) -> AppResult<ApiResponse<()>> {
    state.clip_service.delete(&req.clip_id, &user.id).await?;

    Ok(ApiResponse::ok(()))
}

/// Show a clip.
async fn show(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowClipRequest>,
) -> AppResult<ApiResponse<ClipResponse>> {
    let clip = state
        .clip_service
        .get_by_id_with_access(&req.clip_id, Some(&user.id))
        .await?
        .ok_or_else(|| misskey_common::AppError::NotFound("Clip not found".to_string()))?;

    Ok(ApiResponse::ok(clip.into()))
}

/// List my clips.
async fn list(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListClipsRequest>,
) -> AppResult<ApiResponse<Vec<ClipResponse>>> {
    let limit = req.limit.min(100);
    let clips = state
        .clip_service
        .list_my_clips(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(clips.into_iter().map(Into::into).collect()))
}

/// List a user's public clips.
async fn list_user_clips(
    State(state): State<AppState>,
    Json(req): Json<ListUserClipsRequest>,
) -> AppResult<ApiResponse<Vec<ClipResponse>>> {
    let limit = req.limit.min(100);
    let clips = state
        .clip_service
        .list_user_clips(&req.user_id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(clips.into_iter().map(Into::into).collect()))
}

/// Add a note to a clip.
async fn add_note(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<AddNoteRequest>,
) -> AppResult<ApiResponse<ClipNoteResponse>> {
    let clip_note = state
        .clip_service
        .add_note(&req.clip_id, &req.note_id, &user.id, req.comment)
        .await?;

    // Get note details
    let note = state.note_service.get(&clip_note.note_id).await.ok();

    Ok(ApiResponse::ok(ClipNoteResponse {
        id: clip_note.id,
        created_at: clip_note.created_at.to_rfc3339(),
        note_id: clip_note.note_id,
        comment: clip_note.comment,
        note: note.map(Into::into),
    }))
}

/// Remove a note from a clip.
async fn remove_note(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RemoveNoteRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .clip_service
        .remove_note(&req.clip_id, &req.note_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// List notes in a clip.
async fn notes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListNotesRequest>,
) -> AppResult<ApiResponse<Vec<ClipNoteResponse>>> {
    let limit = req.limit.min(100);
    let clip_notes = state
        .clip_service
        .list_notes(&req.clip_id, Some(&user.id), limit, req.offset)
        .await?;

    // Get note details for each clip note
    let mut results = Vec::new();
    for cn in clip_notes {
        let note = state.note_service.get(&cn.note_id).await.ok();
        results.push(ClipNoteResponse {
            id: cn.id,
            created_at: cn.created_at.to_rfc3339(),
            note_id: cn.note_id,
            comment: cn.comment,
            note: note.map(Into::into),
        });
    }

    Ok(ApiResponse::ok(results))
}

/// Reorder clips.
async fn reorder(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ReorderClipsRequest>,
) -> AppResult<ApiResponse<()>> {
    state.clip_service.reorder(&user.id, req.clip_ids).await?;

    Ok(ApiResponse::ok(()))
}

/// Reorder notes in a clip.
async fn reorder_notes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ReorderNotesRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .clip_service
        .reorder_notes(&req.clip_id, &user.id, req.clip_note_ids)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Find which clips contain a note.
async fn find_note_in_clips(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FindNoteInClipsRequest>,
) -> AppResult<ApiResponse<Vec<ClipResponse>>> {
    let clip_notes = state
        .clip_service
        .find_clips_with_note(&req.note_id, &user.id)
        .await?;

    // Get clip details for each clip note
    let mut clips = Vec::new();
    for cn in clip_notes {
        if let Some(clip) = state.clip_service.get_by_id(&cn.clip_id).await? {
            clips.push(clip.into());
        }
    }

    Ok(ApiResponse::ok(clips))
}

/// Search notes within a clip by text content.
async fn search_notes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<SearchNotesRequest>,
) -> AppResult<ApiResponse<Vec<ClipNoteResponse>>> {
    let limit = req.limit.min(100);

    // Search notes in clip
    let note_ids = state
        .clip_service
        .search_notes(&req.clip_id, Some(&user.id), &req.query, limit, req.offset)
        .await?;

    // Get note details and build response
    let mut results = Vec::new();
    for note_id in note_ids {
        if let Ok(note) = state.note_service.get(&note_id).await {
            // Get clip note for this note
            let clip_notes = state
                .clip_service
                .list_notes(&req.clip_id, Some(&user.id), 1000, 0)
                .await?;

            if let Some(cn) = clip_notes.iter().find(|cn| cn.note_id == note_id) {
                results.push(ClipNoteResponse {
                    id: cn.id.clone(),
                    created_at: cn.created_at.to_rfc3339(),
                    note_id: cn.note_id.clone(),
                    comment: cn.comment.clone(),
                    note: Some(note.into()),
                });
            }
        }
    }

    Ok(ApiResponse::ok(results))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/update", post(update))
        .route("/delete", post(delete))
        .route("/show", post(show))
        .route("/list", post(list))
        .route("/list-user", post(list_user_clips))
        .route("/add-note", post(add_note))
        .route("/remove-note", post(remove_note))
        .route("/notes", post(notes))
        .route("/reorder", post(reorder))
        .route("/reorder-notes", post(reorder_notes))
        .route("/find-note", post(find_note_in_clips))
        .route("/search", post(search_notes))
}
