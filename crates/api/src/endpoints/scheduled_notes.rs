//! Scheduled notes endpoints.

use axum::{Json, Router, extract::State, routing::post};
use chrono::{DateTime, Utc};
use misskey_common::AppResult;
use misskey_core::services::scheduled_note::{CreateScheduledNoteInput, UpdateScheduledNoteInput};
use misskey_db::entities::scheduled_note::{self, ScheduledStatus, ScheduledVisibility};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

// ==================== Request/Response Types ====================

/// Scheduled note response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledNoteResponse {
    pub id: String,
    pub user_id: String,
    pub text: Option<String>,
    pub cw: Option<String>,
    pub visibility: String,
    pub visible_user_ids: Vec<String>,
    pub file_ids: Vec<String>,
    pub reply_id: Option<String>,
    pub renote_id: Option<String>,
    pub poll: Option<serde_json::Value>,
    pub scheduled_at: String,
    pub status: String,
    pub posted_note_id: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl From<scheduled_note::Model> for ScheduledNoteResponse {
    fn from(n: scheduled_note::Model) -> Self {
        Self {
            id: n.id,
            user_id: n.user_id,
            text: n.text,
            cw: n.cw,
            visibility: match n.visibility {
                ScheduledVisibility::Public => "public".to_string(),
                ScheduledVisibility::Home => "home".to_string(),
                ScheduledVisibility::Followers => "followers".to_string(),
                ScheduledVisibility::Specified => "specified".to_string(),
            },
            visible_user_ids: serde_json::from_value(n.visible_user_ids).unwrap_or_default(),
            file_ids: serde_json::from_value(n.file_ids).unwrap_or_default(),
            reply_id: n.reply_id,
            renote_id: n.renote_id,
            poll: n.poll,
            scheduled_at: n.scheduled_at.to_rfc3339(),
            status: match n.status {
                ScheduledStatus::Pending => "pending".to_string(),
                ScheduledStatus::Processing => "processing".to_string(),
                ScheduledStatus::Posted => "posted".to_string(),
                ScheduledStatus::Failed => "failed".to_string(),
                ScheduledStatus::Cancelled => "cancelled".to_string(),
            },
            posted_note_id: n.posted_note_id,
            error_message: n.error_message,
            retry_count: n.retry_count,
            created_at: n.created_at.to_rfc3339(),
            updated_at: n.updated_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Create scheduled note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduledNoteRequest {
    pub text: Option<String>,
    pub cw: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: ScheduledVisibility,
    #[serde(default)]
    pub visible_user_ids: Vec<String>,
    #[serde(default)]
    pub file_ids: Vec<String>,
    pub reply_id: Option<String>,
    pub renote_id: Option<String>,
    pub poll: Option<serde_json::Value>,
    pub scheduled_at: DateTime<Utc>,
}

const fn default_visibility() -> ScheduledVisibility {
    ScheduledVisibility::Public
}

/// Update scheduled note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduledNoteRequest {
    pub note_id: String,
    pub text: Option<Option<String>>,
    pub cw: Option<Option<String>>,
    pub visibility: Option<ScheduledVisibility>,
    pub visible_user_ids: Option<Vec<String>>,
    pub file_ids: Option<Vec<String>>,
    pub scheduled_at: Option<DateTime<Utc>>,
}

/// List scheduled notes request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListScheduledNotesRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
    #[serde(default)]
    pub pending_only: bool,
}

/// Show scheduled note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowScheduledNoteRequest {
    pub note_id: String,
}

/// Cancel/Delete/Retry scheduled note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteIdRequest {
    pub note_id: String,
}

const fn default_limit() -> u64 {
    10
}

// ==================== Handlers ====================

/// Create a new scheduled note.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateScheduledNoteRequest>,
) -> AppResult<ApiResponse<ScheduledNoteResponse>> {
    let input = CreateScheduledNoteInput {
        text: req.text,
        cw: req.cw,
        visibility: req.visibility,
        visible_user_ids: req.visible_user_ids,
        file_ids: req.file_ids,
        reply_id: req.reply_id,
        renote_id: req.renote_id,
        poll: req.poll,
        scheduled_at: req.scheduled_at,
    };

    let note = state.scheduled_note_service.create(&user.id, input).await?;

    Ok(ApiResponse::ok(note.into()))
}

/// Update a scheduled note.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateScheduledNoteRequest>,
) -> AppResult<ApiResponse<ScheduledNoteResponse>> {
    let input = UpdateScheduledNoteInput {
        note_id: req.note_id,
        text: req.text,
        cw: req.cw,
        visibility: req.visibility,
        visible_user_ids: req.visible_user_ids,
        file_ids: req.file_ids,
        scheduled_at: req.scheduled_at,
    };

    let note = state.scheduled_note_service.update(&user.id, input).await?;

    Ok(ApiResponse::ok(note.into()))
}

/// Show a scheduled note.
async fn show(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowScheduledNoteRequest>,
) -> AppResult<ApiResponse<ScheduledNoteResponse>> {
    let note = state
        .scheduled_note_service
        .get_by_id_for_user(&req.note_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(note.into()))
}

/// List scheduled notes.
async fn list(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListScheduledNotesRequest>,
) -> AppResult<ApiResponse<Vec<ScheduledNoteResponse>>> {
    let limit = req.limit.min(100);
    let notes = if req.pending_only {
        state
            .scheduled_note_service
            .list_pending_notes(&user.id, limit, req.offset)
            .await?
    } else {
        state
            .scheduled_note_service
            .list_notes(&user.id, limit, req.offset)
            .await?
    };

    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

/// Cancel a scheduled note.
async fn cancel(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NoteIdRequest>,
) -> AppResult<ApiResponse<ScheduledNoteResponse>> {
    let note = state
        .scheduled_note_service
        .cancel(&req.note_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(note.into()))
}

/// Delete a scheduled note.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NoteIdRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .scheduled_note_service
        .delete(&req.note_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Retry a failed scheduled note.
async fn retry(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<NoteIdRequest>,
) -> AppResult<ApiResponse<ScheduledNoteResponse>> {
    let note = state
        .scheduled_note_service
        .retry(&req.note_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(note.into()))
}

/// Count scheduled notes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountResponse {
    pub total: u64,
    pub pending: u64,
}

async fn count(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<CountResponse>> {
    let total = state.scheduled_note_service.count_notes(&user.id).await?;
    let pending = state
        .scheduled_note_service
        .count_pending_notes(&user.id)
        .await?;

    Ok(ApiResponse::ok(CountResponse { total, pending }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/update", post(update))
        .route("/show", post(show))
        .route("/list", post(list))
        .route("/cancel", post(cancel))
        .route("/delete", post(delete))
        .route("/retry", post(retry))
        .route("/count", post(count))
}
