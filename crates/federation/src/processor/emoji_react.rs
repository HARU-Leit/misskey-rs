//! `EmojiReact` activity processor.
//!
//! Handles incoming `EmojiReact` activities from Pleroma/Akkoma instances.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{reaction, user},
    repositories::{NoteRepository, ReactionRepository, UserRepository},
};
use sea_orm::Set;
use tracing::info;

use super::ActorFetcher;
use crate::{activities::EmojiReactActivity, client::ApClient};

/// Processor for `EmojiReact` activities (Pleroma/Akkoma style reactions).
#[derive(Clone)]
pub struct EmojiReactProcessor {
    actor_fetcher: ActorFetcher,
    note_repo: NoteRepository,
    reaction_repo: ReactionRepository,
    id_gen: IdGenerator,
}

impl EmojiReactProcessor {
    /// Create a new `EmojiReact` processor.
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

    /// Process an incoming `EmojiReact` activity.
    pub async fn process(&self, activity: &EmojiReactActivity) -> AppResult<reaction::Model> {
        info!(
            actor = %activity.actor,
            object = %activity.object,
            content = %activity.content,
            "Processing EmojiReact activity"
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

        // Normalize the reaction content
        let reaction_content = self.normalize_reaction(&activity.content, &activity.tag);

        // Create reaction
        let reaction_id = self.id_gen.generate();
        let model = reaction::ActiveModel {
            id: Set(reaction_id),
            user_id: Set(actor.id.clone()),
            note_id: Set(note.id.clone()),
            reaction: Set(reaction_content.clone()),
            created_at: Set(chrono::Utc::now().into()),
        };

        let reaction = self.reaction_repo.create(model).await?;

        info!(
            reaction_id = %reaction.id,
            actor = %actor.id,
            note = %note.id,
            content = %reaction_content,
            "Created reaction from remote EmojiReact"
        );

        Ok(reaction)
    }

    /// Find an existing actor or fetch from remote.
    async fn find_or_fetch_actor(&self, actor_url: &url::Url) -> AppResult<user::Model> {
        self.actor_fetcher.find_or_fetch(actor_url).await
    }

    /// Normalize reaction content from `EmojiReact` activity.
    fn normalize_reaction(
        &self,
        content: &str,
        tags: &Option<Vec<crate::activities::EmojiTag>>,
    ) -> String {
        // If content is empty, default to heart
        if content.is_empty() {
            return "❤️".to_string();
        }

        // Check if it's a custom emoji shortcode
        if content.starts_with(':') && content.ends_with(':') {
            // Try to get the remote emoji host from tags
            if let Some(tag_list) = tags {
                for tag in tag_list {
                    if tag.kind == "Emoji"
                        && tag.name == content
                        && let Some(ref icon) = tag.icon
                    {
                        // Store as remote emoji format: :emoji@host:
                        if let Some(host) = icon.url.host_str() {
                            return format!(":{}@{}:", content.trim_matches(':'), host);
                        }
                    }
                }
            }
        }

        // Return as-is if it's a Unicode emoji or shortcode
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::activities::{EmojiIcon, EmojiTag};
    use url::Url;

    /// Helper function to test reaction normalization.
    fn normalize_reaction(content: &str, tags: &Option<Vec<EmojiTag>>) -> String {
        // If content is empty, default to heart
        if content.is_empty() {
            return "❤️".to_string();
        }

        // Check if it's a custom emoji shortcode
        if content.starts_with(':') && content.ends_with(':') {
            // Try to get the remote emoji host from tags
            if let Some(tag_list) = tags {
                for tag in tag_list {
                    if tag.kind == "Emoji"
                        && tag.name == content
                        && let Some(ref icon) = tag.icon
                    {
                        // Store as remote emoji format: :emoji@host:
                        if let Some(host) = icon.url.host_str() {
                            return format!(":{}@{}:", content.trim_matches(':'), host);
                        }
                    }
                }
            }
        }

        // Return as-is if it's a Unicode emoji or shortcode
        content.to_string()
    }

    #[test]
    fn test_normalize_unicode_emoji() {
        let result = normalize_reaction("👍", &None);
        assert_eq!(result, "👍");
    }

    #[test]
    fn test_normalize_empty_content() {
        let result = normalize_reaction("", &None);
        assert_eq!(result, "❤️");
    }

    #[test]
    fn test_normalize_custom_emoji_with_tag() {
        let tags = vec![EmojiTag {
            kind: "Emoji".to_string(),
            name: ":blobcat:".to_string(),
            icon: Some(EmojiIcon {
                kind: "Image".to_string(),
                url: Url::parse("https://remote.example/emoji/blobcat.png").unwrap(),
                media_type: Some("image/png".to_string()),
            }),
        }];

        let result = normalize_reaction(":blobcat:", &Some(tags));
        assert_eq!(result, ":blobcat@remote.example:");
    }

    #[test]
    fn test_normalize_custom_emoji_without_tag() {
        let result = normalize_reaction(":blobcat:", &None);
        assert_eq!(result, ":blobcat:");
    }
}
