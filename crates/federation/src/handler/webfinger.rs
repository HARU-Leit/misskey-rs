//! `WebFinger` handler for actor discovery.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use misskey_db::repositories::UserRepository;
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
}

impl WebfingerState {
    /// Create a new `WebFinger` state.
    #[must_use] 
    pub const fn new(domain: String, base_url: Url, user_repo: UserRepository) -> Self {
        Self {
            domain,
            base_url,
            user_repo,
        }
    }
}

/// Parse acct: URI into username and domain.
fn parse_acct(resource: &str) -> Option<(String, String)> {
    let resource = resource.strip_prefix("acct:")?;
    let parts: Vec<&str> = resource.split('@').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

/// Handle `WebFinger` requests.
///
/// `WebFinger` is used to discover `ActivityPub` actors from their username.
/// Example: `/.well-known/webfinger?resource=acct:user@example.com`
pub async fn webfinger_handler(
    State(state): State<WebfingerState>,
    Query(query): Query<WebfingerQuery>,
) -> impl IntoResponse {
    info!(resource = %query.resource, "WebFinger lookup");

    // Parse the resource
    let (username, domain) = match parse_acct(&query.resource) {
        Some((u, d)) => (u, d),
        None => {
            return (StatusCode::BAD_REQUEST, "Invalid resource format").into_response();
        }
    };

    // Check if the domain matches
    if domain != state.domain {
        return (StatusCode::NOT_FOUND, "Unknown domain").into_response();
    }

    // Look up user in database (local users only, so host is None)
    let user = match state
        .user_repo
        .find_by_username_and_host(&username, None)
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
        subject: query.resource.clone(),
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
