//! Delete activity processor.

use misskey_common::{AppError, AppResult};
use misskey_db::repositories::{NoteRepository, UserRepository};
use tracing::info;

use crate::activities::DeleteActivity;

/// Result of processing a Delete activity.
#[derive(Debug)]
pub enum DeleteResult {
    /// Note was deleted.
    NoteDeleted,
    /// Actor was deleted (account deletion).
    ActorDeleted,
    /// Object not found, nothing to delete.
    NotFound,
}

/// Processor for Delete activities.
#[derive(Clone)]
pub struct DeleteProcessor {
    user_repo: UserRepository,
    note_repo: NoteRepository,
}

impl DeleteProcessor {
    /// Create a new delete processor.
    #[must_use] 
    pub const fn new(user_repo: UserRepository, note_repo: NoteRepository) -> Self {
        Self {
            user_repo,
            note_repo,
        }
    }

    /// Process an incoming Delete activity.
    pub async fn process(&self, activity: &DeleteActivity) -> AppResult<DeleteResult> {
        info!(
            actor = %activity.actor,
            object = %activity.object,
            "Processing Delete activity"
        );

        // Verify the actor owns the object being deleted
        let actor = self.user_repo.find_by_uri(activity.actor.as_str()).await?;

        // Try to delete as a note first
        if let Some(note) = self.note_repo.find_by_uri(activity.object.as_str()).await? {
            // Verify ownership
            if let Some(ref actor) = actor
                && note.user_id != actor.id {
                    return Err(AppError::Forbidden(
                        "Actor does not own this note".to_string(),
                    ));
                }

            // Delete the note
            self.note_repo.delete(&note.id).await?;

            info!(
                note_id = %note.id,
                "Note deleted from remote"
            );

            return Ok(DeleteResult::NoteDeleted);
        }

        // Try to delete as an actor (account deletion)
        if activity.actor == activity.object
            && let Some(actor) = actor {
                // Mark user as deleted/suspended
                self.user_repo.mark_as_deleted(&actor.id).await?;

                info!(
                    user_id = %actor.id,
                    "Remote user deleted"
                );

                return Ok(DeleteResult::ActorDeleted);
            }

        info!(
            object = %activity.object,
            "Delete target not found"
        );

        Ok(DeleteResult::NotFound)
    }
}
