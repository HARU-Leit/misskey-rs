//! `ActivityPub` Collection handlers (Outbox, Followers, Following).

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use misskey_db::repositories::{
    ClipRepository, DriveFileRepository, FollowingRepository, NoteRepository, UserRepository,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use url::Url;

use crate::convert::{NoteToApNote, UrlConfig};

/// State required for collection handlers.
#[derive(Clone)]
pub struct CollectionState {
    pub user_repo: UserRepository,
    pub note_repo: NoteRepository,
    pub following_repo: FollowingRepository,
    pub drive_file_repo: DriveFileRepository,
    pub url_config: UrlConfig,
}

impl CollectionState {
    /// Create a new collection state.
    #[must_use]
    pub const fn new(
        user_repo: UserRepository,
        note_repo: NoteRepository,
        following_repo: FollowingRepository,
        drive_file_repo: DriveFileRepository,
        base_url: Url,
    ) -> Self {
        Self {
            user_repo,
            note_repo,
            following_repo,
            drive_file_repo,
            url_config: UrlConfig::new(base_url),
        }
    }
}

/// Query parameters for paginated collections.
#[derive(Debug, Deserialize)]
pub struct CollectionQuery {
    /// Page number (for paging).
    pub page: Option<bool>,
    /// Maximum ID for cursor-based pagination.
    pub max_id: Option<String>,
    /// Minimum number of items per page.
    pub min_id: Option<String>,
}

/// `ActivityPub` `OrderedCollection`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollection {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    #[serde(rename = "type")]
    pub kind: String,
    pub id: Url,
    pub total_items: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<Url>,
}

/// `ActivityPub` `OrderedCollectionPage`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderedCollectionPage {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    #[serde(rename = "type")]
    pub kind: String,
    pub id: Url,
    pub part_of: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Url>,
    pub ordered_items: Vec<serde_json::Value>,
}

/// Default `ActivityStreams` context.
fn activitystreams_context() -> serde_json::Value {
    serde_json::json!([
        "https://www.w3.org/ns/activitystreams",
        {
            "sensitive": "as:sensitive",
            "Hashtag": "as:Hashtag",
            "quoteUrl": "as:quoteUrl",
            "toot": "http://joinmastodon.org/ns#",
            "Emoji": "toot:Emoji",
            "featured": "toot:featured",
            "discoverable": "toot:discoverable",
            "schema": "http://schema.org#",
            "PropertyValue": "schema:PropertyValue",
            "value": "schema:value",
            "misskey": "https://misskey-hub.net/ns#",
            "_misskey_content": "misskey:_misskey_content",
            "_misskey_quote": "misskey:_misskey_quote",
            "_misskey_reaction": "misskey:_misskey_reaction",
            "_misskey_summary": "misskey:_misskey_summary",
            "isCat": "misskey:isCat"
        }
    ])
}

