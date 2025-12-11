//! Mastodon accounts API.
//!
//! Provides account-related endpoints for Mastodon compatibility.
//!
//! Endpoints:
//! - GET /api/v1/accounts/verify_credentials - Get current user
//! - GET /api/v1/accounts/:id - Get account by ID
//! - GET /api/v1/accounts/:id/followers - Get account followers
//! - GET /api/v1/accounts/:id/following - Get account following
//! - GET /api/v1/accounts/:id/statuses - Get account statuses
//! - POST /api/v1/accounts/:id/follow - Follow account
//! - POST /api/v1/accounts/:id/unfollow - Unfollow account
//! - POST /api/v1/accounts/:id/block - Block account
//! - POST /api/v1/accounts/:id/unblock - Unblock account
//! - POST /api/v1/accounts/:id/mute - Mute account
//! - POST /api/v1/accounts/:id/unmute - Unmute account
//! - GET /api/v1/accounts/relationships - Get relationships with accounts

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState};

use super::statuses::{Account, Field, Status, note_to_status, user_to_account};

/// Credential account (current user) response.
#[derive(Debug, Serialize)]
pub struct CredentialAccount {
    #[serde(flatten)]
    pub account: Account,
    pub source: AccountSource,
}

/// Account source (editable fields).
#[derive(Debug, Serialize)]
pub struct AccountSource {
    pub privacy: String,
    pub sensitive: bool,
    pub language: String,
    pub note: String,
    pub fields: Vec<Field>,
}

/// Account relationship.
#[derive(Debug, Serialize)]
pub struct Relationship {
    pub id: String,
    pub following: bool,
    pub showing_reblogs: bool,
    pub notifying: bool,
    pub followed_by: bool,
    pub blocking: bool,
    pub blocked_by: bool,
    pub muting: bool,
    pub muting_notifications: bool,
    pub requested: bool,
    pub domain_blocking: bool,
    pub endorsed: bool,
    pub note: String,
}

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

/// Statuses query parameters.
#[derive(Debug, Deserialize)]
pub struct StatusesQuery {
    pub max_id: Option<String>,
    pub since_id: Option<String>,
    pub min_id: Option<String>,
    #[serde(default = "default_statuses_limit")]
    pub limit: u64,
    #[allow(dead_code)]
    pub only_media: Option<bool>,
    #[allow(dead_code)]
    pub exclude_replies: Option<bool>,
    #[allow(dead_code)]
    pub exclude_reblogs: Option<bool>,
    #[allow(dead_code)]
    pub pinned: Option<bool>,
    #[allow(dead_code)]
    pub tagged: Option<String>,
}

fn default_statuses_limit() -> u64 {
    20
}

/// Relationships query parameters.
#[derive(Debug, Deserialize)]
pub struct RelationshipsQuery {
    #[serde(rename = "id[]")]
    pub id: Option<Vec<String>>,
}

/// GET /`api/v1/accounts/verify_credentials` - Get current user.
async fn verify_credentials(
    AuthUser(user): AuthUser,
    State(_state): State<AppState>,
) -> AppResult<Json<CredentialAccount>> {
    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let account = user_to_account(&user, base_url);

    Ok(Json(CredentialAccount {
        account,
        source: AccountSource {
            privacy: "public".to_string(),
            sensitive: false,
            language: "en".to_string(),
            note: String::new(),
            fields: vec![],
        },
    }))
}

/// GET /api/v1/accounts/:id - Get account by ID.
async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Account>> {
    let user = state.user_service.get(&id).await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let account = user_to_account(&user, base_url);

    Ok(Json(account))
}

