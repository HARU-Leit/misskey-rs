//! Mastodon Favourites API.
//!
//! Provides favourite-related endpoints for Mastodon compatibility.
//!
//! Endpoints:
//! - GET /api/v1/favourites - Get favourited statuses

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use misskey_common::AppResult;
use serde::Deserialize;

use crate::{extractors::AuthUser, middleware::AppState};

use super::statuses::{note_to_status, Status};

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub max_id: Option<String>,
    pub since_id: Option<String>,
    pub min_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    20
}

/// GET /api/v1/favourites - Get favourited statuses.
async fn get_favourites(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Status>>> {
    let limit = query.limit.min(40);

    let favorites = state
        .note_favorite_service
        .get_favorites(&user.id, limit, query.max_id.as_deref())
        .await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let mut statuses = Vec::new();
    for favorite in favorites {
        if let Ok(note) = state.note_service.get(&favorite.note_id).await {
            // Get the note author
            let author = state.user_service.get(&note.user_id).await.ok();
            statuses.push(note_to_status(note, author.as_ref(), base_url));
        }
    }

    Ok(Json(statuses))
}

/// Create the favourites router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_favourites))
}
