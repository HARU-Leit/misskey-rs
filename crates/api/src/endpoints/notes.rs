//! Notes endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::{AntennaService, UpdateNoteInput, note::CreateNoteInput};
use misskey_db::entities::{note, note_edit};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    extractors::{AuthUser, MaybeAuthUser},
    middleware::AppState,
    response::ApiResponse,
};

/// Note response.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NoteResponse {
    pub id: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub user_id: String,
    pub text: Option<String>,
    pub cw: Option<String>,
    pub visibility: String,
    pub reply_id: Option<String>,
    pub renote_id: Option<String>,
    pub channel_id: Option<String>,
    pub replies_count: i32,
    pub renote_count: i32,
    pub reaction_count: i32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_edited: bool,
    /// Whether the note matches a word filter.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub filtered: bool,
    /// The filter action to take (if filtered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_action: Option<String>,
    /// The matched phrases (if filtered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_matches: Option<Vec<String>>,
}

impl From<note::Model> for NoteResponse {
    fn from(note: note::Model) -> Self {
        let is_edited = note.updated_at.is_some();
        Self {
            id: note.id,
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.map(|dt| dt.to_rfc3339()),
            user_id: note.user_id,
            text: note.text,
            cw: note.cw,
            visibility: format!("{:?}", note.visibility).to_lowercase(),
            reply_id: note.reply_id,
            renote_id: note.renote_id,
            channel_id: note.channel_id,
            replies_count: note.replies_count,
            renote_count: note.renote_count,
            reaction_count: note.reaction_count,
            is_edited,
            filtered: false,
            filter_action: None,
            filter_matches: None,
        }
    }
}

/// Note edit history response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEditResponse {
    pub id: String,
    pub note_id: String,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    pub old_cw: Option<String>,
    pub new_cw: Option<String>,
    pub edited_at: String,
}

impl From<note_edit::Model> for NoteEditResponse {
    fn from(edit: note_edit::Model) -> Self {
        Self {
            id: edit.id,
            note_id: edit.note_id,
            old_text: edit.old_text,
            new_text: edit.new_text,
            old_cw: edit.old_cw,
            new_cw: edit.new_cw,
            edited_at: edit.edited_at.to_rfc3339(),
        }
    }
}

/// Create note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    #[serde(flatten)]
    pub input: CreateNoteInput,
}

/// Create a new note.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateNoteRequest>,
) -> AppResult<ApiResponse<NoteResponse>> {
    // Store input data for antenna processing
    let text = req.input.text.clone();
    let reply_id = req.input.reply_id.clone();
    let file_ids = req.input.file_ids.clone();

    // Create the note
    let note = state.note_service.create(&user.id, req.input).await?;

    // Process note against antennas (fire and forget - don't block response)
    let antenna_service = state.antenna_service.clone();
    let note_id = note.id.clone();
    let note_user_id = user.id.clone();
    let note_user_host = user.host.clone();

    tokio::spawn(async move {
        // Get user list memberships for list-based antennas
        // TODO: Implement user_list_service.get_list_memberships_for_user()
        let list_memberships: Vec<String> = vec![];

        let context = AntennaService::create_note_context(
            text.as_deref(),
            &note_user_id,
            note_user_host.as_deref(),
            reply_id.as_deref(),
            &file_ids,
            &list_memberships,
        );

        match antenna_service
            .process_note_for_all_antennas(&note_id, &context)
            .await
        {
            Ok(matched_antennas) => {
                if !matched_antennas.is_empty() {
                    debug!(
                        note_id = %note_id,
                        matched_count = matched_antennas.len(),
                        "Note matched antennas"
                    );
                }
            }
            Err(e) => {
                tracing::error!(error = %e, note_id = %note_id, "Failed to process note for antennas");
            }
        }
    });

    Ok(ApiResponse::ok(note.into()))
}

/// Timeline request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub since_id: Option<String>,
}

const fn default_limit() -> u64 {
    10
}

