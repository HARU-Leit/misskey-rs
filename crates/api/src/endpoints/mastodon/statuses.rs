//! Mastodon statuses API.
//!
//! Provides status-related endpoints for Mastodon compatibility.
//!
//! Endpoints:
//! - POST /api/v1/statuses - Create a new status
//! - GET /api/v1/statuses/:id - Get a status
//! - DELETE /api/v1/statuses/:id - Delete a status
//! - GET /api/v1/statuses/:id/context - Get status context (ancestors/descendants)
//! - POST /api/v1/statuses/:id/favourite - Favourite a status
//! - POST /api/v1/statuses/:id/unfavourite - Unfavourite a status
//! - POST /api/v1/statuses/:id/reblog - Reblog a status
//! - POST /api/v1/statuses/:id/unreblog - Unreblog a status
//! - POST /api/v1/statuses/:id/bookmark - Bookmark a status
//! - POST /api/v1/statuses/:id/unbookmark - Unbookmark a status

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use misskey_common::{AppError, AppResult};
use misskey_core::note::CreateNoteInput;
use misskey_db::entities::{note, user};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState};

/// Mastodon status (toot) response.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub id: String,
    pub created_at: String,
    pub in_reply_to_id: Option<String>,
    pub in_reply_to_account_id: Option<String>,
    pub sensitive: bool,
    pub spoiler_text: String,
    pub visibility: String,
    pub language: Option<String>,
    pub uri: String,
    pub url: Option<String>,
    pub replies_count: i32,
    pub reblogs_count: i32,
    pub favourites_count: i32,
    pub content: String,
    pub reblog: Option<Box<Status>>,
    pub account: Account,
    pub media_attachments: Vec<MediaAttachment>,
    pub mentions: Vec<Mention>,
    pub tags: Vec<Tag>,
    pub emojis: Vec<CustomEmoji>,
    pub card: Option<PreviewCard>,
    pub poll: Option<Poll>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favourited: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bookmarked: Option<bool>,
}

/// Mastodon account (user) in status.
#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub acct: String,
    pub display_name: String,
    pub locked: bool,
    pub bot: bool,
    pub created_at: String,
    pub note: String,
    pub url: String,
    pub avatar: String,
    pub avatar_static: String,
    pub header: String,
    pub header_static: String,
    pub followers_count: i32,
    pub following_count: i32,
    pub statuses_count: i32,
    pub last_status_at: Option<String>,
    pub emojis: Vec<CustomEmoji>,
    pub fields: Vec<Field>,
}

/// Media attachment.
#[derive(Debug, Clone, Serialize)]
pub struct MediaAttachment {
    pub id: String,
    #[serde(rename = "type")]
    pub media_type: String,
    pub url: String,
    pub preview_url: Option<String>,
    pub remote_url: Option<String>,
    pub description: Option<String>,
    pub blurhash: Option<String>,
}

/// Mention in status.
#[derive(Debug, Clone, Serialize)]
pub struct Mention {
    pub id: String,
    pub username: String,
    pub url: String,
    pub acct: String,
}

/// Hashtag in status.
#[derive(Debug, Clone, Serialize)]
pub struct Tag {
    pub name: String,
    pub url: String,
}

/// Custom emoji.
#[derive(Debug, Clone, Serialize)]
pub struct CustomEmoji {
    pub shortcode: String,
    pub url: String,
    pub static_url: String,
    pub visible_in_picker: bool,
}

/// Preview card for links.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewCard {
    pub url: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub image: Option<String>,
}

/// Poll.
#[derive(Debug, Clone, Serialize)]
pub struct Poll {
    pub id: String,
    pub expires_at: Option<String>,
    pub expired: bool,
    pub multiple: bool,
    pub votes_count: i32,
    pub voters_count: Option<i32>,
    pub options: Vec<PollOption>,
    pub voted: Option<bool>,
    pub own_votes: Option<Vec<i32>>,
}

/// Poll option.
#[derive(Debug, Clone, Serialize)]
pub struct PollOption {
    pub title: String,
    pub votes_count: Option<i32>,
}

/// Account field.
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub verified_at: Option<String>,
}

/// Create status request.
#[derive(Debug, Deserialize)]
pub struct CreateStatusRequest {
    pub status: Option<String>,
    pub media_ids: Option<Vec<String>>,
    pub poll: Option<CreatePollRequest>,
    pub in_reply_to_id: Option<String>,
    pub sensitive: Option<bool>,
    pub spoiler_text: Option<String>,
    pub visibility: Option<String>,
    pub language: Option<String>,
    pub scheduled_at: Option<String>,
}

