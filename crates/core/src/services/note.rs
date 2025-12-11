//! Note service.

use crate::services::delivery::DeliveryService;
use crate::services::event_publisher::EventPublisherService;
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::note::{self, Visibility},
    entities::note_edit,
    repositories::{FollowingRepository, NoteRepository, UserRepository},
};
use sea_orm::Set;
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

/// Note service for business logic.
#[derive(Clone)]
pub struct NoteService {
    note_repo: NoteRepository,
    user_repo: UserRepository,
    following_repo: FollowingRepository,
    delivery: Option<DeliveryService>,
    event_publisher: Option<EventPublisherService>,
    server_url: String,
    id_gen: IdGenerator,
}

/// Input for creating a new note.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteInput {
    #[validate(length(max = 3000))]
    pub text: Option<String>,

    #[validate(length(max = 100))]
    pub cw: Option<String>,

    #[serde(default = "default_visibility")]
    pub visibility: Visibility,

    pub reply_id: Option<String>,
    pub renote_id: Option<String>,

    #[validate(length(max = 16))]
    #[serde(default)]
    pub file_ids: Vec<String>,

    #[serde(default)]
    pub visible_user_ids: Vec<String>,

    /// Channel ID to post to (optional).
    pub channel_id: Option<String>,
}

const fn default_visibility() -> Visibility {
    Visibility::Public
}

/// Note with author information.
pub struct NoteWithAuthor {
    pub note: note::Model,
    pub author: misskey_db::entities::user::Model,
}

/// Input for updating a note.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteInput {
    /// The note ID to update.
    pub note_id: String,

    /// New text content (None = no change, Some(None) = remove, Some(Some(text)) = set).
    pub text: Option<Option<String>>,

    /// New content warning (None = no change, Some(None) = remove, Some(Some(cw)) = set).
    pub cw: Option<Option<String>>,

    /// New file IDs.
    #[validate(length(max = 16))]
    pub file_ids: Option<Vec<String>>,
}

