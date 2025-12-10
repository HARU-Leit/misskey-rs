//! Blocking repository.

use std::sync::Arc;

use crate::entities::{blocking, Blocking};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// Blocking repository for database operations.
#[derive(Clone)]
pub struct BlockingRepository {
    db: Arc<DatabaseConnection>,
}

impl BlockingRepository {
    /// Create a new blocking repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a blocking relationship by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<blocking::Model>> {
        Blocking::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a blocking relationship by blocker and blockee.
    pub async fn find_by_pair(
        &self,
        blocker_id: &str,
        blockee_id: &str,
    ) -> AppResult<Option<blocking::Model>> {
        Blocking::find()
            .filter(blocking::Column::BlockerId.eq(blocker_id))
            .filter(blocking::Column::BlockeeId.eq(blockee_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a user is blocking another user.
    pub async fn is_blocking(&self, blocker_id: &str, blockee_id: &str) -> AppResult<bool> {
        Ok(self.find_by_pair(blocker_id, blockee_id).await?.is_some())
    }

    /// Check if either user is blocking the other.
    pub async fn is_blocked_between(&self, user_a: &str, user_b: &str) -> AppResult<bool> {
        Ok(self.is_blocking(user_a, user_b).await? || self.is_blocking(user_b, user_a).await?)
    }

    /// Create a new blocking relationship.
    pub async fn create(&self, model: blocking::ActiveModel) -> AppResult<blocking::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a blocking relationship.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let blocking = self.find_by_id(id).await?;
        if let Some(b) = blocking {
            b.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Delete a blocking relationship by pair.
    pub async fn delete_by_pair(&self, blocker_id: &str, blockee_id: &str) -> AppResult<()> {
        let blocking = self.find_by_pair(blocker_id, blockee_id).await?;
        if let Some(b) = blocking {
            b.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get users that a user is blocking (paginated).
    pub async fn find_blocking(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<blocking::Model>> {
        let mut query = Blocking::find()
            .filter(blocking::Column::BlockerId.eq(user_id))
            .order_by_desc(blocking::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(blocking::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
