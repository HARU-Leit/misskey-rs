//! Note favorites (bookmarks) endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::{AppResult};
use misskey_db::entities::note;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Favorite request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteRequest {
    pub note_id: String,
}

/// Favorite response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteResponse {
    pub id: String,
    pub note_id: String,
    pub created_at: String,
}

/// Favorited note response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritedNoteResponse {
    pub id: String,
    pub created_at: String,
    pub note_id: String,
    pub note: NoteResponse,
}

/// Simple note response for favorites list.
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
    fn from(note: note::Model) -> Self {
        Self {
            id: note.id,
            created_at: note.created_at.to_rfc3339(),
            user_id: note.user_id,
            text: note.text,
            cw: note.cw,
            visibility: format!("{:?}", note.visibility).to_lowercase(),
        }
    }
}

/// List favorites request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFavoritesRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

const fn default_limit() -> u64 {
    10
}

/// Add note to favorites.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FavoriteRequest>,
) -> AppResult<ApiResponse<FavoriteResponse>> {
    let favorite = state
        .note_favorite_service
        .create(&user.id, &req.note_id)
        .await?;

    Ok(ApiResponse::ok(FavoriteResponse {
        id: favorite.id,
        note_id: favorite.note_id,
        created_at: favorite.created_at.to_rfc3339(),
    }))
}

/// Remove note from favorites.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FavoriteRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .note_favorite_service
        .delete(&user.id, &req.note_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// List user's favorites.
async fn list(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListFavoritesRequest>,
) -> AppResult<ApiResponse<Vec<FavoritedNoteResponse>>> {
    let limit = req.limit.min(100);
    let favorites = state
        .note_favorite_service
        .get_favorites(&user.id, limit, req.until_id.as_deref())
        .await?;

    // Get notes for each favorite
    let mut results = Vec::new();
    for fav in favorites {
        if let Ok(note) = state.note_service.get(&fav.note_id).await {
            results.push(FavoritedNoteResponse {
                id: fav.id,
                created_at: fav.created_at.to_rfc3339(),
                note_id: fav.note_id,
                note: note.into(),
            });
        }
    }

    Ok(ApiResponse::ok(results))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/delete", post(delete))
        .route("/list", post(list))
}
