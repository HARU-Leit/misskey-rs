//! Security key repository for WebAuthn/Passkey operations.

use std::sync::Arc;

use crate::entities::{SecurityKey, security_key};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

/// Maximum number of security keys per user.
pub const MAX_SECURITY_KEYS_PER_USER: usize = 10;

/// Security key repository for database operations.
#[derive(Clone)]
pub struct SecurityKeyRepository {
    db: Arc<DatabaseConnection>,
}

impl SecurityKeyRepository {
    /// Create a new security key repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a security key by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<security_key::Model>> {
        SecurityKey::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a security key by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<security_key::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("SecurityKey: {id}")))
    }

    /// Find a security key by credential ID.
    pub async fn find_by_credential_id(
        &self,
        credential_id: &str,
    ) -> AppResult<Option<security_key::Model>> {
        SecurityKey::find()
            .filter(security_key::Column::CredentialId.eq(credential_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all security keys for a user.
    pub async fn find_by_user_id(&self, user_id: &str) -> AppResult<Vec<security_key::Model>> {
        SecurityKey::find()
            .filter(security_key::Column::UserId.eq(user_id))
            .order_by_asc(security_key::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count security keys for a user.
    pub async fn count_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        SecurityKey::find()
            .filter(security_key::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find passkeys for a user (resident credentials).
    pub async fn find_passkeys_by_user_id(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<security_key::Model>> {
        SecurityKey::find()
            .filter(security_key::Column::UserId.eq(user_id))
            .filter(security_key::Column::IsPasskey.eq(true))
            .order_by_asc(security_key::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new security key.
    pub async fn create(&self, model: security_key::ActiveModel) -> AppResult<security_key::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a security key.
    pub async fn update(&self, model: security_key::ActiveModel) -> AppResult<security_key::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update the counter for a security key (for replay protection).
    pub async fn update_counter(&self, id: &str, counter: i64) -> AppResult<()> {
        let key = self.get_by_id(id).await?;
        let mut active: security_key::ActiveModel = key.into();
        active.counter = Set(counter);
        active.last_used_at = Set(Some(chrono::Utc::now().into()));
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Update the name of a security key.
    pub async fn update_name(&self, id: &str, user_id: &str, name: &str) -> AppResult<()> {
        let key = self.get_by_id(id).await?;

        // Verify ownership
        if key.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only rename your own security keys".to_string(),
            ));
        }

        let mut active: security_key::ActiveModel = key.into();
        active.name = Set(name.to_string());
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a security key.
    pub async fn delete(&self, id: &str, user_id: &str) -> AppResult<()> {
        let key = self.get_by_id(id).await?;

        // Verify ownership
        if key.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only delete your own security keys".to_string(),
            ));
        }

        SecurityKey::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete all security keys for a user.
    pub async fn delete_all_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        let result = SecurityKey::delete_many()
            .filter(security_key::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result.rows_affected)
    }

    /// Check if a user has any security keys.
    pub async fn user_has_security_keys(&self, user_id: &str) -> AppResult<bool> {
        let count = self.count_by_user_id(user_id).await?;
        Ok(count > 0)
    }

    /// Check if a user has reached the maximum number of security keys.
    pub async fn user_at_limit(&self, user_id: &str) -> AppResult<bool> {
        let count = self.count_by_user_id(user_id).await?;
        Ok(count as usize >= MAX_SECURITY_KEYS_PER_USER)
    }
}
