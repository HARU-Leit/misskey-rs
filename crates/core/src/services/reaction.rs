//! Reaction service.

use crate::services::delivery::DeliveryService;
use crate::services::event_publisher::EventPublisherService;
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::reaction,
    repositories::{NoteRepository, ReactionRepository, UserRepository},
};
use sea_orm::Set;
use serde_json::json;

/// Reaction service for business logic.
#[derive(Clone)]
pub struct ReactionService {
    reaction_repo: ReactionRepository,
    note_repo: NoteRepository,
    user_repo: Option<UserRepository>,
    delivery: Option<DeliveryService>,
    event_publisher: Option<EventPublisherService>,
    server_url: String,
    id_gen: IdGenerator,
}

impl ReactionService {
    /// Create a new reaction service.
    #[must_use]
    pub fn new(reaction_repo: ReactionRepository, note_repo: NoteRepository) -> Self {
        Self {
            reaction_repo,
            note_repo,
            user_repo: None,
            delivery: None,
            event_publisher: None,
            server_url: String::new(),
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new reaction service with full dependencies.
    #[must_use]
    pub fn with_delivery(
        reaction_repo: ReactionRepository,
        note_repo: NoteRepository,
        user_repo: UserRepository,
        delivery: DeliveryService,
        server_url: String,
    ) -> Self {
        Self {
            reaction_repo,
            note_repo,
            user_repo: Some(user_repo),
            delivery: Some(delivery),
            event_publisher: None,
            server_url,
            id_gen: IdGenerator::new(),
        }
    }

    /// Set the delivery service.
    pub fn set_delivery(
        &mut self,
        user_repo: UserRepository,
        delivery: DeliveryService,
        server_url: String,
    ) {
        self.user_repo = Some(user_repo);
        self.delivery = Some(delivery);
        self.server_url = server_url;
    }

    /// Set the event publisher.
    pub fn set_event_publisher(&mut self, event_publisher: EventPublisherService) {
        self.event_publisher = Some(event_publisher);
    }

    /// System default reaction emoji (fallback when user has no default set).
    const DEFAULT_LIKE_EMOJI: &'static str = "👍";

    /// Like a note using the user's default reaction or system default.
    ///
    /// This is the "one-button like" feature that simplifies reacting to notes.
    /// It uses the user's configured default_reaction if set, otherwise falls back
    /// to the system default (👍).
    pub async fn like(
        &self,
        user_id: &str,
        note_id: &str,
        default_reaction: Option<&str>,
    ) -> AppResult<reaction::Model> {
        // Use user's default reaction, or fall back to system default
        let reaction = default_reaction.unwrap_or(Self::DEFAULT_LIKE_EMOJI);
        self.create(user_id, note_id, reaction).await
    }

    /// Create a reaction on a note.
    pub async fn create(
        &self,
        user_id: &str,
        note_id: &str,
        reaction: &str,
    ) -> AppResult<reaction::Model> {
        // Check if note exists
        let note = self.note_repo.get_by_id(note_id).await?;

        // Check if user already reacted
        if self.reaction_repo.has_reacted(user_id, note_id).await? {
            return Err(AppError::BadRequest(
                "Already reacted to this note".to_string(),
            ));
        }

        // Validate and normalize reaction
        let normalized_reaction = Self::normalize_reaction(reaction);

        let model = reaction::ActiveModel {
            id: Set(self.id_gen.generate()),
            user_id: Set(user_id.to_string()),
            note_id: Set(note_id.to_string()),
            reaction: Set(normalized_reaction.clone()),
            ..Default::default()
        };

        let created = self.reaction_repo.create(model).await?;

        // Increment note reaction count
        self.note_repo.increment_reactions_count(note_id).await?;

        // Queue ActivityPub Like activity for remote note authors
        if let (Some(delivery), Some(user_repo)) = (&self.delivery, &self.user_repo) {
            // Get note author
            if let Ok(note_author) = user_repo.get_by_id(&note.user_id).await {
                // Only send Like to remote users
                if note_author.host.is_some() {
                    if let Some(ref inbox) = note_author.inbox {
                        if let Err(e) = self
                            .queue_like_activity(
                                user_id,
                                &note,
                                &normalized_reaction,
                                inbox,
                                delivery,
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to queue Like activity");
                        }
                    }
                }
            }
        }

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher {
            if let Err(e) = event_publisher
                .publish_reaction_added(note_id, user_id, &normalized_reaction, &note.user_id)
                .await
            {
                tracing::warn!(error = %e, "Failed to publish reaction added event");
            }
        }

        Ok(created)
    }

    /// Delete a reaction from a note.
    pub async fn delete(&self, user_id: &str, note_id: &str) -> AppResult<()> {
        // Check if reaction exists
        let reaction = self
            .reaction_repo
            .find_by_user_and_note(user_id, note_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Reaction not found".to_string()))?;

        // Get note for ActivityPub delivery
        let note = self.note_repo.get_by_id(note_id).await?;

        self.reaction_repo.delete(&reaction.id).await?;

        // Decrement note reaction count
        self.note_repo.decrement_reactions_count(note_id).await?;

        // Queue ActivityPub Undo Like activity for remote note authors
        if let (Some(delivery), Some(user_repo)) = (&self.delivery, &self.user_repo) {
            if let Ok(note_author) = user_repo.get_by_id(&note.user_id).await {
                if note_author.host.is_some() {
                    if let Some(ref inbox) = note_author.inbox {
                        if let Err(e) = self
                            .queue_undo_like_activity(
                                user_id,
                                &note,
                                &reaction.reaction,
                                inbox,
                                delivery,
                            )
                            .await
                        {
                            tracing::warn!(error = %e, "Failed to queue Undo Like activity");
                        }
                    }
                }
            }
        }

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher {
            if let Err(e) = event_publisher
                .publish_reaction_removed(note_id, user_id, &reaction.reaction, &note.user_id)
                .await
            {
                tracing::warn!(error = %e, "Failed to publish reaction removed event");
            }
        }

        Ok(())
    }

    /// Get reactions on a note.
    pub async fn get_reactions(
        &self,
        note_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<reaction::Model>> {
        self.reaction_repo
            .find_by_note(note_id, limit, until_id)
            .await
    }

    /// Get reactions by a user.
    pub async fn get_user_reactions(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<reaction::Model>> {
        self.reaction_repo
            .find_by_user(user_id, limit, until_id)
            .await
    }

    /// Normalize a reaction string.
    fn normalize_reaction(reaction: &str) -> String {
        // If it's a custom emoji format like :emoji:, keep as-is
        if reaction.starts_with(':') && reaction.ends_with(':') && reaction.len() > 2 {
            return reaction.to_string();
        }

        // If it looks like a Unicode emoji, keep as-is
        if !reaction.is_ascii() {
            return reaction.to_string();
        }

        // Default to a star for invalid reactions
        "\u{2B50}".to_string() // Star emoji
    }

    // ==================== ActivityPub Delivery Helpers ====================

    /// Queue a Like activity.
    ///
    /// This sends a Like activity with both `_misskey_reaction` (for Misskey instances)
    /// and `content` (for Pleroma/Akkoma instances) fields for maximum compatibility.
    /// Mastodon will interpret this as a simple favorite (ignoring the emoji content).
    async fn queue_like_activity(
        &self,
        user_id: &str,
        note: &misskey_db::entities::note::Model,
        reaction: &str,
        inbox: &str,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        let actor_url = format!("{}/users/{}", self.server_url, user_id);
        let note_url = note
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/notes/{}", self.server_url, note.id));
        let like_id = format!(
            "{}/activities/like/{}/{}",
            self.server_url, user_id, note.id
        );

        // Use Misskey's EmojiReact extension for custom reactions
        // Include both `_misskey_reaction` (Misskey) and `content` (Pleroma/Akkoma) for compatibility
        let activity = json!({
            "@context": [
                "https://www.w3.org/ns/activitystreams",
                {
                    "_misskey_reaction": "https://misskey-hub.net/ns#_misskey_reaction"
                }
            ],
            "type": "Like",
            "id": like_id,
            "actor": actor_url,
            "object": note_url,
            "_misskey_reaction": reaction,
            "content": reaction,
        });

        delivery.queue_like(user_id, inbox, activity).await?;
        tracing::debug!(user_id = %user_id, note_id = %note.id, reaction = %reaction, "Queued Like activity");
        Ok(())
    }

    /// Queue an Undo Like activity.
    async fn queue_undo_like_activity(
        &self,
        user_id: &str,
        note: &misskey_db::entities::note::Model,
        reaction: &str,
        inbox: &str,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        let actor_url = format!("{}/users/{}", self.server_url, user_id);
        let note_url = note
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/notes/{}", self.server_url, note.id));
        let like_id = format!(
            "{}/activities/like/{}/{}",
            self.server_url, user_id, note.id
        );
        let undo_id = format!(
            "{}/activities/undo/like/{}/{}",
            self.server_url, user_id, note.id
        );

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Undo",
            "id": undo_id,
            "actor": actor_url,
            "object": {
                "type": "Like",
                "id": like_id,
                "actor": actor_url,
                "object": note_url,
                "_misskey_reaction": reaction,
            },
        });

        delivery
            .queue_undo(user_id, vec![inbox.to_string()], activity)
            .await?;
        tracing::debug!(user_id = %user_id, note_id = %note.id, "Queued Undo Like activity");
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use misskey_db::entities::note;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use serde_json::json;
    use std::sync::Arc;

    fn create_test_note(id: &str, user_id: &str) -> note::Model {
        note::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            user_host: None,
            text: Some("Test note".to_string()),
            cw: None,
            visibility: note::Visibility::Public,
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
            created_at: Utc::now().into(),
            updated_at: None,
            channel_id: None,
        }
    }

    fn create_test_reaction(
        id: &str,
        user_id: &str,
        note_id: &str,
        reaction_str: &str,
    ) -> reaction::Model {
        reaction::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            note_id: note_id.to_string(),
            reaction: reaction_str.to_string(),
            created_at: Utc::now().into(),
        }
    }

    // Unit tests for normalize_reaction
    #[test]
    fn test_normalize_reaction_custom_emoji() {
        let result = ReactionService::normalize_reaction(":like:");
        assert_eq!(result, ":like:");
    }

    #[test]
    fn test_normalize_reaction_unicode_emoji() {
        let result = ReactionService::normalize_reaction("👍");
        assert_eq!(result, "👍");
    }

    #[test]
    fn test_normalize_reaction_unicode_face() {
        let result = ReactionService::normalize_reaction("😀");
        assert_eq!(result, "😀");
    }

    #[test]
    fn test_normalize_reaction_invalid_ascii() {
        let result = ReactionService::normalize_reaction("like");
        assert_eq!(result, "⭐"); // Star emoji
    }

    #[test]
    fn test_normalize_reaction_empty_custom_emoji() {
        let result = ReactionService::normalize_reaction("::");
        assert_eq!(result, "⭐"); // Treated as invalid
    }

    #[test]
    fn test_normalize_reaction_single_colon() {
        let result = ReactionService::normalize_reaction(":");
        assert_eq!(result, "⭐");
    }

    // Service tests
    #[tokio::test]
    async fn test_create_reaction_note_not_found() {
        let reaction_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<note::Model>::new()])
                .into_connection(),
        );

