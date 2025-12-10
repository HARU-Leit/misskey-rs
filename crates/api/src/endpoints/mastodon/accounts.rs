//! Mastodon accounts API.
//!
//! Provides account-related endpoints for Mastodon compatibility.

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use misskey_common::AppResult;
use serde::Serialize;

use crate::{extractors::AuthUser, middleware::AppState};

use super::statuses::{Account, Field};

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

/// GET /`api/v1/accounts/verify_credentials` - Get current user.
async fn verify_credentials(
    AuthUser(user): AuthUser,
    State(_state): State<AppState>,
) -> AppResult<Json<CredentialAccount>> {
    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let account = Account {
        id: user.id.clone(),
        username: user.username.clone(),
        acct: user.username.clone(),
        display_name: user.name.clone().unwrap_or_else(|| user.username.clone()),
        locked: user.is_locked,
        bot: user.is_bot,
        created_at: user.created_at.to_rfc3339(),
        note: String::new(),
        url: format!("{}/users/{}", base_url, user.id),
        avatar: user.avatar_url.clone().unwrap_or_default(),
        avatar_static: user.avatar_url.clone().unwrap_or_default(),
        header: user.banner_url.clone().unwrap_or_default(),
        header_static: user.banner_url.clone().unwrap_or_default(),
        followers_count: user.followers_count,
        following_count: user.following_count,
        statuses_count: user.notes_count,
        last_status_at: None,
        emojis: vec![],
        fields: vec![],
    };

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

    let account = Account {
        id: user.id.clone(),
        username: user.username.clone(),
        acct: user.username.clone(),
        display_name: user.name.clone().unwrap_or_else(|| user.username.clone()),
        locked: user.is_locked,
        bot: user.is_bot,
        created_at: user.created_at.to_rfc3339(),
        note: String::new(),
        url: format!("{}/users/{}", base_url, user.id),
        avatar: user.avatar_url.clone().unwrap_or_default(),
        avatar_static: user.avatar_url.clone().unwrap_or_default(),
        header: user.banner_url.clone().unwrap_or_default(),
        header_static: user.banner_url.clone().unwrap_or_default(),
        followers_count: user.followers_count,
        following_count: user.following_count,
        statuses_count: user.notes_count,
        last_status_at: None,
        emojis: vec![],
        fields: vec![],
    };

    Ok(Json(account))
}

/// Create the accounts router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/verify_credentials", get(verify_credentials))
        .route("/{id}", get(get_account))
}
