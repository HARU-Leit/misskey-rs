//! `WebFinger` handler for actor discovery.

#![allow(clippy::expect_used)] // URL joins with known-valid paths cannot fail

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use misskey_db::repositories::{ChannelRepository, UserRepository};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use url::Url;

/// `WebFinger` query parameters.
#[derive(Debug, Deserialize)]
pub struct WebfingerQuery {
    pub resource: String,
}

/// `WebFinger` response.
#[derive(Debug, Serialize)]
pub struct WebfingerResponse {
    pub subject: String,
    pub aliases: Vec<String>,
    pub links: Vec<WebfingerLink>,
}

/// `WebFinger` link.
#[derive(Debug, Serialize)]
pub struct WebfingerLink {
    pub rel: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// State required for `WebFinger` handler.
#[derive(Clone)]
pub struct WebfingerState {
    pub domain: String,
    pub base_url: Url,
    pub user_repo: UserRepository,
    pub channel_repo: Option<ChannelRepository>,
}

impl WebfingerState {
    /// Create a new `WebFinger` state.
    #[must_use]
    pub const fn new(domain: String, base_url: Url, user_repo: UserRepository) -> Self {
        Self {
            domain,
            base_url,
            user_repo,
            channel_repo: None,
        }
    }

    /// Create a new `WebFinger` state with channel support.
    #[must_use]
    pub const fn with_channels(
        domain: String,
        base_url: Url,
        user_repo: UserRepository,
        channel_repo: ChannelRepository,
    ) -> Self {
        Self {
            domain,
            base_url,
            user_repo,
            channel_repo: Some(channel_repo),
        }
    }
}

/// Resource type for `WebFinger` lookup.
#[derive(Debug)]
pub enum ResourceType {
    /// User account (acct:username@domain)
    User { username: String, domain: String },
    /// Channel (acct:!channelname@domain or channel:channelname@domain)
    Channel { name: String, domain: String },
}

/// Parse resource URI into resource type.
fn parse_resource(resource: &str) -> Option<ResourceType> {
    // Try acct: prefix first
    if let Some(resource) = resource.strip_prefix("acct:") {
        // Check for channel prefix (!)
        if let Some(channel_part) = resource.strip_prefix('!') {
            let parts: Vec<&str> = channel_part.split('@').collect();
            if parts.len() == 2 {
                return Some(ResourceType::Channel {
                    name: parts[0].to_string(),
                    domain: parts[1].to_string(),
                });
            }
        }

        // Regular user
        let parts: Vec<&str> = resource.split('@').collect();
        if parts.len() == 2 {
            return Some(ResourceType::User {
                username: parts[0].to_string(),
                domain: parts[1].to_string(),
            });
        }
    }

    // Try channel: prefix
    if let Some(resource) = resource.strip_prefix("channel:") {
        let parts: Vec<&str> = resource.split('@').collect();
        if parts.len() == 2 {
            return Some(ResourceType::Channel {
                name: parts[0].to_string(),
                domain: parts[1].to_string(),
            });
        }
    }

    None
}

/// Handle `WebFinger` requests.
///
/// `WebFinger` is used to discover `ActivityPub` actors from their username.
/// Example: `/.well-known/webfinger?resource=acct:user@example.com`
/// For channels: `/.well-known/webfinger?resource=acct:!channelname@example.com`
pub async fn webfinger_handler(
    State(state): State<WebfingerState>,
    Query(query): Query<WebfingerQuery>,
) -> impl IntoResponse {
    info!(resource = %query.resource, "WebFinger lookup");

    // Parse the resource
    let resource = match parse_resource(&query.resource) {
        Some(r) => r,
        None => {
            return (StatusCode::BAD_REQUEST, "Invalid resource format").into_response();
        }
    };

    match resource {
        ResourceType::User { username, domain } => {
            webfinger_user(&state, &query.resource, &username, &domain).await
        }
        ResourceType::Channel { name, domain } => {
            webfinger_channel(&state, &query.resource, &name, &domain).await
        }
    }
}

/// Handle `WebFinger` lookup for users.
async fn webfinger_user(
    state: &WebfingerState,
    resource: &str,
    username: &str,
    domain: &str,
) -> axum::response::Response {
    // Check if the domain matches
    if domain != state.domain {
        return (StatusCode::NOT_FOUND, "Unknown domain").into_response();
    }

    // Look up user in database (local users only, so host is None)
    let user = match state
        .user_repo
        .find_by_username_and_host(username, None)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            info!(username = %username, "User not found for WebFinger");
            return (StatusCode::NOT_FOUND, "User not found").into_response();
        }
        Err(e) => {
            warn!(error = %e, "Database error during WebFinger lookup");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Check if user is suspended
    if user.is_suspended {
        return (StatusCode::GONE, "User is suspended").into_response();
    }

    // Build actor URL using user ID for consistency
    let actor_url = state
        .base_url
        .join(&format!("/users/{}", user.id))
        .expect("valid URL");

    let response = WebfingerResponse {
        subject: resource.to_string(),
        aliases: vec![
            actor_url.to_string(),
            format!("{}/@{}", state.base_url, username),
        ],
        links: vec![
            WebfingerLink {
                rel: "self".to_string(),
                link_type: Some("application/activity+json".to_string()),
                href: Some(actor_url.to_string()),
                template: None,
            },
            WebfingerLink {
                rel: "http://webfinger.net/rel/profile-page".to_string(),
                link_type: Some("text/html".to_string()),
                href: Some(format!("{}/@{}", state.base_url, username)),
                template: None,
            },
        ],
    };

    (
        StatusCode::OK,
        [("Content-Type", "application/jrd+json")],
        Json(response),
    )
        .into_response()
}

/// Handle `WebFinger` lookup for channels.
async fn webfinger_channel(
    state: &WebfingerState,
    resource: &str,
    name: &str,
    domain: &str,
) -> axum::response::Response {
    // Check if the domain matches
    if domain != state.domain {
        return (StatusCode::NOT_FOUND, "Unknown domain").into_response();
    }

    // Check if channel repository is available
    let channel_repo = match &state.channel_repo {
        Some(repo) => repo,
        None => {
            return (StatusCode::NOT_FOUND, "Channel federation not enabled").into_response();
        }
    };

    // Search for channel by name (case-insensitive)
    let channels = match channel_repo.search(name, 1, 0).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Database error during channel WebFinger lookup");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Find exact match
    let channel = channels
        .into_iter()
        .find(|c| c.name.eq_ignore_ascii_case(name) && c.host.is_none());

    let channel = if let Some(c) = channel { c } else {
        info!(name = %name, "Channel not found for WebFinger");
        return (StatusCode::NOT_FOUND, "Channel not found").into_response();
    };

    // Check if channel has federation enabled
    if channel.uri.is_none() {
        return (StatusCode::NOT_FOUND, "Channel federation not enabled").into_response();
    }

    // Check if channel is archived
    if channel.is_archived {
        return (StatusCode::GONE, "Channel is archived").into_response();
    }

    // Build actor URL using channel ID
    let actor_url = state
        .base_url
        .join(&format!("/channels/{}", channel.id))
        .expect("valid URL");

    let response = WebfingerResponse {
        subject: resource.to_string(),
        aliases: vec![
            actor_url.to_string(),
            format!("{}/channels/{}", state.base_url, channel.id),
        ],
        links: vec![
            WebfingerLink {
                rel: "self".to_string(),
                link_type: Some("application/activity+json".to_string()),
                href: Some(actor_url.to_string()),
                template: None,
            },
            WebfingerLink {
                rel: "http://webfinger.net/rel/profile-page".to_string(),
                link_type: Some("text/html".to_string()),
                href: Some(format!("{}/channels/{}", state.base_url, channel.id)),
                template: None,
            },
        ],
    };

    (
        StatusCode::OK,
        [("Content-Type", "application/jrd+json")],
        Json(response),
    )
        .into_response()
}
