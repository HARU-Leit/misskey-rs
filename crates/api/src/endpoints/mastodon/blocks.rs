//! Mastodon Blocks API.
//!
//! Provides block-related endpoints for Mastodon compatibility.
//!
//! Endpoints:
//! - GET /api/v1/blocks - Get blocked accounts

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use misskey_common::AppResult;
use serde::Deserialize;

use crate::{extractors::AuthUser, middleware::AppState};

use super::statuses::{user_to_account, Account};

/// Pagination query parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub max_id: Option<String>,
    pub since_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    40
}

/// GET /api/v1/blocks - Get blocked accounts.
async fn get_blocks(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Account>>> {
    let limit = query.limit.min(80);

    let blockings = state
        .blocking_service
        .get_blocking(&user.id, limit, query.max_id.as_deref())
        .await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let mut accounts = Vec::new();
    for blocking in blockings {
        if let Ok(blocked_user) = state.user_service.get(&blocking.blockee_id).await {
            accounts.push(user_to_account(&blocked_user, base_url));
        }
    }

    Ok(Json(accounts))
}

/// Create the blocks router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_blocks))
}
