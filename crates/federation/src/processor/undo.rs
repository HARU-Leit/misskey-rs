//! Undo activity processor.

use misskey_common::{AppError, AppResult};
use misskey_db::repositories::{
    FollowingRepository, NoteRepository, ReactionRepository, UserRepository,
};
use tracing::info;
use url::Url;

/// Parsed Undo activity with resolved inner activity details.
#[derive(Debug, Clone)]
pub struct ParsedUndoActivity {
    pub id: Url,
    pub actor: Url,
    /// The type of activity being undone (Follow, Like, Announce).
    pub object_type: String,
    /// The ID of the activity being undone.
    pub object_id: Url,
    /// For Undo Follow: the followee URL.
    /// For Undo Like: the note URL.
    pub object_object: Option<Url>,
}

/// Result of processing an Undo activity.
#[derive(Debug)]
pub enum UndoResult {
    /// Follow was undone.
    Unfollowed,
    /// Like was undone.
    Unreacted,
    /// Announce was undone.
    Unrenoted,
    /// Unknown object type, ignored.
    Ignored,
}

/// Processor for Undo activities.
#[derive(Clone)]
pub struct UndoProcessor {
    user_repo: UserRepository,
    following_repo: FollowingRepository,
    reaction_repo: ReactionRepository,
    note_repo: NoteRepository,
}

impl UndoProcessor {
    /// Create a new undo processor.
    #[must_use]
    pub const fn new(
        user_repo: UserRepository,
        following_repo: FollowingRepository,
        reaction_repo: ReactionRepository,
        note_repo: NoteRepository,
    ) -> Self {
        Self {
            user_repo,
            following_repo,
            reaction_repo,
            note_repo,
        }
    }

    /// Process an incoming Undo activity.
    pub async fn process(&self, activity: &ParsedUndoActivity) -> AppResult<UndoResult> {
        info!(
            actor = %activity.actor,
            object_type = %activity.object_type,
            "Processing Undo activity"
        );

        match activity.object_type.as_str() {
            "Follow" => self.undo_follow(activity).await,
            "Like" => self.undo_like(activity).await,
            "Announce" => self.undo_announce(activity).await,
            _ => {
                info!(object_type = %activity.object_type, "Unknown Undo object type, ignoring");
                Ok(UndoResult::Ignored)
            }
        }
    }

    /// Undo a Follow activity.
    async fn undo_follow(&self, activity: &ParsedUndoActivity) -> AppResult<UndoResult> {
        // Find the actor (follower)
        let follower = self
            .user_repo
            .find_by_uri(activity.actor.as_str())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Actor not found: {}", activity.actor)))?;

        // Get the followee URL from object.object
        let followee_url = activity
            .object_object
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("Undo Follow missing object.object".to_string()))?;

        // Extract local user ID from followee URL
        let followee_id = extract_local_user_id(followee_url)?;
        let followee = self
            .user_repo
            .find_by_id(&followee_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Followee not found: {followee_id}")))?;

        // Check if following exists
        if !self
            .following_repo
            .is_following(&follower.id, &followee.id)
            .await?
        {
            info!("Follow relationship doesn't exist, nothing to undo");
            return Ok(UndoResult::Unfollowed);
        }

        // Delete the following relationship
        self.following_repo
            .delete_by_pair(&follower.id, &followee.id)
            .await?;

        // Update counts
        self.user_repo
            .decrement_following_count(&follower.id)
            .await?;
        self.user_repo
            .decrement_followers_count(&followee.id)
            .await?;

        info!(
            follower = %follower.id,
            followee = %followee.id,
            "Unfollowed"
        );

        Ok(UndoResult::Unfollowed)
    }

    /// Undo a Like activity.
    async fn undo_like(&self, activity: &ParsedUndoActivity) -> AppResult<UndoResult> {
        // Find the actor
        let actor = self
            .user_repo
            .find_by_uri(activity.actor.as_str())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Actor not found: {}", activity.actor)))?;

        // The object_id is the Like activity ID, but we need to find the note
        // In practice, we should look up the Like by its activity ID
        // For now, we'll try to find by object.object if available
        let note_url = activity
            .object_object
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("Undo Like missing note reference".to_string()))?;

        let note = self
            .note_repo
            .find_by_uri(note_url.as_str())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Note not found: {note_url}")))?;

        // Delete the reaction
        if self
            .reaction_repo
            .find_by_user_and_note(&actor.id, &note.id)
            .await?
            .is_some()
        {
            self.reaction_repo
                .delete_by_user_and_note(&actor.id, &note.id)
                .await?;

            info!(
                actor = %actor.id,
                note = %note.id,
                "Reaction removed"
            );
        }

        Ok(UndoResult::Unreacted)
    }

    /// Undo an Announce activity.
    async fn undo_announce(&self, activity: &ParsedUndoActivity) -> AppResult<UndoResult> {
        // Find the renote by its URI (activity ID)
        if let Some(renote) = self
            .note_repo
            .find_by_uri(activity.object_id.as_str())
            .await?
        {
            // Delete the renote
            self.note_repo.delete(&renote.id).await?;

            // Decrement renote count on original note
            if let Some(ref original_id) = renote.renote_id {
                self.note_repo.decrement_renote_count(original_id).await?;
            }

            info!(
                renote_id = %renote.id,
                "Renote removed"
            );
        }

        Ok(UndoResult::Unrenoted)
    }
}

/// Extract local user ID from a URL.
fn extract_local_user_id(url: &Url) -> AppResult<String> {
    let path = url.path();
    if let Some(id) = path.strip_prefix("/users/") {
        return Ok(id.to_string());
    }
    Err(AppError::BadRequest(format!(
        "Cannot extract user ID from URL: {url}"
    )))
}