/// Create poll request.
#[derive(Debug, Deserialize)]
pub struct CreatePollRequest {
    pub options: Vec<String>,
    pub expires_in: i32,
    pub multiple: Option<bool>,
    pub hide_totals: Option<bool>,
}

/// Status context response.
#[derive(Debug, Serialize)]
pub struct StatusContext {
    pub ancestors: Vec<Status>,
    pub descendants: Vec<Status>,
}

/// Convert Mastodon visibility to Misskey visibility.
fn mastodon_to_misskey_visibility(visibility: &str) -> note::Visibility {
    match visibility {
        "public" => note::Visibility::Public,
        "unlisted" => note::Visibility::Home,
        "private" => note::Visibility::Followers,
        "direct" => note::Visibility::Specified,
        _ => note::Visibility::Public,
    }
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

/// Convert user to Mastodon account.
#[allow(clippy::unwrap_used)] // unwrap is safe inside is_some() check
pub fn user_to_account(user: &user::Model, base_url: &str) -> Account {
    Account {
        id: user.id.clone(),
        username: user.username.clone(),
        acct: if user.host.is_some() {
            format!("{}@{}", user.username, user.host.as_ref().unwrap())
        } else {
            user.username.clone()
        },
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
    }
}

/// Convert note to Mastodon status.
pub fn note_to_status(note: note::Model, author: Option<&user::Model>, base_url: &str) -> Status {
    let visibility = misskey_to_mastodon_visibility(&note.visibility);

    let account = if let Some(user) = author {
        user_to_account(user, base_url)
    } else {
        // Fallback account info
        Account {
            id: note.user_id.clone(),
            username: "unknown".to_string(),
            acct: "unknown".to_string(),
            display_name: "Unknown".to_string(),
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
        }
    };

    Status {
        id: note.id.clone(),
        created_at: note.created_at.to_rfc3339(),
        in_reply_to_id: note.reply_id.clone(),
        in_reply_to_account_id: None, // Would need additional lookup
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
        account,
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

/// POST /api/v1/statuses - Create a new status.
async fn create_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateStatusRequest>,
) -> AppResult<Json<Status>> {
    let visibility = req
        .visibility
        .as_deref()
        .map_or(note::Visibility::Public, mastodon_to_misskey_visibility);

    let input = CreateNoteInput {
        text: req.status,
        cw: req.spoiler_text,
        visibility,
        reply_id: req.in_reply_to_id,
        renote_id: None,
        file_ids: req.media_ids.unwrap_or_default(),
        visible_user_ids: vec![],
        channel_id: None,
    };

    let note = state.note_service.create(&user.id, input).await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let status = note_to_status(note, Some(&user), base_url);

    Ok(Json(status))
}

/// GET /api/v1/statuses/:id - Get a status.
async fn get_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    let note = state.note_service.get(&id).await?;
    let author = state.user_service.get(&note.user_id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let status = note_to_status(note, author.as_ref(), base_url);

    Ok(Json(status))
}

/// DELETE /api/v1/statuses/:id - Delete a status.
async fn delete_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    let note = state.note_service.get(&id).await?;

    // Verify ownership
    if note.user_id != user.id {
        return Err(AppError::Forbidden("Not your status".to_string()));
    }

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let status = note_to_status(note, Some(&user), base_url);

    state.note_service.delete(&user.id, &id).await?;

    Ok(Json(status))
}

/// GET /api/v1/statuses/:id/context - Get status context.
async fn get_status_context(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<StatusContext>> {
    let note = state.note_service.get(&id).await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    // Get ancestors (parent chain)
    let mut ancestors = Vec::new();
    let mut current_id = note.reply_id.clone();
    while let Some(parent_id) = current_id {
        if let Ok(parent) = state.note_service.get(&parent_id).await {
            let author = state.user_service.get(&parent.user_id).await.ok();
            current_id = parent.reply_id.clone();
            ancestors.push(note_to_status(parent, author.as_ref(), base_url));
        } else {
            break;
        }
    }
    ancestors.reverse();

    // Get descendants (replies)
    let replies = state.note_service.get_replies(&id, 50).await?;
    let mut descendants = Vec::new();
    for reply in replies {
        let author = state.user_service.get(&reply.user_id).await.ok();
        descendants.push(note_to_status(reply, author.as_ref(), base_url));
    }

    Ok(Json(StatusContext {
        ancestors,
        descendants,
    }))
}

/// POST /api/v1/statuses/:id/favourite - Favourite a status.
async fn favourite_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    // Add to favorites
    state.note_favorite_service.create(&user.id, &id).await?;

    let note = state.note_service.get(&id).await?;
    let author = state.user_service.get(&note.user_id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let mut status = note_to_status(note, author.as_ref(), base_url);
    status.favourited = Some(true);

    Ok(Json(status))
}

/// POST /api/v1/statuses/:id/unfavourite - Unfavourite a status.
async fn unfavourite_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    // Remove from favorites
    state.note_favorite_service.delete(&user.id, &id).await?;

    let note = state.note_service.get(&id).await?;
    let author = state.user_service.get(&note.user_id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let mut status = note_to_status(note, author.as_ref(), base_url);
    status.favourited = Some(false);

    Ok(Json(status))
}

/// POST /api/v1/statuses/:id/reblog - Reblog a status.
async fn reblog_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    // Create a renote
    let input = CreateNoteInput {
        text: None,
        cw: None,
        visibility: note::Visibility::Public,
        reply_id: None,
        renote_id: Some(id.clone()),
        file_ids: vec![],
        visible_user_ids: vec![],
        channel_id: None,
    };

    let renote = state.note_service.create(&user.id, input).await?;

    // Get the original note
    let original_note = state.note_service.get(&id).await?;
    let original_author = state.user_service.get(&original_note.user_id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";

    let mut original_status = note_to_status(original_note, original_author.as_ref(), base_url);
    original_status.reblogged = Some(true);

    // Return the reblog status with the original embedded
    let mut reblog_status = note_to_status(renote, Some(&user), base_url);
    reblog_status.reblog = Some(Box::new(original_status));

    Ok(Json(reblog_status))
}

/// POST /api/v1/statuses/:id/unreblog - Unreblog a status.
async fn unreblog_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    // Find the user's renote of this note and delete it
    let renotes = state.note_service.get_renotes(&id, 100).await?;
    for renote in renotes {
        if renote.user_id == user.id && renote.text.is_none() {
            // This is a pure renote (not a quote) by this user
            let _ = state.note_service.delete(&renote.id, &user.id).await;
            break;
        }
    }

    let note = state.note_service.get(&id).await?;
    let author = state.user_service.get(&note.user_id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let mut status = note_to_status(note, author.as_ref(), base_url);
    status.reblogged = Some(false);

    Ok(Json(status))
}

/// POST /api/v1/statuses/:id/bookmark - Bookmark a status.
async fn bookmark_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    // Use clips as bookmarks (create a default bookmark clip if needed)
    let clips = state.clip_service.list_my_clips(&user.id, 100, 0).await?;
    let bookmark_clip = if let Some(clip) = clips.into_iter().find(|c| c.name == "Bookmarks") {
        clip
    } else {
        state
            .clip_service
            .create(
                &user.id,
                "Bookmarks".to_string(),
                Some("Mastodon API bookmarks".to_string()),
                false,
            )
            .await?
    };

    // add_note(clip_id, note_id, user_id, comment)
    let _ = state
        .clip_service
        .add_note(&bookmark_clip.id, &id, &user.id, None)
        .await;

    let note = state.note_service.get(&id).await?;
    let author = state.user_service.get(&note.user_id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let mut status = note_to_status(note, author.as_ref(), base_url);
    status.bookmarked = Some(true);

    Ok(Json(status))
}

/// POST /api/v1/statuses/:id/unbookmark - Unbookmark a status.
async fn unbookmark_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Status>> {
    // Find bookmark clip and remove note
    let clips = state.clip_service.list_my_clips(&user.id, 100, 0).await?;
    if let Some(clip) = clips.into_iter().find(|c| c.name == "Bookmarks") {
        // remove_note(clip_id, note_id, user_id)
        let _ = state
            .clip_service
            .remove_note(&clip.id, &id, &user.id)
            .await;
    }

    let note = state.note_service.get(&id).await?;
    let author = state.user_service.get(&note.user_id).await.ok();

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let mut status = note_to_status(note, author.as_ref(), base_url);
    status.bookmarked = Some(false);

    Ok(Json(status))
}

/// Create the statuses router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_status))
        .route("/{id}", get(get_status).delete(delete_status))
        .route("/{id}/context", get(get_status_context))
        .route("/{id}/favourite", post(favourite_status))
        .route("/{id}/unfavourite", post(unfavourite_status))
        .route("/{id}/reblog", post(reblog_status))
        .route("/{id}/unreblog", post(unreblog_status))
        .route("/{id}/bookmark", post(bookmark_status))
        .route("/{id}/unbookmark", post(unbookmark_status))
}
