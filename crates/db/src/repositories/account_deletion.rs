//! Account deletion repository.

use std::sync::Arc;

use crate::entities::{AccountDeletion, account_deletion};
use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

use crate::entities::account_deletion::DeletionStatus;

/// Account deletion repository for database operations.
#[derive(Clone)]
pub struct AccountDeletionRepository {
    db: Arc<DatabaseConnection>,
}

impl AccountDeletionRepository {
    /// Create a new account deletion repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find an account deletion by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<account_deletion::Model>> {
        AccountDeletion::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an account deletion by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<account_deletion::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Account deletion {id} not found")))
    }

    /// Find an account deletion by user ID.
    pub async fn find_by_user_id(
        &self,
        user_id: &str,
    ) -> AppResult<Option<account_deletion::Model>> {
        AccountDeletion::find()
            .filter(account_deletion::Column::UserId.eq(user_id))
            .order_by_desc(account_deletion::Column::CreatedAt)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a pending or scheduled deletion for a user.
    pub async fn find_pending_by_user_id(
        &self,
        user_id: &str,
    ) -> AppResult<Option<account_deletion::Model>> {
        AccountDeletion::find()
            .filter(account_deletion::Column::UserId.eq(user_id))
            .filter(
                account_deletion::Column::Status
                    .eq(DeletionStatus::Scheduled)
                    .or(account_deletion::Column::Status.eq(DeletionStatus::InProgress)),
            )
            .order_by_desc(account_deletion::Column::CreatedAt)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all pending deletions.
    pub async fn find_pending(&self, limit: u64) -> AppResult<Vec<account_deletion::Model>> {
        AccountDeletion::find()
            .filter(account_deletion::Column::Status.eq(DeletionStatus::Scheduled))
            .filter(account_deletion::Column::ScheduledAt.lte(Utc::now()))
            .order_by_asc(account_deletion::Column::ScheduledAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new account deletion.
    pub async fn create(
        &self,
        model: account_deletion::ActiveModel,
    ) -> AppResult<account_deletion::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an account deletion.
    pub async fn update(
        &self,
        model: account_deletion::ActiveModel,
    ) -> AppResult<account_deletion::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Mark an account deletion as in progress.
    pub async fn mark_in_progress(&self, id: &str) -> AppResult<account_deletion::Model> {
        let deletion = self.get_by_id(id).await?;
        let mut active: account_deletion::ActiveModel = deletion.into();
        active.status = Set(DeletionStatus::InProgress);
        self.update(active).await
    }

    /// Mark an account deletion as completed.
    pub async fn mark_completed(&self, id: &str) -> AppResult<account_deletion::Model> {
        let deletion = self.get_by_id(id).await?;
        let mut active: account_deletion::ActiveModel = deletion.into();
        active.status = Set(DeletionStatus::Completed);
        active.completed_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Mark an account deletion as cancelled.
    pub async fn mark_cancelled(&self, id: &str) -> AppResult<account_deletion::Model> {
        let deletion = self.get_by_id(id).await?;
        let mut active: account_deletion::ActiveModel = deletion.into();
        active.status = Set(DeletionStatus::Cancelled);
        self.update(active).await
    }

    /// Delete an account deletion record.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        AccountDeletion::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
