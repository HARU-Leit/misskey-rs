//! Update activity processor.

use misskey_common::{AppError, AppResult};
use misskey_db::{
    entities::user,
    repositories::UserRepository,
};
use sea_orm::Set;
use tracing::info;

use crate::activities::{UpdateActivity, UpdateObject};
use crate::actors::ApPerson;

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
}

impl UpdateProcessor {
    /// Create a new update processor.
    #[must_use] 
    pub const fn new(user_repo: UserRepository) -> Self {
        Self { user_repo }
    }

    /// Process an incoming Update activity.
    pub async fn process(&self, activity: &UpdateActivity) -> AppResult<UpdateResult> {
        info!(
            actor = %activity.actor,
            "Processing Update activity"
        );

        match &activity.object {
            UpdateObject::Person(person) => self.update_actor_from_person(activity, person).await,
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
}