const fn max_limit() -> u64 {
    100
}

use misskey_db::entities::word_filter::FilterContext;

/// Apply word filters to notes for a user.
async fn apply_word_filters(
    state: &AppState,
    user_id: &str,
    notes: Vec<note::Model>,
    context: FilterContext,
) -> AppResult<Vec<NoteResponse>> {
    let mut responses = Vec::new();

    for note in notes {
        let mut response: NoteResponse = note.clone().into();

        // Apply filter to note text
        if let Some(ref text) = note.text {
            let filter_result = state
                .word_filter_service
                .apply_filters(user_id, text, context.clone())
                .await?;

            if filter_result.matched {
                response.filtered = true;
                response.filter_action = filter_result
                    .action
                    .map(|a| format!("{:?}", a).to_lowercase());
                response.filter_matches = Some(filter_result.matched_phrases);
            }
        }

        responses.push(response);
    }

    Ok(responses)
}

/// Filter notes based on filter action (hide filtered notes if action is Hide).
fn filter_hidden_notes(notes: Vec<NoteResponse>) -> Vec<NoteResponse> {
    notes
        .into_iter()
        .filter(|n| n.filter_action.as_deref() != Some("hide"))
        .collect()
}

/// Get home timeline (notes from followed users + own notes).
async fn timeline(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<TimelineRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit());

    // Get bot user IDs to exclude if hide_bots is enabled
    let exclude_user_ids = state
        .user_service
        .get_exclude_user_ids_for_timeline(&user.id)
        .await?;

    let notes = state
        .note_service
        .home_timeline(
            &user.id,
            limit,
            req.until_id.as_deref(),
            exclude_user_ids.as_deref(),
        )
        .await?;

    // Apply word filters
    let filtered_notes = apply_word_filters(&state, &user.id, notes, FilterContext::Home).await?;
    let result = filter_hidden_notes(filtered_notes);

    Ok(ApiResponse::ok(result))
}

/// Get local timeline.
async fn local_timeline(
    MaybeAuthUser(user): MaybeAuthUser,
    State(state): State<AppState>,
    Json(req): Json<TimelineRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit());

    // Get bot user IDs to exclude if user is authenticated and has hide_bots enabled
    let exclude_user_ids = if let Some(ref user) = user {
        state
            .user_service
            .get_exclude_user_ids_for_timeline(&user.id)
            .await?
    } else {
        None
    };

    let notes = state
        .note_service
        .local_timeline(limit, req.until_id.as_deref(), exclude_user_ids.as_deref())
        .await?;

    // Apply word filters if user is authenticated
    let result = if let Some(ref user) = user {
        let filtered_notes =
            apply_word_filters(&state, &user.id, notes, FilterContext::Public).await?;
        filter_hidden_notes(filtered_notes)
    } else {
        notes.into_iter().map(Into::into).collect()
    };

    Ok(ApiResponse::ok(result))
}

/// Get global timeline.
async fn global_timeline(
    MaybeAuthUser(user): MaybeAuthUser,
    State(state): State<AppState>,
    Json(req): Json<TimelineRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit());

    // Get bot user IDs to exclude if user is authenticated and has hide_bots enabled
    let exclude_user_ids = if let Some(ref user) = user {
        state
            .user_service
            .get_exclude_user_ids_for_timeline(&user.id)
            .await?
    } else {
        None
    };

    let notes = state
        .note_service
        .global_timeline(limit, req.until_id.as_deref(), exclude_user_ids.as_deref())
        .await?;

    // Apply word filters if user is authenticated
    let result = if let Some(ref user) = user {
        let filtered_notes =
            apply_word_filters(&state, &user.id, notes, FilterContext::Public).await?;
        filter_hidden_notes(filtered_notes)
    } else {
        notes.into_iter().map(Into::into).collect()
    };

    Ok(ApiResponse::ok(result))
}

/// Show note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowNoteRequest {
    #[serde(alias = "noteId")]
    pub note_id: String,
}