/// Handle GET /users/{username}/outbox - User's outbox collection.
pub async fn outbox_handler(
    State(state): State<CollectionState>,
    Path(username): Path<String>,
    Query(query): Query<CollectionQuery>,
) -> impl IntoResponse {
    info!(username = %username, "ActivityPub outbox lookup");

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

    let outbox_url = state.url_config.outbox_url(&username);

    // If page=true, return a page of activities
    if query.page == Some(true) {
        let limit = 20u64;

        // Get public notes for this user
        let notes = match state
            .note_repo
            .find_public_by_user(&user.id, limit, query.max_id.as_deref())
            .await
        {
            Ok(n) => n,
            Err(e) => {
                error!(error = %e, "Failed to fetch notes");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Convert notes to Create activities
        let mut items: Vec<serde_json::Value> = Vec::new();
        for note in &notes {
            // Get files for this note
            let file_ids: Vec<String> =
                serde_json::from_value(note.file_ids.clone()).unwrap_or_default();
            let files = if file_ids.is_empty() {
                vec![]
            } else {
                state
                    .drive_file_repo
                    .find_by_ids(&file_ids)
                    .await
                    .unwrap_or_default()
            };

            let ap_note = note.to_ap_note(&state.url_config, &username, &files);
            let note_url = state
                .url_config
                .base_url
                .join(&format!("/notes/{}", note.id))
                .expect("valid URL");

            // Wrap in Create activity
            let create_activity = serde_json::json!({
                "type": "Create",
                "id": format!("{}/activity", note_url),
                "actor": state.url_config.user_url(&username).to_string(),
                "published": note.created_at.to_rfc3339(),
                "to": ap_note.to,
                "cc": ap_note.cc,
                "object": ap_note,
            });
            items.push(create_activity);
        }

        // Build page URL
        let mut page_url = outbox_url.clone();
        page_url.set_query(Some("page=true"));

        // Build next page URL if we have more items
        let next = if notes.len() == limit as usize {
            notes.last().map(|n| {
                let mut next_url = outbox_url.clone();
                next_url.set_query(Some(&format!("page=true&max_id={}", n.id)));
                next_url
            })
        } else {
            None
        };

        let page = OrderedCollectionPage {
            context: activitystreams_context(),
            kind: "OrderedCollectionPage".to_string(),
            id: page_url,
            part_of: outbox_url,
            prev: None,
            next,
            ordered_items: items,
        };

        return (
            StatusCode::OK,
            [("Content-Type", "application/activity+json; charset=utf-8")],
            Json(page),
        )
            .into_response();
    }

    // Return collection summary
    let total_items = user.notes_count as u64;
    let first = {
        let mut first_url = outbox_url.clone();
        first_url.set_query(Some("page=true"));
        first_url
    };

    let collection = OrderedCollection {
        context: activitystreams_context(),
        kind: "OrderedCollection".to_string(),
        id: outbox_url,
        total_items,
        first: Some(first),
        last: None,
    };

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(collection),
    )
        .into_response()
}

/// Handle GET /users/{username}/followers - User's followers collection.
pub async fn followers_handler(
    State(state): State<CollectionState>,
    Path(username): Path<String>,
    Query(query): Query<CollectionQuery>,
) -> impl IntoResponse {
    info!(username = %username, "ActivityPub followers lookup");

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

    if user.is_suspended {
        return (StatusCode::GONE, "User is suspended").into_response();
    }

    let followers_url = state.url_config.followers_url(&username);

    // If page=true, return a page of followers
    if query.page == Some(true) {
        let limit = 40u64;

        // Get followers for this user
        let followers = match state
            .following_repo
            .find_followers(&user.id, limit, query.max_id.as_deref())
            .await
        {
            Ok(f) => f,
            Err(e) => {
                error!(error = %e, "Failed to fetch followers");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Get user URIs for followers
        let follower_ids: Vec<String> = followers.iter().map(|f| f.follower_id.clone()).collect();
        let follower_users = match state.user_repo.find_by_ids(&follower_ids).await {
            Ok(u) => u,
            Err(e) => {
                error!(error = %e, "Failed to fetch follower users");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Convert to URIs
        let items: Vec<serde_json::Value> = follower_users
            .iter()
            .map(|u| {
                if let Some(ref uri) = u.uri {
                    serde_json::json!(uri)
                } else {
                    // Local user
                    serde_json::json!(state.url_config.user_url(&u.username).to_string())
                }
            })
            .collect();

        let mut page_url = followers_url.clone();
        page_url.set_query(Some("page=true"));

        let next = if followers.len() == limit as usize {
            followers.last().map(|f| {
                let mut next_url = followers_url.clone();
                next_url.set_query(Some(&format!("page=true&max_id={}", f.id)));
                next_url
            })
        } else {
            None
        };

        let page = OrderedCollectionPage {
            context: activitystreams_context(),
            kind: "OrderedCollectionPage".to_string(),
            id: page_url,
            part_of: followers_url,
            prev: None,
            next,
            ordered_items: items,
        };

        return (
            StatusCode::OK,
            [("Content-Type", "application/activity+json; charset=utf-8")],
            Json(page),
        )
            .into_response();
    }

    // Return collection summary
    let total_items = user.followers_count as u64;
    let first = {
        let mut first_url = followers_url.clone();
        first_url.set_query(Some("page=true"));
        first_url
    };

    let collection = OrderedCollection {
        context: activitystreams_context(),
        kind: "OrderedCollection".to_string(),
        id: followers_url,
        total_items,
        first: Some(first),
        last: None,
    };

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(collection),
    )
        .into_response()
}

/// Handle GET /users/{username}/following - User's following collection.
pub async fn following_handler(
    State(state): State<CollectionState>,
    Path(username): Path<String>,
    Query(query): Query<CollectionQuery>,
) -> impl IntoResponse {
    info!(username = %username, "ActivityPub following lookup");

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

    if user.is_suspended {
        return (StatusCode::GONE, "User is suspended").into_response();
    }

    let following_url = state.url_config.following_url(&username);

    // If page=true, return a page of following
    if query.page == Some(true) {
        let limit = 40u64;

        // Get following for this user
        let following = match state
            .following_repo
            .find_following(&user.id, limit, query.max_id.as_deref())
            .await
        {
            Ok(f) => f,
            Err(e) => {
                error!(error = %e, "Failed to fetch following");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Get user URIs for following
        let following_ids: Vec<String> = following.iter().map(|f| f.followee_id.clone()).collect();
        let following_users = match state.user_repo.find_by_ids(&following_ids).await {
            Ok(u) => u,
            Err(e) => {
                error!(error = %e, "Failed to fetch following users");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Convert to URIs
        let items: Vec<serde_json::Value> = following_users
            .iter()
            .map(|u| {
                if let Some(ref uri) = u.uri {
                    serde_json::json!(uri)
                } else {
                    // Local user
                    serde_json::json!(state.url_config.user_url(&u.username).to_string())
                }
            })
            .collect();

        let mut page_url = following_url.clone();
        page_url.set_query(Some("page=true"));

        let next = if following.len() == limit as usize {
            following.last().map(|f| {
                let mut next_url = following_url.clone();
                next_url.set_query(Some(&format!("page=true&max_id={}", f.id)));
                next_url
            })
        } else {
            None
        };

        let page = OrderedCollectionPage {
            context: activitystreams_context(),
            kind: "OrderedCollectionPage".to_string(),
            id: page_url,
            part_of: following_url,
            prev: None,
            next,
            ordered_items: items,
        };

        return (
            StatusCode::OK,
            [("Content-Type", "application/activity+json; charset=utf-8")],
            Json(page),
        )
            .into_response();
    }

    // Return collection summary
    let total_items = user.following_count as u64;
    let first = {
        let mut first_url = following_url.clone();
        first_url.set_query(Some("page=true"));
        first_url
    };

    let collection = OrderedCollection {
        context: activitystreams_context(),
        kind: "OrderedCollection".to_string(),
        id: following_url,
        total_items,
        first: Some(first),
        last: None,
    };

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(collection),
    )
        .into_response()
}

/// State required for clip collection handler.
#[derive(Clone)]
pub struct ClipCollectionState {
    pub user_repo: UserRepository,
    pub clip_repo: ClipRepository,
    pub note_repo: NoteRepository,
    pub drive_file_repo: DriveFileRepository,
    pub url_config: UrlConfig,
}

impl ClipCollectionState {
    /// Create a new clip collection state.
    #[must_use]
    pub const fn new(
        user_repo: UserRepository,
        clip_repo: ClipRepository,
        note_repo: NoteRepository,
        drive_file_repo: DriveFileRepository,
        base_url: Url,
    ) -> Self {
        Self {
            user_repo,
            clip_repo,
            note_repo,
            drive_file_repo,
            url_config: UrlConfig::new(base_url),
        }
    }
}

/// Handle GET /`users/{username}/clips/{clip_id`} - User's clip as `ActivityPub` Collection.
pub async fn clip_handler(
    State(state): State<ClipCollectionState>,
    Path((username, clip_id)): Path<(String, String)>,
    Query(query): Query<CollectionQuery>,
) -> impl IntoResponse {
    info!(username = %username, clip_id = %clip_id, "ActivityPub clip lookup");

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

    if user.is_suspended {
        return (StatusCode::GONE, "User is suspended").into_response();
    }

    // Find clip by ID
    let clip = match state.clip_repo.find_by_id(&clip_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            info!(clip_id = %clip_id, "Clip not found");
            return (StatusCode::NOT_FOUND, "Clip not found").into_response();
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch clip");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Verify clip belongs to user
    if clip.user_id != user.id {
        return (StatusCode::NOT_FOUND, "Clip not found").into_response();
    }

    // Only public clips are accessible via ActivityPub
    if !clip.is_public {
        return (StatusCode::FORBIDDEN, "Clip is not public").into_response();
    }

    let clip_url = state
        .url_config
        .base_url
        .join(&format!("/users/{username}/clips/{clip_id}"))
        .expect("valid URL");

    // If page=true, return a page of notes in the clip
    if query.page == Some(true) {
        let limit = 20u64;

        // Get notes in this clip
        let clip_notes = match state.clip_repo.find_notes_in_clip(&clip_id, limit, 0).await {
            Ok(n) => n,
            Err(e) => {
                error!(error = %e, "Failed to fetch clip notes");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Get the actual notes
        let note_ids: Vec<String> = clip_notes.iter().map(|cn| cn.note_id.clone()).collect();
        let notes = match state.note_repo.find_by_ids(&note_ids).await {
            Ok(n) => n,
            Err(e) => {
                error!(error = %e, "Failed to fetch notes");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Convert notes to AP objects
        let mut items: Vec<serde_json::Value> = Vec::new();
        for note in &notes {
            // Get files for this note
            let file_ids: Vec<String> =
                serde_json::from_value(note.file_ids.clone()).unwrap_or_default();
            let files = if file_ids.is_empty() {
                vec![]
            } else {
                state
                    .drive_file_repo
                    .find_by_ids(&file_ids)
                    .await
                    .unwrap_or_default()
            };

            // Get note author
            let author_username = match state.user_repo.find_by_id(&note.user_id).await {
                Ok(Some(u)) => u.username,
                _ => "unknown".to_string(),
            };

            let ap_note = note.to_ap_note(&state.url_config, &author_username, &files);
            items.push(serde_json::to_value(&ap_note).unwrap_or_default());
        }

        let mut page_url = clip_url.clone();
        page_url.set_query(Some("page=true"));

        let next = if clip_notes.len() == limit as usize {
            clip_notes.last().map(|cn| {
                let mut next_url = clip_url.clone();
                next_url.set_query(Some(&format!("page=true&max_id={}", cn.id)));
                next_url
            })
        } else {
            None
        };

        let page = OrderedCollectionPage {
            context: activitystreams_context(),
            kind: "OrderedCollectionPage".to_string(),
            id: page_url,
            part_of: clip_url,
            prev: None,
            next,
            ordered_items: items,
        };

        return (
            StatusCode::OK,
            [("Content-Type", "application/activity+json; charset=utf-8")],
            Json(page),
        )
            .into_response();
    }

    // Return collection summary
    let total_items = clip.notes_count as u64;
    let first = {
        let mut first_url = clip_url.clone();
        first_url.set_query(Some("page=true"));
        first_url
    };

    // Build a collection with clip metadata
    let collection = serde_json::json!({
        "@context": activitystreams_context(),
        "type": "OrderedCollection",
        "id": clip_url.to_string(),
        "name": clip.name,
        "summary": clip.description,
        "totalItems": total_items,
        "first": first.to_string(),
        "attributedTo": state.url_config.user_url(&username).to_string(),
        "published": clip.created_at.to_rfc3339(),
    });

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(collection),
    )
        .into_response()
}

/// Handle GET /users/{username}/clips - List of user's public clips.
pub async fn clips_list_handler(
    State(state): State<ClipCollectionState>,
    Path(username): Path<String>,
    Query(query): Query<CollectionQuery>,
) -> impl IntoResponse {
    info!(username = %username, "ActivityPub clips list lookup");

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

    if user.is_suspended {
        return (StatusCode::GONE, "User is suspended").into_response();
    }

    let clips_url = state
        .url_config
        .base_url
        .join(&format!("/users/{username}/clips"))
        .expect("valid URL");

    // If page=true, return a page of clips
    if query.page == Some(true) {
        let limit = 20u64;

        // Get public clips for this user
        let clips = match state
            .clip_repo
            .find_public_by_user(&user.id, limit, 0)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "Failed to fetch clips");
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

        // Convert clips to Collection references
        let items: Vec<serde_json::Value> = clips
            .iter()
            .map(|c| {
                let clip_url = state
                    .url_config
                    .base_url
                    .join(&format!("/users/{}/clips/{}", username, c.id))
                    .expect("valid URL");
                serde_json::json!({
                    "type": "OrderedCollection",
                    "id": clip_url.to_string(),
                    "name": c.name,
                    "summary": c.description,
                    "totalItems": c.notes_count,
                })
            })
            .collect();

        let mut page_url = clips_url.clone();
        page_url.set_query(Some("page=true"));

        let next = if clips.len() == limit as usize {
            clips.last().map(|c| {
                let mut next_url = clips_url.clone();
                next_url.set_query(Some(&format!("page=true&max_id={}", c.id)));
                next_url
            })
        } else {
            None
        };

        let page = OrderedCollectionPage {
            context: activitystreams_context(),
            kind: "OrderedCollectionPage".to_string(),
            id: page_url,
            part_of: clips_url,
            prev: None,
            next,
            ordered_items: items,
        };

        return (
            StatusCode::OK,
            [("Content-Type", "application/activity+json; charset=utf-8")],
            Json(page),
        )
            .into_response();
    }

    // Return collection summary
    let clip_count = match state.clip_repo.count_public_by_user(&user.id).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Failed to count clips");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let first = {
        let mut first_url = clips_url.clone();
        first_url.set_query(Some("page=true"));
        first_url
    };

    let collection = OrderedCollection {
        context: activitystreams_context(),
        kind: "OrderedCollection".to_string(),
        id: clips_url,
        total_items: clip_count,
        first: Some(first),
        last: None,
    };

    (
        StatusCode::OK,
        [("Content-Type", "application/activity+json; charset=utf-8")],
        Json(collection),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activitystreams_context() {
        let ctx = activitystreams_context();
        assert!(ctx.is_array());
    }
}
