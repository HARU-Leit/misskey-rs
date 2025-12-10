//! Drive file repository.

use std::sync::Arc;

use crate::entities::{drive_file, DriveFile};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// Drive file repository for database operations.
#[derive(Clone)]
pub struct DriveFileRepository {
    db: Arc<DatabaseConnection>,
}

impl DriveFileRepository {
    /// Create a new drive file repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a file by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<drive_file::Model>> {
        DriveFile::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a file by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<drive_file::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("DriveFile: {id}")))
    }

    /// Find files by IDs.
    pub async fn find_by_ids(&self, ids: &[String]) -> AppResult<Vec<drive_file::Model>> {
        DriveFile::find()
            .filter(drive_file::Column::Id.is_in(ids.to_vec()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a file by MD5 hash.
    pub async fn find_by_md5(&self, md5: &str) -> AppResult<Option<drive_file::Model>> {
        DriveFile::find()
            .filter(drive_file::Column::Md5.eq(md5))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a file by URI.
    pub async fn find_by_uri(&self, uri: &str) -> AppResult<Option<drive_file::Model>> {
        DriveFile::find()
            .filter(drive_file::Column::Uri.eq(uri))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new file.
    pub async fn create(&self, model: drive_file::ActiveModel) -> AppResult<drive_file::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a file.
    pub async fn update(&self, model: drive_file::ActiveModel) -> AppResult<drive_file::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a file.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let file = self.find_by_id(id).await?;
        if let Some(f) = file {
            f.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get files for a user (paginated).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
        folder_id: Option<&str>,
    ) -> AppResult<Vec<drive_file::Model>> {
        let mut query = DriveFile::find()
            .filter(drive_file::Column::UserId.eq(user_id))
            .order_by_desc(drive_file::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(drive_file::Column::Id.lt(id));
        }

        if let Some(fid) = folder_id {
            query = query.filter(drive_file::Column::FolderId.eq(fid));
        } else {
            query = query.filter(drive_file::Column::FolderId.is_null());
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Calculate total storage used by a user.
    pub async fn get_storage_used(&self, user_id: &str) -> AppResult<i64> {
        use sea_orm::FromQueryResult;

        #[derive(FromQueryResult)]
        struct SumResult {
            total: Option<i64>,
        }

        let result = DriveFile::find()
            .filter(drive_file::Column::UserId.eq(user_id))
            .filter(drive_file::Column::IsLink.eq(false))
            .select_only()
            .column_as(drive_file::Column::Size.sum(), "total")
            .into_model::<SumResult>()
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.and_then(|r| r.total).unwrap_or(0))
    }
}