/// Get a note by ID.
async fn show(
    State(state): State<AppState>,
    Json(req): Json<ShowNoteRequest>,
) -> AppResult<ApiResponse<NoteResponse>> {
    let note = state.note_service.get(&req.note_id).await?;
    Ok(ApiResponse::ok(note.into()))
}

/// Delete note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNoteRequest {
    #[serde(alias = "noteId")]
    pub note_id: String,
}

/// Delete a note.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteNoteRequest>,
) -> AppResult<ApiResponse<()>> {
    state.note_service.delete(&req.note_id, &user.id).await?;
    Ok(ApiResponse::ok(()))
}

/// User notes request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserNotesRequest {
    #[serde(alias = "userId")]
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

/// Get notes by a user.
async fn user_notes(
    State(state): State<AppState>,
    Json(req): Json<UserNotesRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit());
    let notes = state
        .note_service
        .user_notes(&req.user_id, limit, req.until_id.as_deref())
        .await?;
    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

/// Replies request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepliesRequest {
    #[serde(alias = "noteId")]
    pub note_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

/// Get replies to a note.
async fn replies(
    State(state): State<AppState>,
    Json(req): Json<RepliesRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit());
    let notes = state.note_service.get_replies(&req.note_id, limit).await?;
    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

/// Renotes request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenotesRequest {
    #[serde(alias = "noteId")]
    pub note_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

/// Get renotes of a note.
async fn renotes(
    State(state): State<AppState>,
    Json(req): Json<RenotesRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit());
    let notes = state.note_service.get_renotes(&req.note_id, limit).await?;
    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

/// Thread/conversation request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationRequest {
    #[serde(alias = "noteId")]
    pub note_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

/// Get conversation/thread for a note (ancestors and the note itself).
async fn conversation(
    State(state): State<AppState>,
    Json(req): Json<ConversationRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit()) as usize;
    let notes = state
        .note_service
        .get_conversation(&req.note_id, limit)
        .await?;
    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

/// Children (replies tree) request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildrenRequest {
    #[serde(alias = "noteId")]
    pub note_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub depth: u8,
}

/// Get children (reply tree) for a note.
async fn children(
    State(state): State<AppState>,
    Json(req): Json<ChildrenRequest>,
) -> AppResult<ApiResponse<Vec<NoteResponse>>> {
    let limit = req.limit.min(max_limit());
    let notes = state.note_service.get_children(&req.note_id, limit).await?;
    Ok(ApiResponse::ok(notes.into_iter().map(Into::into).collect()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/delete", post(delete))
        .route("/show", post(show))
        .route("/timeline", post(timeline))
        .route("/local-timeline", post(local_timeline))
        .route("/global-timeline", post(global_timeline))
        .route("/users/notes", post(user_notes))
        .route("/replies", post(replies))
        .route("/renotes", post(renotes))
        .route("/conversation", post(conversation))
        .route("/children", post(children))
        .route("/update", post(update_note))
        .route("/history", post(get_history))
}

// ==================== Note Editing ====================

/// Update note request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    #[serde(flatten)]
    pub input: UpdateNoteInput,
}

/// Edit history request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRequest {
    pub note_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Update a note.
async fn update_note(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateNoteRequest>,
) -> AppResult<ApiResponse<NoteResponse>> {
    let note = state.note_service.update(&user.id, req.input).await?;
    Ok(ApiResponse::ok(note.into()))
}

/// Get edit history for a note.
async fn get_history(
    State(state): State<AppState>,
    Json(req): Json<HistoryRequest>,
) -> AppResult<ApiResponse<Vec<NoteEditResponse>>> {
    let limit = req.limit.min(max_limit());
    let history = state
        .note_service
        .get_edit_history(&req.note_id, limit, req.offset)
        .await?;
    Ok(ApiResponse::ok(
        history.into_iter().map(Into::into).collect(),
    ))
}