impl NoteService {
    /// Create a new note service.
    #[must_use]
    pub fn new(
        note_repo: NoteRepository,
        user_repo: UserRepository,
        following_repo: FollowingRepository,
    ) -> Self {
        Self {
            note_repo,
            user_repo,
            following_repo,
            delivery: None,
            event_publisher: None,
            server_url: String::new(),
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new note service with ActivityPub delivery support.
    #[must_use]
    pub fn with_delivery(
        note_repo: NoteRepository,
        user_repo: UserRepository,
        following_repo: FollowingRepository,
        delivery: DeliveryService,
        server_url: String,
    ) -> Self {
        Self {
            note_repo,
            user_repo,
            following_repo,
            delivery: Some(delivery),
            event_publisher: None,
            server_url,
            id_gen: IdGenerator::new(),
        }
    }

    /// Set the delivery service.
    pub fn set_delivery(&mut self, delivery: DeliveryService, server_url: String) {
        self.delivery = Some(delivery);
        self.server_url = server_url;
    }

    /// Set the event publisher.
    pub fn set_event_publisher(&mut self, event_publisher: EventPublisherService) {
        self.event_publisher = Some(event_publisher);
    }

    /// Create a new note.
    pub async fn create(&self, user_id: &str, input: CreateNoteInput) -> AppResult<note::Model> {
        input.validate()?;

        // Validate: text or renote required
        if input.text.is_none() && input.renote_id.is_none() && input.file_ids.is_empty() {
            return Err(AppError::BadRequest(
                "Text, renote, or files required".to_string(),
            ));
        }

        // Validate reply target exists
        let reply = if let Some(ref reply_id) = input.reply_id {
            Some(self.note_repo.get_by_id(reply_id).await?)
        } else {
            None
        };

        // Validate renote target exists
        let renote = if let Some(ref renote_id) = input.renote_id {
            Some(self.note_repo.get_by_id(renote_id).await?)
        } else {
            None
        };

        // Get user
        let user = self.user_repo.get_by_id(user_id).await?;

        // Extract mentions from text
        let mentions = extract_mentions(input.text.as_deref().unwrap_or(""));

        // Extract hashtags from text
        let tags = extract_hashtags(input.text.as_deref().unwrap_or(""));

        // Determine thread_id
        let thread_id = reply
            .as_ref()
            .and_then(|r| r.thread_id.clone())
            .or_else(|| reply.as_ref().map(|r| r.id.clone()));

        let note_id = self.id_gen.generate();

        let model = note::ActiveModel {
            id: Set(note_id.clone()),
            user_id: Set(user_id.to_string()),
            user_host: Set(user.host.clone()),
            text: Set(input.text),
            cw: Set(input.cw),
            visibility: Set(input.visibility),
            reply_id: Set(input.reply_id),
            renote_id: Set(input.renote_id),
            thread_id: Set(thread_id),
            mentions: Set(json!(mentions)),
            visible_user_ids: Set(json!(input.visible_user_ids)),
            file_ids: Set(json!(input.file_ids)),
            tags: Set(json!(tags)),
            reactions: Set(json!({})),
            is_local: Set(user.host.is_none()),
            channel_id: Set(input.channel_id.clone()),
            ..Default::default()
        };

        let note = self.note_repo.create(model).await?;

        // Update user's notes count
        self.user_repo.increment_notes_count(user_id).await?;

        // Update reply count if this is a reply
        if let Some(ref reply_note) = reply {
            self.note_repo
                .increment_replies_count(&reply_note.id)
                .await?;
            tracing::debug!(reply_to = %reply_note.id, "Created reply");
        }

        // Update renote count if this is a renote (pure renote without text)
        if let Some(ref renote_note) = renote {
            if note.text.is_none() {
                self.note_repo
                    .increment_renote_count(&renote_note.id)
                    .await?;
            }
            tracing::debug!(renote_of = %renote_note.id, "Created renote");
        }

        // Queue ActivityPub delivery if federation is enabled
        if let Some(ref delivery) = self.delivery {
            // Only deliver local notes
            if note.is_local {
                if let Err(e) = self.queue_create_delivery(&note, &user, delivery).await {
                    tracing::warn!(error = %e, note_id = %note.id, "Failed to queue ActivityPub delivery");
                }
            }
        }

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher {
            let visibility_str = match note.visibility {
                Visibility::Public => "public",
                Visibility::Home => "home",
                Visibility::Followers => "followers",
                Visibility::Specified => "specified",
            };
            if let Err(e) = event_publisher
                .publish_note_created(
                    &note.id,
                    &note.user_id,
                    note.text.as_deref(),
                    visibility_str,
                )
                .await
            {
                tracing::warn!(error = %e, note_id = %note.id, "Failed to publish note created event");
            }
        }

        Ok(note)
    }

    /// Queue ActivityPub Create delivery for a note.
    async fn queue_create_delivery(
        &self,
        note: &note::Model,
        user: &misskey_db::entities::user::Model,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        // Only deliver public/home notes
        if !matches!(note.visibility, Visibility::Public | Visibility::Home) {
            return Ok(());
        }

        // Build ActivityPub Note object
        let note_url = format!("{}/notes/{}", self.server_url, note.id);
        let actor_url = format!("{}/users/{}", self.server_url, user.id);
        let activity_id = format!("{}/activities/create/{}", self.server_url, note.id);

        let followers_url = format!("{}/followers", actor_url);
        let public_url = "https://www.w3.org/ns/activitystreams#Public".to_string();

        let (to_field, cc_field): (Vec<String>, Vec<String>) =
            if note.visibility == Visibility::Public {
                (vec![public_url.clone()], vec![followers_url.clone()])
            } else {
                (vec![followers_url.clone()], vec![])
            };

        let ap_note = json!({
            "type": "Note",
            "id": note_url,
            "attributedTo": actor_url,
            "content": note.text.clone().unwrap_or_default(),
            "published": note.created_at.to_rfc3339(),
            "to": to_field,
            "cc": cc_field,
            "inReplyTo": note.reply_id.as_ref().map(|id| format!("{}/notes/{}", self.server_url, id)),
            "summary": note.cw.clone(),
            "sensitive": note.cw.is_some(),
        });

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Create",
            "id": activity_id,
            "actor": actor_url,
            "object": ap_note,
            "published": note.created_at.to_rfc3339(),
            "to": to_field,
            "cc": cc_field,
        });

        // Get follower inboxes
        let inboxes = self.get_follower_inboxes(&user.id).await?;

        if !inboxes.is_empty() {
            delivery
                .queue_create_note(&user.id, &note.id, activity, inboxes)
                .await?;
            tracing::debug!(note_id = %note.id, "Queued ActivityPub Create delivery");
        }

        Ok(())
    }

