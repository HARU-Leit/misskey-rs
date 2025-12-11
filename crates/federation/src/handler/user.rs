//! `ActivityPub` user (Person) endpoint handler.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use misskey_db::repositories::{UserKeypairRepository, UserRepository};
use tracing::{error, info};
use url::Url;

use crate::convert::{UrlConfig, UserToApPerson};

/// State required for user `ActivityPub` handler.
#[derive(Clone)]
pub struct UserApState {
    pub user_repo: UserRepository,
    pub keypair_repo: UserKeypairRepository,
    pub url_config: UrlConfig,
}

impl UserApState {
    /// Create a new user AP state.
    #[must_use]
    pub const fn new(
        user_repo: UserRepository,
        keypair_repo: UserKeypairRepository,
        base_url: Url,
    ) -> Self {
        Self {
            user_repo,
            keypair_repo,
            url_config: UrlConfig::new(base_url),
        }
    }
}

/// Handle GET /users/{id} for `ActivityPub` Person retrieval.
///
/// Returns the user as an `ActivityPub` Person object with their public key.
pub async fn user_handler(
    State(state): State<UserApState>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    info!(user_id = %user_id, "ActivityPub user lookup");

    // Find user
    let user = match state.user_repo.find_by_id(&user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            info!(user_id = %user_id, "User not found");
            return (StatusCode::NOT_FOUND, "User not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch user");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Check if user is local (no host)
    if user.host.is_some() {
        return (
            StatusCode::GONE,
            "Remote users should be fetched from their origin server",
        )
            .into_response();
    }

    // Check if user is suspended
    if user.is_suspended {
        return (StatusCode::GONE, "User is suspended").into_response();
    }

    // Get public key
    let public_key_pem = match state.keypair_repo.find_by_user_id(&user_id).await {
        Ok(Some(keypair)) => Some(keypair.public_key),
        Ok(None) => {
            error!(user_id = %user_id, "Keypair not found for local user");
            None
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch keypair");
            None
        }
    };

    // Convert to ActivityPub Person
    let person = user.to_ap_person(&state.url_config, public_key_pem.as_deref());

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(person),
    )
        .into_response()
}

/// Handle GET /users/{id} by username (alternative route).
pub async fn user_by_username_handler(
    State(state): State<UserApState>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    info!(username = %username, "ActivityPub user lookup by username");

    // Find user by username (local users only)
    let user = match state
        .user_repo
        .find_by_username_and_host(&username, None)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            info!(username = %username, "User not found");
            return (StatusCode::NOT_FOUND, "User not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch user");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Check if user is suspended
    if user.is_suspended {
        return (StatusCode::GONE, "User is suspended").into_response();
    }

    // Get public key
    let public_key_pem = match state.keypair_repo.find_by_user_id(&user.id).await {
        Ok(Some(keypair)) => Some(keypair.public_key),
        Ok(None) => {
            error!(user_id = %user.id, "Keypair not found for local user");
            None
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch keypair");
            None
        }
    };

    // Convert to ActivityPub Person
    let person = user.to_ap_person(&state.url_config, public_key_pem.as_deref());

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(person),
    )
        .into_response()
}
