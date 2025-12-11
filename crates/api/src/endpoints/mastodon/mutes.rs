//! Mastodon Mutes API.
//!
//! Provides mute-related endpoints for Mastodon compatibility.
//!
//! Endpoints:
//! - GET /api/v1/mutes - Get muted accounts

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use misskey_common::AppResult;
use serde::Deserialize;

use crate::{extractors::AuthUser, middleware::AppState};

use super::statuses::{Account, user_to_account};

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub max_id: Option<String>,
    pub since_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

const fn default_limit() -> u64 {
    40
}

/// GET /api/v1/mutes - Get muted accounts.
async fn get_mutes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Account>>> {
    let limit = query.limit.min(80);

    let mutings = state
        .muting_service
        .get_muting(&user.id, limit, query.max_id.as_deref())
        .await?;

    let base_url = &state.base_url;

    let mut accounts = Vec::new();
    for muting in mutings {
        if let Ok(muted_user) = state.user_service.get(&muting.mutee_id).await {
            accounts.push(user_to_account(&muted_user, base_url));
        }
    }

    Ok(Json(accounts))
}

/// Create the mutes router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_mutes))
}
