//! Mastodon-compatible API endpoints.
//!
//! Provides compatibility with Mastodon clients by implementing
//! a subset of the Mastodon API v1.

mod statuses;
mod timelines;
mod accounts;

use axum::Router;

use crate::middleware::AppState;

/// Create the Mastodon API v1 router.
pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/statuses", statuses::router())
        .nest("/timelines", timelines::router())
        .nest("/accounts", accounts::router())
}
