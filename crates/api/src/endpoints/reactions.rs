//! Reactions endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Create reaction request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReactionRequest {
    pub note_id: String,
    pub reaction: String,
}

/// Create a reaction on a note.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateReactionRequest>,
) -> AppResult<ApiResponse<()>> {
    // Get the note first to find the author
    let note = state.note_service.get(&req.note_id).await?;

    state
        .reaction_service
        .create(&user.id, &req.note_id, &req.reaction)
        .await?;

    // Create notification for the note author (if not self-reaction)
    if note.user_id != user.id
        && let Err(e) = state
            .notification_service
            .create_reaction_notification(&note.user_id, &user.id, &req.note_id, &req.reaction)
            .await
    {
        tracing::warn!(error = %e, "Failed to create reaction notification");
    }

    Ok(ApiResponse::ok(()))
}

/// Delete reaction request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReactionRequest {
    pub note_id: String,
}

/// Delete a reaction from a note.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteReactionRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .reaction_service
        .delete(&user.id, &req.note_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// List reactions request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReactionsRequest {
    pub note_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

const fn default_limit() -> u64 {
    10
}

/// Reaction response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionResponse {
    pub id: String,
    pub created_at: String,
    pub user_id: String,
    pub note_id: String,
    pub reaction: String,
}

impl From<misskey_db::entities::reaction::Model> for ReactionResponse {
    fn from(r: misskey_db::entities::reaction::Model) -> Self {
        Self {
            id: r.id,
            created_at: r.created_at.to_rfc3339(),
            user_id: r.user_id,
            note_id: r.note_id,
            reaction: r.reaction,
        }
    }
}

/// Get reactions on a note.
async fn reactions(
    State(state): State<AppState>,
    Json(req): Json<ListReactionsRequest>,
) -> AppResult<ApiResponse<Vec<ReactionResponse>>> {
    let limit = req.limit.min(100);
    let reactions = state
        .reaction_service
        .get_reactions(&req.note_id, limit, req.until_id.as_deref())
        .await?;

    Ok(ApiResponse::ok(
        reactions.into_iter().map(Into::into).collect(),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/delete", post(delete))
        .route("/reactions", post(reactions))
}
