//! Import job repository.

use std::sync::Arc;

use crate::entities::{ImportJob, import_job};
use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use crate::entities::import_job::ImportStatus;

/// Import job repository for database operations.
#[derive(Clone)]
pub struct ImportJobRepository {
    db: Arc<DatabaseConnection>,
}

impl ImportJobRepository {
    /// Create a new import job repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find an import job by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<import_job::Model>> {
        ImportJob::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an import job by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<import_job::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Import job {id} not found")))
    }

    /// Find an import job by ID and verify ownership.
    pub async fn find_by_id_and_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<Option<import_job::Model>> {
        ImportJob::find_by_id(id)
            .filter(import_job::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an import job by ID and verify ownership, returning an error if not found.
    pub async fn get_by_id_and_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<import_job::Model> {
        self.find_by_id_and_user(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Import job {id} not found")))
    }

    /// Find all import jobs for a user.
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<import_job::Model>> {
        ImportJob::find()
            .filter(import_job::Column::UserId.eq(user_id))
            .order_by_desc(import_job::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count import jobs for a user.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        ImportJob::find()
            .filter(import_job::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find jobs that are due for processing.
    pub async fn find_pending(&self, limit: u64) -> AppResult<Vec<import_job::Model>> {
        ImportJob::find()
            .filter(import_job::Column::Status.eq(ImportStatus::Queued))
            .order_by_asc(import_job::Column::CreatedAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new import job.
    pub async fn create(&self, model: import_job::ActiveModel) -> AppResult<import_job::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an import job.
    pub async fn update(&self, model: import_job::ActiveModel) -> AppResult<import_job::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Mark an import job as validating.
    pub async fn mark_validating(&self, id: &str) -> AppResult<import_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: import_job::ActiveModel = job.into();
        active.status = Set(ImportStatus::Validating);
        self.update(active).await
    }

    /// Mark an import job as processing.
    pub async fn mark_processing(&self, id: &str) -> AppResult<import_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: import_job::ActiveModel = job.into();
        active.status = Set(ImportStatus::Processing);
        self.update(active).await
    }

    /// Update progress of an import job (progress only).
    pub async fn update_progress(&self, id: &str, progress: i32) -> AppResult<import_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: import_job::ActiveModel = job.into();
        active.progress = Set(progress);
        self.update(active).await
    }

    /// Update progress of an import job with item counts.
    pub async fn update_progress_with_counts(
        &self,
        id: &str,
        progress: i32,
        imported: i32,
        skipped: i32,
        failed: i32,
    ) -> AppResult<import_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: import_job::ActiveModel = job.into();
        active.progress = Set(progress);
        active.imported_items = Set(imported);
        active.skipped_items = Set(skipped);
        active.failed_items = Set(failed);
        self.update(active).await
    }

    /// Mark an import job as completed.
    pub async fn mark_completed(
        &self,
        id: &str,
        imported: i32,
        skipped: i32,
        failed: i32,
    ) -> AppResult<import_job::Model> {
        let job = self.get_by_id(id).await?;
        let status = if failed > 0 {
            ImportStatus::PartiallyCompleted
        } else {
            ImportStatus::Completed
        };
        let mut active: import_job::ActiveModel = job.into();
        active.status = Set(status);
        active.progress = Set(100);
        active.imported_items = Set(imported);
        active.skipped_items = Set(skipped);
        active.failed_items = Set(failed);
        active.completed_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Mark an import job as failed.
    pub async fn mark_failed(&self, id: &str, error_message: &str) -> AppResult<import_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: import_job::ActiveModel = job.into();
        active.status = Set(ImportStatus::Failed);
        active.error_message = Set(Some(error_message.to_string()));
        active.completed_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Delete an import job.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        ImportJob::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