        let reaction_repo = ReactionRepository::new(reaction_db);
        let note_repo = NoteRepository::new(note_db);

        let service = ReactionService::new(reaction_repo, note_repo);

        let result = service.create("user1", "nonexistent", "👍").await;
        assert!(result.is_err());
        match result {
            Err(AppError::NoteNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected NoteNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_create_reaction_already_reacted() {
        let note = create_test_note("note1", "author1");
        let existing_reaction = create_test_reaction("r1", "user1", "note1", "👍");

        let reaction_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[existing_reaction]])
                .into_connection(),
        );
        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note]])
                .into_connection(),
        );

        let reaction_repo = ReactionRepository::new(reaction_db);
        let note_repo = NoteRepository::new(note_db);

        let service = ReactionService::new(reaction_repo, note_repo);

        let result = service.create("user1", "note1", "❤️").await;
        assert!(result.is_err());
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("Already reacted"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_delete_reaction_not_found() {
        let reaction_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<reaction::Model>::new()])
                .into_connection(),
        );
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let reaction_repo = ReactionRepository::new(reaction_db);
        let note_repo = NoteRepository::new(note_db);

        let service = ReactionService::new(reaction_repo, note_repo);

        let result = service.delete("user1", "note1").await;
        assert!(result.is_err());
        match result {
            Err(AppError::NotFound(msg)) => {
                assert!(msg.contains("Reaction not found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_reactions() {
        let r1 = create_test_reaction("r1", "user1", "note1", "👍");
        let r2 = create_test_reaction("r2", "user2", "note1", "❤️");

        let reaction_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[r1, r2]])
                .into_connection(),
        );
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let reaction_repo = ReactionRepository::new(reaction_db);
        let note_repo = NoteRepository::new(note_db);

        let service = ReactionService::new(reaction_repo, note_repo);

        let result = service.get_reactions("note1", 10, None).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_user_reactions() {
        let r1 = create_test_reaction("r1", "user1", "note1", "👍");
        let r2 = create_test_reaction("r2", "user1", "note2", "❤️");

        let reaction_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[r1, r2]])
                .into_connection(),
        );
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let reaction_repo = ReactionRepository::new(reaction_db);
        let note_repo = NoteRepository::new(note_db);

        let service = ReactionService::new(reaction_repo, note_repo);

        let result = service.get_user_reactions("user1", 10, None).await.unwrap();
        assert_eq!(result.len(), 2);
    }
}