    /// Get unique inbox URLs for a user's followers.
    async fn get_follower_inboxes(&self, user_id: &str) -> AppResult<Vec<String>> {
        let followers = self
            .following_repo
            .find_followers(user_id, 10000, None)
            .await?;

        let mut inboxes: Vec<String> = followers
            .into_iter()
            .filter_map(|f| {
                // Prefer shared inbox for efficiency
                f.followee_shared_inbox.or(f.followee_inbox)
            })
            .collect();

        // Deduplicate shared inboxes
        inboxes.sort();
        inboxes.dedup();

        Ok(inboxes)
    }

    /// Get a note by ID.
    pub async fn get(&self, id: &str) -> AppResult<note::Model> {
        self.note_repo.get_by_id(id).await
    }

    /// Delete a note.
    pub async fn delete(&self, note_id: &str, user_id: &str) -> AppResult<()> {
        let note = self.note_repo.get_by_id(note_id).await?;

        // Check ownership
        if note.user_id != user_id {
            return Err(AppError::Forbidden(
                "Cannot delete other user's note".to_string(),
            ));
        }

        // Queue ActivityPub Delete before actually deleting
        // (we need the note data for building the activity)
        if let Some(ref delivery) = self.delivery {
            if note.is_local {
                if let Err(e) = self.queue_delete_delivery(&note, delivery).await {
                    tracing::warn!(error = %e, note_id = %note.id, "Failed to queue ActivityPub Delete delivery");
                }
            }
        }

        // Decrement reply count if this was a reply
        if let Some(ref reply_id) = note.reply_id {
            let _ = self.note_repo.decrement_replies_count(reply_id).await;
        }

        // Decrement renote count if this was a pure renote
        if let Some(ref renote_id) = note.renote_id
            && note.text.is_none()
        {
            let _ = self.note_repo.decrement_renote_count(renote_id).await;
        }

        self.note_repo.delete(note_id).await?;

        // Update user's notes count
        self.user_repo.decrement_notes_count(user_id).await?;

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher {
            if let Err(e) = event_publisher.publish_note_deleted(note_id, user_id).await {
                tracing::warn!(error = %e, note_id = %note_id, "Failed to publish note deleted event");
            }
        }

        Ok(())
    }

