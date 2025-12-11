//! Mastodon timelines API.
//!
//! Provides GET /api/v1/timelines/home, /api/v1/timelines/public, and /api/v1/timelines/bubble.

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use misskey_common::AppResult;
use misskey_db::entities::note;
use serde::Deserialize;

use crate::{extractors::AuthUser, middleware::AppState};

use super::statuses::{Account, Status};

/// Timeline query parameters.
#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    pub max_id: Option<String>,
    pub since_id: Option<String>,
    pub min_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
    pub local: Option<bool>,
    pub remote: Option<bool>,
    pub only_media: Option<bool>,
}

const fn default_limit() -> i32 {
    20
}

/// Convert Misskey visibility to Mastodon visibility.
fn misskey_to_mastodon_visibility(visibility: &note::Visibility) -> String {
    match visibility {
        note::Visibility::Public => "public".to_string(),
        note::Visibility::Home => "unlisted".to_string(),
        note::Visibility::Followers => "private".to_string(),
        note::Visibility::Specified => "direct".to_string(),
    }
}

/// Convert note to Mastodon status.
fn note_to_status(note: note::Model, base_url: &str) -> Status {
    let visibility = misskey_to_mastodon_visibility(&note.visibility);

    Status {
        id: note.id.clone(),
        created_at: note.created_at.to_rfc3339(),
        in_reply_to_id: note.reply_id.clone(),
        in_reply_to_account_id: None,
        sensitive: note.cw.is_some(),
        spoiler_text: note.cw.clone().unwrap_or_default(),
        visibility,
        language: None,
        uri: format!("{}/notes/{}", base_url, note.id),
        url: Some(format!("{}/notes/{}", base_url, note.id)),
        replies_count: note.replies_count,
        reblogs_count: note.renote_count,
        favourites_count: note.reaction_count,
        content: note.text.clone().unwrap_or_default(),
        reblog: None,
        account: Account {
            id: note.user_id.clone(),
            username: "user".to_string(),
            acct: "user".to_string(),
            display_name: "User".to_string(),
            locked: false,
            bot: false,
            created_at: note.created_at.to_rfc3339(),
            note: String::new(),
            url: format!("{}/users/{}", base_url, note.user_id),
            avatar: String::new(),
            avatar_static: String::new(),
            header: String::new(),
            header_static: String::new(),
            followers_count: 0,
            following_count: 0,
            statuses_count: 0,
            last_status_at: None,
            emojis: vec![],
            fields: vec![],
        },
        media_attachments: vec![],
        mentions: vec![],
        tags: vec![],
        emojis: vec![],
        card: None,
        poll: None,
        favourited: None,
        reblogged: None,
        bookmarked: None,
    }
}

/// GET /api/v1/timelines/home - Get home timeline.
async fn home_timeline(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Query(params): Query<TimelineQuery>,
) -> AppResult<Json<Vec<Status>>> {
    let limit = params.limit.min(40).max(1) as u64;

    // Get bot user IDs to exclude if hide_bots is enabled
    let exclude_user_ids = state
        .user_service
        .get_exclude_user_ids_for_timeline(&user.id)
        .await?;

    let notes = state
        .note_service
        .home_timeline(
            &user.id,
            limit,
            params.max_id.as_deref(),
            exclude_user_ids.as_deref(),
        )
        .await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let statuses: Vec<Status> = notes
        .into_iter()
        .map(|n| note_to_status(n, base_url))
        .collect();

    Ok(Json(statuses))
}

/// GET /api/v1/timelines/public - Get public timeline.
async fn public_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineQuery>,
) -> AppResult<Json<Vec<Status>>> {
    let limit = params.limit.min(40).max(1) as u64;
    let local_only = params.local.unwrap_or(false);

    // Public timeline doesn't have authentication, so no bot filtering
    let notes = if local_only {
        state
            .note_service
            .local_timeline(limit, params.max_id.as_deref(), None)
            .await?
    } else {
        state
            .note_service
            .global_timeline(limit, params.max_id.as_deref(), None)
            .await?
    };

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let statuses: Vec<Status> = notes
        .into_iter()
        .map(|n| note_to_status(n, base_url))
        .collect();

    Ok(Json(statuses))
}

/// GET /api/v1/timelines/bubble - Get bubble timeline.
///
/// Shows public notes from local users and whitelisted remote instances.
/// The list of whitelisted instances is configured in `meta_settings.bubble_instances`.
async fn bubble_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineQuery>,
) -> AppResult<Json<Vec<Status>>> {
    let limit = params.limit.min(40).max(1) as u64;

    // Get bubble instances from meta settings
    let bubble_hosts = state.meta_settings_service.get_bubble_instances().await?;

    // Bubble timeline doesn't have authentication, so no bot filtering
    let notes = state
        .note_service
        .bubble_timeline(&bubble_hosts, limit, params.max_id.as_deref(), None)
        .await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let statuses: Vec<Status> = notes
        .into_iter()
        .map(|n| note_to_status(n, base_url))
        .collect();

    Ok(Json(statuses))
}

/// Create the timelines router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/home", get(home_timeline))
        .route("/public", get(public_timeline))
        .route("/bubble", get(bubble_timeline))
}
