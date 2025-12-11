//! Mastodon Bookmarks API.
//!
//! Provides bookmark-related endpoints for Mastodon compatibility.
//!
//! Endpoints:
//! - GET /api/v1/bookmarks - Get bookmarked statuses

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
    #[allow(dead_code)]
    pub since_id: Option<String>,
    #[allow(dead_code)]
    pub min_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    20
}

/// GET /api/v1/bookmarks - Get bookmarked statuses.
async fn get_bookmarks(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Status>>> {
    let limit = query.limit.min(40);

    // Find the "Bookmarks" clip using list_my_clips
    let clips = state.clip_service.list_my_clips(&user.id, 100, 0).await?;
    let bookmark_clip = clips.into_iter().find(|c| c.name == "Bookmarks");

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let mut statuses = Vec::new();

    if let Some(clip) = bookmark_clip {
        // Get clip notes using list_notes
        let clip_notes = state
            .clip_service
            .list_notes(&clip.id, Some(&user.id), limit, 0)
            .await?;

        for clip_note in clip_notes {
            // clip_note contains note_id, need to fetch the actual note
            if let Ok(note) = state.note_service.get(&clip_note.note_id).await {
                let author = state.user_service.get(&note.user_id).await.ok();
                let mut status = note_to_status(note, author.as_ref(), base_url);
                status.bookmarked = Some(true);
                statuses.push(status);
            }
        }
    }

    Ok(Json(statuses))
}

/// Create the bookmarks router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_bookmarks))
}