    /// Queue ActivityPub Delete delivery for a note.
    async fn queue_delete_delivery(
        &self,
        note: &note::Model,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        // Only deliver deletion of public/home notes
        if !matches!(note.visibility, Visibility::Public | Visibility::Home) {
            return Ok(());
        }

        let note_url = format!("{}/notes/{}", self.server_url, note.id);
        let actor_url = format!("{}/users/{}", self.server_url, note.user_id);
        let activity_id = format!("{}/activities/delete/{}", self.server_url, note.id);

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Delete",
            "id": activity_id,
            "actor": actor_url,
            "object": note_url,
            "to": ["https://www.w3.org/ns/activitystreams#Public"],
            "cc": [format!("{}/followers", actor_url)],
        });

        // Get follower inboxes
        let inboxes = self.get_follower_inboxes(&note.user_id).await?;

        if !inboxes.is_empty() {
            delivery
                .queue_delete_note(&note.user_id, &note.id, activity, inboxes)
                .await?;
            tracing::debug!(note_id = %note.id, "Queued ActivityPub Delete delivery");
        }

        Ok(())
    }

    /// Get local public timeline.
    pub async fn local_timeline(
        &self,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        self.note_repo.find_local_public(limit, until_id).await
    }

    /// Get global public timeline.
    pub async fn global_timeline(
        &self,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        self.note_repo.find_global_public(limit, until_id).await
    }

    /// Get home timeline (notes from followed users + own notes).
    pub async fn home_timeline(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        // Get IDs of users that the current user follows
        let followings = self
            .following_repo
            .find_following(user_id, 10000, None)
            .await?;
        let following_ids: Vec<String> = followings.iter().map(|f| f.followee_id.clone()).collect();

        self.note_repo
            .find_home_timeline(user_id, &following_ids, limit, until_id)
            .await
    }

    /// Get user's notes.
    pub async fn user_notes(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        self.note_repo.find_by_user(user_id, limit, until_id).await
    }

    /// Search notes by text content.
    pub async fn search_notes(
        &self,
        query: &str,
        limit: u64,
        until_id: Option<&str>,
        user_id: Option<&str>,
        host: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        self.note_repo
            .search(query, limit, until_id, user_id, host)
            .await
    }

    /// Search notes by hashtag.
    pub async fn search_by_tag(
        &self,
        tag: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        self.note_repo.search_by_tag(tag, limit, until_id).await
    }

    /// Find trending notes (high reaction count in recent timeframe).
    pub async fn find_trending(
        &self,
        limit: u64,
        min_reactions: i32,
        hours: i64,
    ) -> AppResult<Vec<note::Model>> {
        self.note_repo
            .find_trending(limit, min_reactions, hours)
            .await
    }

    /// Get replies to a note.
    pub async fn get_replies(&self, note_id: &str, limit: u64) -> AppResult<Vec<note::Model>> {
        self.note_repo.find_replies(note_id, limit).await
    }

    /// Get renotes of a note.
    pub async fn get_renotes(&self, note_id: &str, limit: u64) -> AppResult<Vec<note::Model>> {
        self.note_repo.find_renotes(note_id, limit).await
    }

    /// Get conversation (ancestors) for a note.
    /// Returns the chain of parent notes leading to this note.
    pub async fn get_conversation(
        &self,
        note_id: &str,
        limit: usize,
    ) -> AppResult<Vec<note::Model>> {
        let mut conversation = self.note_repo.find_ancestors(note_id, limit).await?;

        // Add the current note at the end
        if let Ok(note) = self.note_repo.find_by_id(note_id).await
            && let Some(n) = note
        {
            conversation.push(n);
        }

        Ok(conversation)
    }

    /// Get children (immediate replies) for a note.
    pub async fn get_children(&self, note_id: &str, limit: u64) -> AppResult<Vec<note::Model>> {
        self.note_repo.find_children(note_id, limit, None).await
    }

    /// Get full thread for a note.
    /// Returns all notes in the same thread.
    pub async fn get_thread(&self, note_id: &str, limit: u64) -> AppResult<Vec<note::Model>> {
        let note = self.note_repo.get_by_id(note_id).await?;

        // Use thread_id if available, otherwise use the note's own ID
        let thread_id = note.thread_id.as_ref().unwrap_or(&note.id);
        self.note_repo.find_thread(thread_id, limit).await
    }

    // ==================== Note Editing ====================

    /// Update a note (with edit history).
    pub async fn update(&self, user_id: &str, input: UpdateNoteInput) -> AppResult<note::Model> {
        input.validate()?;

        // Get the existing note
        let note = self.note_repo.get_by_id(&input.note_id).await?;

        // Verify ownership
        if note.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only edit your own notes".to_string(),
            ));
        }

        // Verify the note is local (can't edit federated notes)
        if !note.is_local {
            return Err(AppError::BadRequest(
                "Cannot edit federated notes".to_string(),
            ));
        }

        // Track what's being changed
        let old_text = note.text.clone();
        let old_cw = note.cw.clone();
        let old_file_ids: Vec<String> =
            serde_json::from_value(note.file_ids.clone()).unwrap_or_default();

        // Check if anything actually changed
        let text_changed = input.text.is_some();
        let cw_changed = input.cw.is_some();
        let files_changed = input.file_ids.is_some();

        if !text_changed && !cw_changed && !files_changed {
            // No changes, return the note as-is
            return Ok(note);
        }

        // Compute new values for history
        let new_text = input.text.clone().unwrap_or_else(|| old_text.clone());
        let new_cw = input.cw.clone().unwrap_or_else(|| old_cw.clone());
        let new_file_ids = input
            .file_ids
            .clone()
            .unwrap_or_else(|| old_file_ids.clone());

        // Create edit history record
        let edit_id = self.id_gen.generate();
        let edit_record = note_edit::ActiveModel {
            id: Set(edit_id),
            note_id: Set(note.id.clone()),
            old_text: Set(old_text.clone()),
            new_text: Set(new_text.clone()),
            old_cw: Set(old_cw.clone()),
            new_cw: Set(new_cw.clone()),
            old_file_ids: Set(json!(old_file_ids)),
            new_file_ids: Set(json!(new_file_ids.clone())),
            edited_at: Set(chrono::Utc::now().into()),
        };

        self.note_repo.create_edit_history(edit_record).await?;

        // Update the note
        let mut active_note: note::ActiveModel = note.into();

        if let Some(new_text_value) = input.text {
            // Re-extract mentions and hashtags
            let mentions = if let Some(ref t) = new_text_value {
                extract_mentions(t)
            } else {
                Vec::new()
            };
            let tags = if let Some(ref t) = new_text_value {
                extract_hashtags(t)
            } else {
                Vec::new()
            };

            active_note.text = Set(new_text_value);
            active_note.mentions = Set(json!(mentions));
            active_note.tags = Set(json!(tags));
        }

        if let Some(new_cw_value) = input.cw {
            active_note.cw = Set(new_cw_value);
        }

        if let Some(file_ids) = input.file_ids {
            active_note.file_ids = Set(json!(file_ids));
        }

        active_note.updated_at = Set(Some(chrono::Utc::now().into()));

        let updated_note = self.note_repo.update(active_note).await?;

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher {
            if let Err(e) = event_publisher.publish_note_updated(&updated_note.id).await {
                tracing::warn!(error = %e, note_id = %updated_note.id, "Failed to publish note updated event");
            }
        }

        Ok(updated_note)
    }

    /// Get edit history for a note.
    pub async fn get_edit_history(
        &self,
        note_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<note_edit::Model>> {
        // Verify the note exists
        let _ = self.note_repo.get_by_id(note_id).await?;

        self.note_repo
            .get_edit_history(note_id, limit, offset)
            .await
    }

    /// Count edits for a note.
    pub async fn count_edits(&self, note_id: &str) -> AppResult<u64> {
        self.note_repo.count_edit_history(note_id).await
    }

    /// Check if a note has been edited.
    pub async fn is_edited(&self, note_id: &str) -> AppResult<bool> {
        let count = self.note_repo.count_edit_history(note_id).await?;
        Ok(count > 0)
    }

    // ==================== Channel Timeline ====================

    /// Get channel timeline (notes posted to a specific channel).
    pub async fn channel_timeline(
        &self,
        channel_id: &str,
        limit: u64,
        until_id: Option<&str>,
        since_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        self.note_repo
            .find_by_channel(channel_id, limit, until_id, since_id)
            .await
    }
}

