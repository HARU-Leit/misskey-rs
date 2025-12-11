//! Mastodon-compatible API endpoints.
//!
//! Provides compatibility with Mastodon clients by implementing
//! a subset of the Mastodon API v1.

#![allow(missing_docs)]

mod accounts;
mod blocks;
mod bookmarks;
mod favourites;
mod media;
mod mutes;
mod statuses;
mod timelines;

use axum::Router;

use crate::middleware::AppState;

// Re-export for use in other modules
pub use media::MediaAttachment;
pub use statuses::{Account, Status};

/// Create the Mastodon API v1 router.
pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/statuses", statuses::router())
        .nest("/timelines", timelines::router())
        .nest("/accounts", accounts::router())
        .nest("/media", media::router())
        .nest("/favourites", favourites::router())
        .nest("/blocks", blocks::router())
        .nest("/mutes", mutes::router())
        .nest("/bookmarks", bookmarks::router())
}
