//! Mastodon statuses API.
//!
//! Provides POST /api/v1/statuses for creating posts.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::note::CreateNoteInput;
use misskey_db::entities::note;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState};

/// Mastodon status (toot) response.
#[derive(Debug, Serialize)]
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
}

/// Mastodon account (user) in status.
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct Mention {
    pub id: String,
    pub username: String,
    pub url: String,
    pub acct: String,
}

/// Hashtag in status.
#[derive(Debug, Serialize)]
pub struct Tag {
    pub name: String,
    pub url: String,
}

/// Custom emoji.
#[derive(Debug, Serialize)]
pub struct CustomEmoji {
    pub shortcode: String,
    pub url: String,
    pub static_url: String,
    pub visible_in_picker: bool,
}

/// Preview card for links.
#[derive(Debug, Serialize)]
pub struct PreviewCard {
    pub url: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub image: Option<String>,
}

/// Poll.
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct PollOption {
    pub title: String,
    pub votes_count: Option<i32>,
}

/// Account field.
#[derive(Debug, Serialize)]
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

/// Convert Mastodon visibility to Misskey visibility.
fn mastodon_to_misskey_visibility(visibility: &str) -> misskey_db::entities::note::Visibility {
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

/// Convert note to Mastodon status.
fn note_to_status(note: note::Model, base_url: &str) -> Status {
    let visibility = misskey_to_mastodon_visibility(&note.visibility);

    Status {
        id: note.id.clone(),
        created_at: note.created_at.to_rfc3339(),
        in_reply_to_id: note.reply_id.clone(),
        in_reply_to_account_id: None, // Would need to look up
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
            username: "user".to_string(), // Would need to look up
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
    };

    let note = state.note_service.create(&user.id, input).await?;

    // TODO: Get base_url from config
    let base_url = "https://example.com";
    let status = note_to_status(note, base_url);

    Ok(Json(status))
}

/// Create the statuses router.
pub fn router() -> Router<AppState> {
    Router::new().route("/", post(create_status))
}
