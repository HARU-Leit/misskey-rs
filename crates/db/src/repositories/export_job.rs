//! Export job repository.

use std::sync::Arc;

use crate::entities::{ExportJob, export_job};
use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use crate::entities::export_job::ExportStatus;

/// Export job repository for database operations.
#[derive(Clone)]
pub struct ExportJobRepository {
    db: Arc<DatabaseConnection>,
}

impl ExportJobRepository {
    /// Create a new export job repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find an export job by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<export_job::Model>> {
        ExportJob::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an export job by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<export_job::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Export job {id} not found")))
    }

    /// Find an export job by ID and verify ownership.
    pub async fn find_by_id_and_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<Option<export_job::Model>> {
        ExportJob::find_by_id(id)
            .filter(export_job::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an export job by ID and verify ownership, returning an error if not found.
    pub async fn get_by_id_and_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<export_job::Model> {
        self.find_by_id_and_user(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Export job {id} not found")))
    }

    /// Find all export jobs for a user.
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<export_job::Model>> {
        ExportJob::find()
            .filter(export_job::Column::UserId.eq(user_id))
            .order_by_desc(export_job::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find pending export jobs for a user.
    pub async fn find_pending_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<export_job::Model>> {
        ExportJob::find()
            .filter(export_job::Column::UserId.eq(user_id))
            .filter(export_job::Column::Status.eq(ExportStatus::Pending))
            .order_by_desc(export_job::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count export jobs for a user.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        ExportJob::find()
            .filter(export_job::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find jobs that are due for processing.
    pub async fn find_pending(&self, limit: u64) -> AppResult<Vec<export_job::Model>> {
        ExportJob::find()
            .filter(export_job::Column::Status.eq(ExportStatus::Pending))
            .order_by_asc(export_job::Column::CreatedAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new export job.
    pub async fn create(&self, model: export_job::ActiveModel) -> AppResult<export_job::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an export job.
    pub async fn update(&self, model: export_job::ActiveModel) -> AppResult<export_job::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Mark an export job as processing.
    pub async fn mark_processing(&self, id: &str) -> AppResult<export_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: export_job::ActiveModel = job.into();
        active.status = Set(ExportStatus::Processing);
        self.update(active).await
    }

    /// Update progress of an export job.
    pub async fn update_progress(&self, id: &str, progress: i32) -> AppResult<export_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: export_job::ActiveModel = job.into();
        active.progress = Set(progress);
        self.update(active).await
    }

    /// Mark an export job as completed with download URL and expiry.
    pub async fn mark_completed_with_url(
        &self,
        id: &str,
        download_url: &str,
        file_path: Option<&str>,
        expires_at: chrono::DateTime<Utc>,
    ) -> AppResult<export_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: export_job::ActiveModel = job.into();
        active.status = Set(ExportStatus::Completed);
        active.progress = Set(100);
        active.download_url = Set(Some(download_url.to_string()));
        active.file_path = Set(file_path.map(std::string::ToString::to_string));
        active.completed_at = Set(Some(Utc::now().into()));
        active.expires_at = Set(Some(expires_at.into()));
        self.update(active).await
    }

    /// Mark an export job as completed (optionally with download URL).
    pub async fn mark_completed(
        &self,
        id: &str,
        download_url: Option<&str>,
        file_path: Option<&str>,
    ) -> AppResult<export_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: export_job::ActiveModel = job.into();
        active.status = Set(ExportStatus::Completed);
        active.progress = Set(100);
        active.download_url = Set(download_url.map(std::string::ToString::to_string));
        active.file_path = Set(file_path.map(std::string::ToString::to_string));
        active.completed_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Mark an export job as failed.
    pub async fn mark_failed(&self, id: &str, error_message: &str) -> AppResult<export_job::Model> {
        let job = self.get_by_id(id).await?;
        let mut active: export_job::ActiveModel = job.into();
        active.status = Set(ExportStatus::Failed);
        active.error_message = Set(Some(error_message.to_string()));
        active.completed_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Delete an export job.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        ExportJob::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete expired export jobs (cleanup).
    pub async fn delete_expired(&self) -> AppResult<u64> {
        let now = Utc::now();

        let result = ExportJob::delete_many()
            .filter(export_job::Column::ExpiresAt.lt(now))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
