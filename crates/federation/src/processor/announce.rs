//! Announce activity processor.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{note, user},
    repositories::{NoteRepository, UserRepository},
};
use sea_orm::Set;
use serde_json::json;
use tracing::info;

use crate::activities::AnnounceActivity;

/// Processor for Announce activities (renotes/boosts).
#[derive(Clone)]
pub struct AnnounceProcessor {
    user_repo: UserRepository,
    note_repo: NoteRepository,
    id_gen: IdGenerator,
}

impl AnnounceProcessor {
    /// Create a new announce processor.
    #[must_use]
    pub const fn new(user_repo: UserRepository, note_repo: NoteRepository) -> Self {
        Self {
            user_repo,
            note_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Process an incoming Announce activity.
    pub async fn process(&self, activity: &AnnounceActivity) -> AppResult<note::Model> {
        info!(
            actor = %activity.actor,
            object = %activity.object,
            "Processing Announce activity"
        );

        // Find the note being renoted
        let original_note = self
            .note_repo
            .find_by_uri(activity.object.as_str())
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Original note not found: {}", activity.object))
            })?;

        // Find or fetch the actor
        let actor = self.find_or_fetch_actor(&activity.actor).await?;

        // Check if already renoted by this user
        if let Some(existing) = self
            .note_repo
            .find_renote(&actor.id, &original_note.id)
            .await?
        {
            info!(renote_id = %existing.id, "User already renoted this note");
            return Ok(existing);
        }

        // Create renote
        let renote_id = self.id_gen.generate();
        let created_at = activity.published;

        let model = note::ActiveModel {
            id: Set(renote_id),
            user_id: Set(actor.id.clone()),
            user_host: Set(actor.host.clone()),
            text: Set(None),
            cw: Set(None),
            visibility: Set(note::Visibility::Public),
            reply_id: Set(None),
            renote_id: Set(Some(original_note.id.clone())),
            thread_id: Set(None),
            mentions: Set(json!([])),
            visible_user_ids: Set(json!([])),
            file_ids: Set(json!([])),
            tags: Set(json!([])),
            reactions: Set(json!({})),
            is_local: Set(false),
            uri: Set(Some(activity.id.to_string())),
            created_at: Set(created_at.into()),
            ..Default::default()
        };

        let renote = self.note_repo.create(model).await?;

        // Update renote count on original note
        self.note_repo
            .increment_renote_count(&original_note.id)
            .await?;

        info!(
            renote_id = %renote.id,
            actor = %actor.id,
            original_note = %original_note.id,
            "Created renote from remote"
        );

        Ok(renote)
    }

    /// Find an existing actor or fetch from remote.
    async fn find_or_fetch_actor(&self, actor_url: &url::Url) -> AppResult<user::Model> {
        if let Some(user) = self.user_repo.find_by_uri(actor_url.as_str()).await? {
            return Ok(user);
        }

        // TODO: Fetch actor from remote server
        Err(AppError::NotFound(format!("Actor not found: {actor_url}")))
    }
}
