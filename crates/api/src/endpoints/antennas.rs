//! Antenna endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::services::antenna::{CreateAntennaInput, UpdateAntennaInput};
use misskey_db::entities::antenna::{self, AntennaSource};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

// ==================== Request/Response Types ====================

/// Antenna response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AntennaResponse {
    pub id: String,
    pub created_at: String,
    pub user_id: String,
    pub name: String,
    pub src: String,
    pub user_list_id: Option<String>,
    pub keywords: Vec<Vec<String>>,
    pub exclude_keywords: Vec<Vec<String>>,
    pub users: Vec<String>,
    pub instances: Vec<String>,
    pub case_sensitive: bool,
    pub with_replies: bool,
    pub with_file: bool,
    pub notify: bool,
    pub local_only: bool,
    pub is_active: bool,
    pub notes_count: i64,
    pub last_used_at: Option<String>,
}

impl From<antenna::Model> for AntennaResponse {
    fn from(a: antenna::Model) -> Self {
        Self {
            id: a.id,
            created_at: a.created_at.to_rfc3339(),
            user_id: a.user_id,
            name: a.name,
            src: match a.src {
                AntennaSource::Home => "home".to_string(),
                AntennaSource::All => "all".to_string(),
                AntennaSource::Users => "users".to_string(),
                AntennaSource::List => "list".to_string(),
                AntennaSource::Instances => "instances".to_string(),
            },
            user_list_id: a.user_list_id,
            keywords: serde_json::from_value(a.keywords).unwrap_or_default(),
            exclude_keywords: serde_json::from_value(a.exclude_keywords).unwrap_or_default(),
            users: serde_json::from_value(a.users).unwrap_or_default(),
            instances: serde_json::from_value(a.instances).unwrap_or_default(),
            case_sensitive: a.case_sensitive,
            with_replies: a.with_replies,
            with_file: a.with_file,
            notify: a.notify,
            local_only: a.local_only,
            is_active: a.is_active,
            notes_count: a.notes_count,
            last_used_at: a.last_used_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Antenna note response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AntennaNoteResponse {
    pub id: String,
    pub antenna_id: String,
    pub note_id: String,
    pub is_read: bool,
    pub created_at: String,
}

/// Show antenna request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowAntennaRequest {
    pub antenna_id: String,
}

/// Delete antenna request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAntennaRequest {
    pub antenna_id: String,
}

/// List antennas request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAntennasRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Get notes request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNotesRequest {
    pub antenna_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

/// Reorder antennas request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderAntennasRequest {
    pub antenna_ids: Vec<String>,
}

/// Mark as read request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkAsReadRequest {
    pub antenna_id: String,
}

const fn default_limit() -> u64 {
    10
}

// ==================== Handlers ====================

/// Create a new antenna.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateAntennaInput>,
) -> AppResult<ApiResponse<AntennaResponse>> {
    let antenna = state.antenna_service.create(&user.id, input).await?;

    Ok(ApiResponse::ok(antenna.into()))
}

/// Update an antenna.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<UpdateAntennaInput>,
) -> AppResult<ApiResponse<AntennaResponse>> {
    let antenna = state.antenna_service.update(&user.id, input).await?;

    Ok(ApiResponse::ok(antenna.into()))
}

/// Delete an antenna.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteAntennaRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .antenna_service
        .delete(&req.antenna_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Show an antenna.
async fn show(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowAntennaRequest>,
) -> AppResult<ApiResponse<AntennaResponse>> {
    let antenna = state
        .antenna_service
        .get_by_id_for_user(&req.antenna_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(antenna.into()))
}

/// List antennas.
async fn list(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListAntennasRequest>,
) -> AppResult<ApiResponse<Vec<AntennaResponse>>> {
    let limit = req.limit.min(100);
    let antennas = state
        .antenna_service
        .list_antennas(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        antennas.into_iter().map(Into::into).collect(),
    ))
}

/// Get notes from an antenna.
async fn notes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetNotesRequest>,
) -> AppResult<ApiResponse<Vec<AntennaNoteResponse>>> {
    let limit = req.limit.min(100);
    let notes = state
        .antenna_service
        .get_notes(&req.antenna_id, &user.id, limit, req.until_id.as_deref())
        .await?;

    Ok(ApiResponse::ok(
        notes
            .into_iter()
            .map(|n| AntennaNoteResponse {
                id: n.id,
                antenna_id: n.antenna_id,
                note_id: n.note_id,
                is_read: n.is_read,
                created_at: n.created_at.to_rfc3339(),
            })
            .collect(),
    ))
}

/// Reorder antennas.
async fn reorder(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ReorderAntennasRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .antenna_service
        .reorder(&user.id, req.antenna_ids)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Mark all notes in an antenna as read.
async fn mark_all_as_read(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<MarkAsReadRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .antenna_service
        .mark_all_as_read(&req.antenna_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Get unread count for an antenna.
async fn unread_count(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowAntennaRequest>,
) -> AppResult<ApiResponse<u64>> {
    let count = state
        .antenna_service
        .get_unread_count(&req.antenna_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(count))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/update", post(update))
        .route("/delete", post(delete))
        .route("/show", post(show))
        .route("/list", post(list))
        .route("/notes", post(notes))
        .route("/reorder", post(reorder))
        .route("/mark-all-as-read", post(mark_all_as_read))
        .route("/unread-count", post(unread_count))
}
