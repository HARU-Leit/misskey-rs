//! Blocking endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_db::entities::blocking::Model as BlockingModel;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Block user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockUserRequest {
    pub user_id: String,
}

/// Unblock user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnblockUserRequest {
    pub user_id: String,
}

/// List blocking request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBlockingRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

const fn default_limit() -> u64 {
    30
}

/// Blocking response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockingResponse {
    pub id: String,
    pub created_at: String,
    pub blockee_id: String,
}

impl From<BlockingModel> for BlockingResponse {
    fn from(b: BlockingModel) -> Self {
        Self {
            id: b.id,
            created_at: b.created_at.to_rfc3339(),
            blockee_id: b.blockee_id,
        }
    }
}

/// Block a user.
async fn block_user(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<BlockUserRequest>,
) -> AppResult<ApiResponse<BlockingResponse>> {
    let blocking = state.blocking_service.block(&user.id, &req.user_id).await?;
    Ok(ApiResponse::ok(blocking.into()))
}

/// Unblock a user.
async fn unblock_user(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UnblockUserRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .blocking_service
        .unblock(&user.id, &req.user_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Get list of blocked users.
async fn list_blocking(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListBlockingRequest>,
) -> AppResult<ApiResponse<Vec<BlockingResponse>>> {
    let limit = req.limit.min(100);
    let blockings = state
        .blocking_service
        .get_blocking(&user.id, limit, req.until_id.as_deref())
        .await?;
    Ok(ApiResponse::ok(
        blockings.into_iter().map(Into::into).collect(),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(block_user))
        .route("/delete", post(unblock_user))
        .route("/list", post(list_blocking))
}
