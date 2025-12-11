//! Muting repository.

use std::sync::Arc;

use crate::entities::{Muting, muting};
use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// Muting repository for database operations.
#[derive(Clone)]
pub struct MutingRepository {
    db: Arc<DatabaseConnection>,
}

impl MutingRepository {
    /// Create a new muting repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a muting relationship by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<muting::Model>> {
        Muting::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a muting relationship by muter and mutee.
    pub async fn find_by_pair(
        &self,
        muter_id: &str,
        mutee_id: &str,
    ) -> AppResult<Option<muting::Model>> {
        Muting::find()
            .filter(muting::Column::MuterId.eq(muter_id))
            .filter(muting::Column::MuteeId.eq(mutee_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a user is muting another user (considering expiration).
    pub async fn is_muting(&self, muter_id: &str, mutee_id: &str) -> AppResult<bool> {
        let muting = self.find_by_pair(muter_id, mutee_id).await?;
        match muting {
            Some(m) => {
                // Check if mute has expired
                if let Some(expires_at) = m.expires_at {
                    Ok(expires_at > Utc::now().fixed_offset())
                } else {
                    // Permanent mute
                    Ok(true)
                }
            }
            None => Ok(false),
        }
    }

    /// Create a new muting relationship.
    pub async fn create(&self, model: muting::ActiveModel) -> AppResult<muting::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a muting relationship.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let muting = self.find_by_id(id).await?;
        if let Some(m) = muting {
            m.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Delete a muting relationship by pair.
    pub async fn delete_by_pair(&self, muter_id: &str, mutee_id: &str) -> AppResult<()> {
        let muting = self.find_by_pair(muter_id, mutee_id).await?;
        if let Some(m) = muting {
            m.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get users that a user is muting (paginated).
    pub async fn find_muting(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<muting::Model>> {
        let mut query = Muting::find()
            .filter(muting::Column::MuterId.eq(user_id))
            .order_by_desc(muting::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(muting::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete expired mutes.
    pub async fn delete_expired(&self) -> AppResult<u64> {
        use sea_orm::DeleteResult;

        let result: DeleteResult = Muting::delete_many()
            .filter(muting::Column::ExpiresAt.is_not_null())
            .filter(muting::Column::ExpiresAt.lt(Utc::now().fixed_offset()))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
