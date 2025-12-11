//! `ActivityPub` channel (Group) endpoint handler.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use misskey_db::repositories::{ChannelRepository, DriveFileRepository};
use tracing::{error, info};

use crate::convert::{ChannelToApGroup, UrlConfig};

/// State required for channel `ActivityPub` handler.
#[derive(Clone)]
pub struct ChannelApState {
    pub channel_repo: ChannelRepository,
    pub drive_file_repo: DriveFileRepository,
    pub url_config: UrlConfig,
}

impl ChannelApState {
    /// Create a new channel AP state.
    #[must_use]
    pub const fn new(
        channel_repo: ChannelRepository,
        drive_file_repo: DriveFileRepository,
        url_config: UrlConfig,
    ) -> Self {
        Self {
            channel_repo,
            drive_file_repo,
            url_config,
        }
    }
}

/// Handle GET /channels/{id} for `ActivityPub` Group retrieval.
///
/// Returns the channel as an `ActivityPub` Group object.
pub async fn channel_handler(
    State(state): State<ChannelApState>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    info!(channel_id = %channel_id, "ActivityPub channel lookup");

    // Find channel
    let channel = match state.channel_repo.find_by_id(&channel_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            info!(channel_id = %channel_id, "Channel not found");
            return (StatusCode::NOT_FOUND, "Channel not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch channel");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Check if channel is local (no host)
    if channel.host.is_some() {
        return (
            StatusCode::GONE,
            "Remote channels should be fetched from their origin server",
        )
            .into_response();
    }

    // Check if channel is archived (return 410 Gone for archived channels)
    if channel.is_archived {
        return (StatusCode::GONE, "Channel is archived").into_response();
    }

    // Check if federation is enabled for this channel (has URI)
    if channel.uri.is_none() {
        return (StatusCode::NOT_FOUND, "Channel federation is not enabled").into_response();
    }

    // Get banner URL if available
    let banner_url = if let Some(ref banner_id) = channel.banner_id {
        match state.drive_file_repo.find_by_id(banner_id).await {
            Ok(Some(file)) => Some(file.url),
            _ => None,
        }
    } else {
        None
    };

    // Convert to ActivityPub Group
    let group = channel.to_ap_group(&state.url_config, banner_url.as_deref());

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(group),
    )
        .into_response()
}

/// Handle GET /channels/{id}/inbox for channel inbox.
///
/// Note: Actual inbox processing is done via the main inbox endpoint.
/// This returns 405 Method Not Allowed for GET requests.
pub async fn channel_inbox_handler(Path(channel_id): Path<String>) -> impl IntoResponse {
    info!(channel_id = %channel_id, "Channel inbox GET request (not supported)");
    (StatusCode::METHOD_NOT_ALLOWED, "POST requests only")
}

/// Handle GET /channels/{id}/outbox for channel outbox.
///
/// Returns an `OrderedCollection` of the channel's notes.
pub async fn channel_outbox_handler(
    State(state): State<ChannelApState>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    info!(channel_id = %channel_id, "ActivityPub channel outbox");

    // Verify channel exists and is local
    let channel = match state.channel_repo.find_by_id(&channel_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Channel not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch channel");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    if channel.host.is_some() {
        return (StatusCode::GONE, "Remote channel").into_response();
    }

    // Return OrderedCollection stub (full implementation would paginate notes)
    let outbox = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "OrderedCollection",
        "id": state.url_config.channel_outbox_url(&channel_id).to_string(),
        "totalItems": channel.notes_count,
        "first": format!("{}/page/1", state.url_config.channel_outbox_url(&channel_id)),
    });

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(outbox),
    )
        .into_response()
}

/// Handle GET /channels/{id}/followers for channel followers collection.
pub async fn channel_followers_handler(
    State(state): State<ChannelApState>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    info!(channel_id = %channel_id, "ActivityPub channel followers");

    // Verify channel exists and is local
    let channel = match state.channel_repo.find_by_id(&channel_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Channel not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch channel");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    if channel.host.is_some() {
        return (StatusCode::GONE, "Remote channel").into_response();
    }

    // Return OrderedCollection stub
    let followers = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "OrderedCollection",
        "id": state.url_config.channel_followers_url(&channel_id).to_string(),
        "totalItems": channel.users_count,
    });

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(followers),
    )
        .into_response()
}