/// GET /api/v1/accounts/:id/followers - Get account followers.
async fn get_account_followers(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Account>>> {
    let limit = query.limit.min(80);

    let followers = state
        .following_service
        .get_followers(&id, limit, query.max_id.as_deref())
        .await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let mut accounts = Vec::new();
    for follower in followers {
        if let Ok(user) = state.user_service.get(&follower.follower_id).await {
            accounts.push(user_to_account(&user, base_url));
        }
    }

    Ok(Json(accounts))
}

/// GET /api/v1/accounts/:id/following - Get account following.
async fn get_account_following(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Account>>> {
    let limit = query.limit.min(80);

    let following = state
        .following_service
        .get_following(&id, limit, query.max_id.as_deref())
        .await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let mut accounts = Vec::new();
    for follow in following {
        if let Ok(user) = state.user_service.get(&follow.followee_id).await {
            accounts.push(user_to_account(&user, base_url));
        }
    }

    Ok(Json(accounts))
}

/// GET /api/v1/accounts/:id/statuses - Get account statuses.
async fn get_account_statuses(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<StatusesQuery>,
) -> AppResult<Json<Vec<Status>>> {
    let limit = query.limit.min(40);

    // Get user notes using user_notes method
    let notes = state
        .note_service
        .user_notes(&id, limit, query.max_id.as_deref())
        .await?;

    // Get user for account info
    let user = state.user_service.get(&id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let statuses: Vec<Status> = notes
        .into_iter()
        .map(|note| note_to_status(note, user.as_ref(), base_url))
        .collect();

    Ok(Json(statuses))
}

/// POST /api/v1/accounts/:id/follow - Follow an account.
async fn follow_account(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Relationship>> {
    // Use follow method instead of create
    let _ = state.following_service.follow(&user.id, &id).await?;

    let relationship = build_relationship(&state, &user.id, &id).await?;
    Ok(Json(relationship))
}

/// POST /api/v1/accounts/:id/unfollow - Unfollow an account.
async fn unfollow_account(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Relationship>> {
    // Use unfollow method instead of delete
    let _ = state.following_service.unfollow(&user.id, &id).await;

    let relationship = build_relationship(&state, &user.id, &id).await?;
    Ok(Json(relationship))
}

/// POST /api/v1/accounts/:id/block - Block an account.
async fn block_account(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Relationship>> {
    // Use block method instead of create
    let _ = state.blocking_service.block(&user.id, &id).await?;

    let relationship = build_relationship(&state, &user.id, &id).await?;
    Ok(Json(relationship))
}

/// POST /api/v1/accounts/:id/unblock - Unblock an account.
async fn unblock_account(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Relationship>> {
    // Use unblock method instead of delete
    let _ = state.blocking_service.unblock(&user.id, &id).await;

    let relationship = build_relationship(&state, &user.id, &id).await?;
    Ok(Json(relationship))
}

/// POST /api/v1/accounts/:id/mute - Mute an account.
async fn mute_account(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Relationship>> {
    // Use mute method (None for permanent mute)
    let _ = state.muting_service.mute(&user.id, &id, None).await?;

    let relationship = build_relationship(&state, &user.id, &id).await?;
    Ok(Json(relationship))
}

/// POST /api/v1/accounts/:id/unmute - Unmute an account.
async fn unmute_account(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Relationship>> {
    // Use unmute method instead of delete
    let _ = state.muting_service.unmute(&user.id, &id).await;

    let relationship = build_relationship(&state, &user.id, &id).await?;
    Ok(Json(relationship))
}

/// GET /api/v1/accounts/relationships - Get relationships with accounts.
async fn get_relationships(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<RelationshipsQuery>,
) -> AppResult<Json<Vec<Relationship>>> {
    let ids = query.id.unwrap_or_default();

    let mut relationships = Vec::new();
    for id in ids {
        let relationship = build_relationship(&state, &user.id, &id).await?;
        relationships.push(relationship);
    }

    Ok(Json(relationships))
}

/// Build a relationship object for a user pair.
async fn build_relationship(
    state: &AppState,
    user_id: &str,
    target_id: &str,
) -> AppResult<Relationship> {
    let following = state
        .following_service
        .is_following(user_id, target_id)
        .await
        .unwrap_or(false);

    let followed_by = state
        .following_service
        .is_following(target_id, user_id)
        .await
        .unwrap_or(false);

    let blocking = state
        .blocking_service
        .is_blocking(user_id, target_id)
        .await
        .unwrap_or(false);

    let blocked_by = state
        .blocking_service
        .is_blocking(target_id, user_id)
        .await
        .unwrap_or(false);

    let muting = state
        .muting_service
        .is_muting(user_id, target_id)
        .await
        .unwrap_or(false);

    // Check pending requests by looking at pending_requests list
    let pending_requests = state
        .following_service
        .get_pending_requests(user_id, 100, None)
        .await
        .unwrap_or_default();
    let requested = pending_requests.iter().any(|r| r.followee_id == target_id);

    Ok(Relationship {
        id: target_id.to_string(),
        following,
        showing_reblogs: true,
        notifying: false,
        followed_by,
        blocking,
        blocked_by,
        muting,
        muting_notifications: false,
        requested,
        domain_blocking: false,
        endorsed: false,
        note: String::new(),
    })
}

/// Create the accounts router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/verify_credentials", get(verify_credentials))
        .route("/relationships", get(get_relationships))
        .route("/{id}", get(get_account))
        .route("/{id}/followers", get(get_account_followers))
        .route("/{id}/following", get(get_account_following))
        .route("/{id}/statuses", get(get_account_statuses))
        .route("/{id}/follow", post(follow_account))
        .route("/{id}/unfollow", post(unfollow_account))
        .route("/{id}/block", post(block_account))
        .route("/{id}/unblock", post(unblock_account))
        .route("/{id}/mute", post(mute_account))
        .route("/{id}/unmute", post(unmute_account))
}
