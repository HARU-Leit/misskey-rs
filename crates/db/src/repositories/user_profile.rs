//! User profile repository.

use std::sync::Arc;

use crate::entities::{user_profile, UserProfile};
use misskey_common::{AppError, AppResult};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;

/// User profile repository for database operations.
#[derive(Clone)]
pub struct UserProfileRepository {
    db: Arc<DatabaseConnection>,
}

impl UserProfileRepository {
    /// Create a new user profile repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a user profile by user ID.
    pub async fn find_by_user_id(&self, user_id: &str) -> AppResult<Option<user_profile::Model>> {
        UserProfile::find_by_id(user_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a user profile by user ID, returning an error if not found.
    pub async fn get_by_user_id(&self, user_id: &str) -> AppResult<user_profile::Model> {
        self.find_by_user_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("UserProfile: {user_id}")))
    }

    /// Create a new user profile.
    pub async fn create(&self, model: user_profile::ActiveModel) -> AppResult<user_profile::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a user profile.
    pub async fn update(&self, model: user_profile::ActiveModel) -> AppResult<user_profile::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update password hash for a user.
    pub async fn update_password(&self, user_id: &str, password_hash: &str) -> AppResult<()> {
        let profile = self.get_by_user_id(user_id).await?;
        let mut active: user_profile::ActiveModel = profile.into();
        active.password = Set(Some(password_hash.to_string()));
        active.update(self.db.as_ref()).await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Verify password for a user (returns the profile if password matches).
    pub async fn verify_password(
        &self,
        user_id: &str,
        password: &str,
    ) -> AppResult<Option<user_profile::Model>> {
        let profile = self.find_by_user_id(user_id).await?;

        match profile {
            Some(p) => {
                if let Some(ref hash) = p.password {
                    use argon2::{Argon2, PasswordHash, PasswordVerifier};
                    let parsed_hash = PasswordHash::new(hash)
                        .map_err(|e| AppError::Internal(format!("Invalid password hash: {e}")))?;

                    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                        Ok(Some(p))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Get pinned note IDs for a user.
    pub async fn get_pinned_note_ids(&self, user_id: &str) -> AppResult<Vec<String>> {
        let profile = self.get_by_user_id(user_id).await?;
        let ids: Vec<String> = serde_json::from_value(profile.pinned_note_ids)
            .unwrap_or_default();
        Ok(ids)
    }

    /// Pin a note to user's profile.
    pub async fn pin_note(&self, user_id: &str, note_id: &str, max_pins: usize) -> AppResult<Vec<String>> {
        let profile = self.get_by_user_id(user_id).await?;
        let mut pinned: Vec<String> = serde_json::from_value(profile.pinned_note_ids.clone())
            .unwrap_or_default();

        // Check if already pinned
        if pinned.contains(&note_id.to_string()) {
            return Ok(pinned);
        }

        // Check max pins limit
        if pinned.len() >= max_pins {
            return Err(AppError::BadRequest(format!(
                "Cannot pin more than {max_pins} notes"
            )));
        }

        // Add to pinned list
        pinned.push(note_id.to_string());

        // Update profile
        let mut active: user_profile::ActiveModel = profile.into();
        active.pinned_note_ids = Set(json!(pinned));
        active.update(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(pinned)
    }

    /// Unpin a note from user's profile.
    pub async fn unpin_note(&self, user_id: &str, note_id: &str) -> AppResult<Vec<String>> {
        let profile = self.get_by_user_id(user_id).await?;
        let mut pinned: Vec<String> = serde_json::from_value(profile.pinned_note_ids.clone())
            .unwrap_or_default();

        // Remove from pinned list
        pinned.retain(|id| id != note_id);

        // Update profile
        let mut active: user_profile::ActiveModel = profile.into();
        active.pinned_note_ids = Set(json!(pinned));
        active.update(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(pinned)
    }

    /// Reorder pinned notes.
    pub async fn reorder_pinned_notes(&self, user_id: &str, note_ids: Vec<String>) -> AppResult<()> {
        let profile = self.get_by_user_id(user_id).await?;
        let current: Vec<String> = serde_json::from_value(profile.pinned_note_ids.clone())
            .unwrap_or_default();

        // Validate that all IDs in new order exist in current pinned list
        for id in &note_ids {
            if !current.contains(id) {
                return Err(AppError::BadRequest(format!(
                    "Note {id} is not pinned"
                )));
            }
        }

        // Update with new order
        let mut active: user_profile::ActiveModel = profile.into();
        active.pinned_note_ids = Set(json!(note_ids));
        active.update(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
