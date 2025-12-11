//! Muting endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_db::entities::muting::Model as MutingModel;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Mute user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MuteUserRequest {
    pub user_id: String,
    /// Duration in seconds. None = permanent mute.
    pub expires_in: Option<i64>,
}

/// Unmute user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmuteUserRequest {
    pub user_id: String,
}

/// List muting request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMutingRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

const fn default_limit() -> u64 {
    30
}

/// Muting response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutingResponse {
    pub id: String,
    pub created_at: String,
    pub mutee_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl From<MutingModel> for MutingResponse {
    fn from(m: MutingModel) -> Self {
        Self {
            id: m.id,
            created_at: m.created_at.to_rfc3339(),
            mutee_id: m.mutee_id,
            expires_at: m.expires_at.map(|e| e.to_rfc3339()),
        }
    }
}

/// Mute a user.
async fn mute_user(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<MuteUserRequest>,
) -> AppResult<ApiResponse<MutingResponse>> {
    let muting = state
        .muting_service
        .mute(&user.id, &req.user_id, req.expires_in)
        .await?;
    Ok(ApiResponse::ok(muting.into()))
}

/// Unmute a user.
async fn unmute_user(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UnmuteUserRequest>,
) -> AppResult<ApiResponse<()>> {
    state.muting_service.unmute(&user.id, &req.user_id).await?;
    Ok(ApiResponse::ok(()))
}

/// Get list of muted users.
async fn list_muting(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListMutingRequest>,
) -> AppResult<ApiResponse<Vec<MutingResponse>>> {
    let limit = req.limit.min(100);
    let mutings = state
        .muting_service
        .get_muting(&user.id, limit, req.until_id.as_deref())
        .await?;
    Ok(ApiResponse::ok(
        mutings.into_iter().map(Into::into).collect(),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(mute_user))
        .route("/delete", post(unmute_user))
        .route("/list", post(list_muting))
}
