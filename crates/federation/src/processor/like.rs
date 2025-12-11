//! Like activity processor.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{reaction, user},
    repositories::{NoteRepository, ReactionRepository, UserRepository},
};
use sea_orm::Set;
use tracing::info;

use super::ActorFetcher;
use crate::{activities::LikeActivity, client::ApClient};

/// Processor for Like activities (reactions).
#[derive(Clone)]
pub struct LikeProcessor {
    actor_fetcher: ActorFetcher,
    note_repo: NoteRepository,
    reaction_repo: ReactionRepository,
    id_gen: IdGenerator,
}

impl LikeProcessor {
    /// Create a new like processor.
    #[must_use]
    pub const fn new(
        user_repo: UserRepository,
        note_repo: NoteRepository,
        reaction_repo: ReactionRepository,
        ap_client: ApClient,
    ) -> Self {
        Self {
            actor_fetcher: ActorFetcher::new(user_repo, ap_client),
            note_repo,
            reaction_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Process an incoming Like activity.
    pub async fn process(&self, activity: &LikeActivity) -> AppResult<reaction::Model> {
        info!(
            actor = %activity.actor,
            object = %activity.object,
            "Processing Like activity"
        );

        // Find the note being reacted to
        let note = self
            .note_repo
            .find_by_uri(activity.object.as_str())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Note not found: {}", activity.object)))?;

        // Find or fetch the actor
        let actor = self.find_or_fetch_actor(&activity.actor).await?;

        // Check if already reacted
        if let Some(existing) = self
            .reaction_repo
            .find_by_user_and_note(&actor.id, &note.id)
            .await?
        {
            info!(reaction_id = %existing.id, "User already reacted to this note");
            return Ok(existing);
        }

        // Determine the reaction content
        let reaction_content = self.normalize_reaction(activity);

        // Create reaction
        let reaction_id = self.id_gen.generate();
        let model = reaction::ActiveModel {
            id: Set(reaction_id),
            user_id: Set(actor.id.clone()),
            note_id: Set(note.id.clone()),
            reaction: Set(reaction_content),
            created_at: Set(chrono::Utc::now().into()),
        };

        let reaction = self.reaction_repo.create(model).await?;

        info!(
            reaction_id = %reaction.id,
            actor = %actor.id,
            note = %note.id,
            "Created reaction from remote"
        );

        Ok(reaction)
    }

    /// Find an existing actor or fetch from remote.
    async fn find_or_fetch_actor(&self, actor_url: &url::Url) -> AppResult<user::Model> {
        self.actor_fetcher.find_or_fetch(actor_url).await
    }

    /// Normalize reaction content from `ActivityPub`.
    fn normalize_reaction(&self, activity: &LikeActivity) -> String {
        // Try Misskey-specific reaction first, then content, then default
        if let Some(ref reaction) = activity.misskey_reaction
            && !reaction.is_empty()
        {
            return reaction.clone();
        }
        if let Some(ref content) = activity.content
            && !content.is_empty()
        {
            return content.clone();
        }
        // Default reaction (Misskey uses ❤️ for standard Like)
        "❤️".to_string()
    }
}
