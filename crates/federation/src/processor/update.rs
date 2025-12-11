//! Update activity processor.

use misskey_common::{AppError, AppResult};
use misskey_db::{
    entities::{note, user},
    repositories::{NoteRepository, UserRepository},
};
use sea_orm::Set;
use serde_json::json;
use tracing::info;

use crate::activities::{UpdateActivity, UpdateObject};
use crate::actors::ApPerson;
use crate::objects::ApNote;

/// Result of processing an Update activity.
#[derive(Debug)]
pub enum UpdateResult {
    /// Actor profile was updated.
    ActorUpdated,
    /// Note was updated.
    NoteUpdated,
    /// Unknown object type.
    Ignored,
}

/// Processor for Update activities.
#[derive(Clone)]
pub struct UpdateProcessor {
    user_repo: UserRepository,
    note_repo: NoteRepository,
}

impl UpdateProcessor {
    /// Create a new update processor.
    #[must_use]
    pub const fn new(user_repo: UserRepository, note_repo: NoteRepository) -> Self {
        Self {
            user_repo,
            note_repo,
        }
    }

    /// Process an incoming Update activity.
    pub async fn process(&self, activity: &UpdateActivity) -> AppResult<UpdateResult> {
        info!(
            actor = %activity.actor,
            "Processing Update activity"
        );

        match &activity.object {
            UpdateObject::Person(person) => self.update_actor_from_person(activity, person).await,
            UpdateObject::Note(ap_note) => self.update_note_from_activity(activity, ap_note).await,
            UpdateObject::ObjectUrl(_url) => {
                // Just a URL reference, we'd need to fetch the object
                // For now, just ignore
                info!("Update with URL reference, ignoring");
                Ok(UpdateResult::Ignored)
            }
        }
    }

    /// Update an actor's profile from an embedded Person object.
    async fn update_actor_from_person(
        &self,
        activity: &UpdateActivity,
        person: &ApPerson,
    ) -> AppResult<UpdateResult> {
        // Find the actor
        let actor = self
            .user_repo
            .find_by_uri(activity.actor.as_str())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Actor not found: {}", activity.actor)))?;

        // Build update model
        let mut model: user::ActiveModel = actor.into();

        if let Some(ref name) = person.name {
            model.name = Set(Some(name.clone()));
        }

        if let Some(ref summary) = person.summary {
            model.description = Set(Some(summary.clone()));
        }

        if let Some(ref icon) = person.icon {
            model.avatar_url = Set(Some(icon.url.to_string()));
        }

        if let Some(ref image) = person.image {
            model.banner_url = Set(Some(image.url.to_string()));
        }

        if let Some(manually_approves) = person.manually_approves_followers {
            model.is_locked = Set(manually_approves);
        }

        model.last_fetched_at = Set(Some(chrono::Utc::now().into()));
        model.updated_at = Set(Some(chrono::Utc::now().into()));

        self.user_repo.update(model).await?;

        info!(
            actor = %activity.actor,
            "Actor profile updated"
        );

        Ok(UpdateResult::ActorUpdated)
    }

    /// Update a note from an embedded Note object.
    async fn update_note_from_activity(
        &self,
        activity: &UpdateActivity,
        ap_note: &ApNote,
    ) -> AppResult<UpdateResult> {
        // Verify the actor matches the note's attributedTo
        if activity.actor != ap_note.attributed_to {
            return Err(AppError::BadRequest(
                "Actor does not match note's attributedTo".to_string(),
            ));
        }

        // Find the existing note by URI
        let note_uri = ap_note.id.as_str();
        let existing_note = self
            .note_repo
            .find_by_uri(note_uri)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Note not found: {note_uri}")))?;

        // Only update remote notes (don't allow external updates to local notes)
        if existing_note.is_local {
            return Err(AppError::BadRequest(
                "Cannot update local notes via ActivityPub".to_string(),
            ));
        }

        // Build update model
        let mut model: note::ActiveModel = existing_note.into();

        // Update text content
        model.text = Set(Some(ap_note.content.clone()));

        // Update content warning (summary)
        if let Some(ref summary) = ap_note.summary {
            model.cw = Set(Some(summary.clone()));
        } else {
            model.cw = Set(None);
        }

        // Extract mentions from tags
        let mentions: Vec<String> = ap_note
            .tag
            .as_ref()
            .map(|tags| {
                tags.iter()
                    .filter(|t| t.kind == "Mention")
                    .filter_map(|t| t.href.as_ref().map(|h| h.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Extract hashtags from tags
        let hashtags: Vec<String> = ap_note
            .tag
            .as_ref()
            .map(|tags| {
                tags.iter()
                    .filter(|t| t.kind == "Hashtag")
                    .filter_map(|t| {
                        t.name
                            .as_ref()
                            .map(|n| n.trim_start_matches('#').to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        model.mentions = Set(json!(mentions));
        model.tags = Set(json!(hashtags));

        // Update timestamp
        model.updated_at = Set(Some(chrono::Utc::now().into()));

        self.note_repo.update(model).await?;

        info!(
            note_uri = %note_uri,
            actor = %activity.actor,
            "Remote note updated"
        );

        Ok(UpdateResult::NoteUpdated)
    }
}