/// Extract @mentions from text.
fn extract_mentions(text: &str) -> Vec<String> {
    let mut mentions = Vec::new();
    for word in text.split_whitespace() {
        if word.starts_with('@') && word.len() > 1 {
            mentions.push(word[1..].to_string());
        }
    }
    mentions
}

/// Extract #hashtags from text.
fn extract_hashtags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for word in text.split_whitespace() {
        if word.starts_with('#') && word.len() > 1 {
            tags.push(word[1..].to_lowercase());
        }
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use misskey_db::entities::{following, user};
    use sea_orm::{DatabaseBackend, MockDatabase};
    use serde_json::json;
    use std::sync::Arc;

    fn create_test_user(id: &str, username: &str) -> user::Model {
        user::Model {
            id: id.to_string(),
            username: username.to_string(),
            username_lower: username.to_lowercase(),
            host: None,
            name: Some("Test User".to_string()),
            description: None,
            avatar_url: None,
            banner_url: None,
            is_bot: false,
            is_cat: false,
            is_locked: false,
            is_suspended: false,
            is_silenced: false,
            is_admin: false,
            is_moderator: false,
            followers_count: 0,
            following_count: 0,
            notes_count: 0,
            inbox: None,
            shared_inbox: None,
            featured: None,
            uri: None,
            last_fetched_at: None,
            token: Some("test_token".to_string()),
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    fn create_test_note(id: &str, user_id: &str, text: Option<&str>) -> note::Model {
        note::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            user_host: None,
            text: text.map(std::string::ToString::to_string),
            cw: None,
            visibility: Visibility::Public,
            reply_id: None,
            renote_id: None,
            thread_id: None,
            mentions: json!([]),
            visible_user_ids: json!([]),
            file_ids: json!([]),
            tags: json!([]),
            reactions: json!({}),
            replies_count: 0,
            renote_count: 0,
            reaction_count: 0,
            is_local: true,
            uri: None,
            url: None,
            channel_id: None,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    // Unit tests for helper functions
    #[test]
    fn test_extract_mentions_single() {
        let mentions = extract_mentions("Hello @user!");
        assert_eq!(mentions, vec!["user!"]);
    }

    #[test]
    fn test_extract_mentions_multiple() {
        let mentions = extract_mentions("Hello @alice and @bob");
        assert_eq!(mentions, vec!["alice", "bob"]);
    }

    #[test]
    fn test_extract_mentions_with_host() {
        let mentions = extract_mentions("Hello @user@example.com");
        assert_eq!(mentions, vec!["user@example.com"]);
    }

    #[test]
    fn test_extract_mentions_empty() {
        let mentions = extract_mentions("Hello world");
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_extract_mentions_at_only() {
        let mentions = extract_mentions("Just @ symbol");
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_extract_hashtags_single() {
        let tags = extract_hashtags("Check out #rust");
        assert_eq!(tags, vec!["rust"]);
    }

    #[test]
    fn test_extract_hashtags_multiple() {
        let tags = extract_hashtags("#rust #programming #code");
        assert_eq!(tags, vec!["rust", "programming", "code"]);
    }

    #[test]
    fn test_extract_hashtags_lowercase() {
        let tags = extract_hashtags("#Rust #PROGRAMMING");
        assert_eq!(tags, vec!["rust", "programming"]);
    }

    #[test]
    fn test_extract_hashtags_empty() {
        let tags = extract_hashtags("No hashtags here");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_extract_hashtags_hash_only() {
        let tags = extract_hashtags("Just # symbol");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_default_visibility() {
        assert_eq!(default_visibility(), Visibility::Public);
    }

    // Service tests
    #[tokio::test]
    async fn test_create_note_empty_content_returns_error() {
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let user_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let following_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let note_repo = NoteRepository::new(note_db);
        let user_repo = UserRepository::new(user_db);
        let following_repo = FollowingRepository::new(following_db);

        let service = NoteService::new(note_repo, user_repo, following_repo);

        let input = CreateNoteInput {
            text: None,
            cw: None,
            visibility: Visibility::Public,
            reply_id: None,
            renote_id: None,
            file_ids: vec![],
            visible_user_ids: vec![],
            channel_id: None,
        };

        let result = service.create("user1", input).await;
        assert!(result.is_err());
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("Text, renote, or files required"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_delete_note_wrong_owner_returns_error() {
        let note = create_test_note("note1", "user1", Some("Hello"));

        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note.clone()]])
                .into_connection(),
        );
        let user_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let following_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let note_repo = NoteRepository::new(note_db);
        let user_repo = UserRepository::new(user_db);
        let following_repo = FollowingRepository::new(following_db);

        let service = NoteService::new(note_repo, user_repo, following_repo);

        // Try to delete note owned by user1 as user2
        let result = service.delete("note1", "user2").await;
        assert!(result.is_err());
        match result {
            Err(AppError::Forbidden(msg)) => {
                assert!(msg.contains("Cannot delete other user's note"));
            }
            _ => panic!("Expected Forbidden error"),
        }
    }

    #[tokio::test]
    async fn test_get_note_not_found() {
        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<note::Model>::new()])
                .into_connection(),
        );
        let user_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let following_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let note_repo = NoteRepository::new(note_db);
        let user_repo = UserRepository::new(user_db);
        let following_repo = FollowingRepository::new(following_db);

        let service = NoteService::new(note_repo, user_repo, following_repo);

        let result = service.get("nonexistent").await;
        assert!(result.is_err());
        match result {
            Err(AppError::NoteNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected NoteNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_local_timeline() {
        let note1 = create_test_note("note1", "user1", Some("First"));
        let note2 = create_test_note("note2", "user2", Some("Second"));

        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note1, note2]])
                .into_connection(),
        );
        let user_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let following_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let note_repo = NoteRepository::new(note_db);
        let user_repo = UserRepository::new(user_db);
        let following_repo = FollowingRepository::new(following_db);

        let service = NoteService::new(note_repo, user_repo, following_repo);

        let result = service.local_timeline(10, None).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_home_timeline() {
        let following1 = following::Model {
            id: "f1".to_string(),
            follower_id: "user1".to_string(),
            followee_id: "user2".to_string(),
            follower_host: None,
            followee_host: None,
            followee_inbox: None,
            followee_shared_inbox: None,
            created_at: Utc::now().into(),
        };
        let note1 = create_test_note("note1", "user1", Some("My note"));
        let note2 = create_test_note("note2", "user2", Some("Friend's note"));

        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note1, note2]])
                .into_connection(),
        );
        let user_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let following_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[following1]])
                .into_connection(),
        );

        let note_repo = NoteRepository::new(note_db);
        let user_repo = UserRepository::new(user_db);
        let following_repo = FollowingRepository::new(following_db);

        let service = NoteService::new(note_repo, user_repo, following_repo);

        let result = service.home_timeline("user1", 10, None).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_user_notes() {
        let note1 = create_test_note("note1", "user1", Some("First"));
        let note2 = create_test_note("note2", "user1", Some("Second"));

        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note1, note2]])
                .into_connection(),
        );
        let user_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let following_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let note_repo = NoteRepository::new(note_db);
        let user_repo = UserRepository::new(user_db);
        let following_repo = FollowingRepository::new(following_db);

        let service = NoteService::new(note_repo, user_repo, following_repo);

        let result = service.user_notes("user1", 10, None).await.unwrap();
        assert_eq!(result.len(), 2);
    }
}
