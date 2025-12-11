//! API endpoints.

#![allow(missing_docs)]

mod account;
mod admin;
mod announcements;
mod antennas;
mod auth;
mod blocking;
mod channels;
mod clips;
mod drive;
mod emojis;
mod favorites;
mod following;
mod gallery;
mod groups;
mod hashtags;
mod lists;
pub mod mastodon;
mod messaging;
mod meta;
mod metrics;
mod muting;
mod notes;
mod notifications;
mod oauth;
mod pages;
mod poll;
mod reactions;
mod scheduled_notes;
mod search;
mod security_keys;
mod sw;
mod translation;
mod two_factor;
mod users;
mod webhooks;
mod word_filters;

use axum::Router;

use crate::middleware::AppState;
use crate::sse;

/// Create the API router.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .nest("/meta", meta::router())
        .nest("/notes", notes::router())
        .nest("/users", users::router())
        .nest("/following", following::router())
        .nest("/notes/reactions", reactions::router())
        .nest("/notifications", notifications::router())
        .nest("/blocking", blocking::router())
        .nest("/mute", muting::router())
        .nest("/drive", drive::router())
        .nest("/poll", poll::router())
        .nest("/search", search::router())
        .nest("/hashtags", hashtags::router())
        .nest("/notes/favorites", favorites::router())
        .nest("/users/lists", lists::router())
        .nest("/admin", admin::router())
        .nest("/emojis", emojis::router())
        .nest("/announcements", announcements::router())
        .nest("/antennas", antennas::router())
        .nest("/channels", channels::router())
        .nest("/clips", clips::router())
        .nest("/messaging", messaging::router())
        .nest("/word-filters", word_filters::router())
        .nest("/notes/schedule", scheduled_notes::router())
        .nest("/i/2fa", two_factor::router())
        .nest("/i/security-keys", security_keys::router())
        .nest("/oauth", oauth::router())
        .nest("/i/webhooks", webhooks::router())
        .nest("/pages", pages::router())
        .nest("/gallery", gallery::router())
        .nest("/translate", translation::router())
        .nest("/sw", sw::router())
        .nest("/i/account", account::router())
        .nest("/groups", groups::router())
        .nest("/streaming/sse", sse::router())
        .nest("/metrics", metrics::router())
        // Mastodon-compatible API
        .nest("/v1", mastodon::router())
}
