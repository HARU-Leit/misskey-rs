//! Drive folder repository.

use std::sync::Arc;

use crate::entities::{drive_folder, DriveFolder};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// Drive folder repository for database operations.
#[derive(Clone)]
pub struct DriveFolderRepository {
    db: Arc<DatabaseConnection>,
}

impl DriveFolderRepository {
    /// Create a new drive folder repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a folder by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<drive_folder::Model>> {
        DriveFolder::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a folder by ID, returning error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<drive_folder::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Folder not found: {id}")))
    }

    /// Create a new folder.
    pub async fn create(
        &self,
        model: drive_folder::ActiveModel,
    ) -> AppResult<drive_folder::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a folder.
    pub async fn update(
        &self,
        model: drive_folder::ActiveModel,
    ) -> AppResult<drive_folder::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a folder.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let folder = self.find_by_id(id).await?;
        if let Some(f) = folder {
            f.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Find folders by user ID.
    pub async fn find_by_user(
        &self,
        user_id: &str,
        parent_id: Option<&str>,
        limit: u64,
    ) -> AppResult<Vec<drive_folder::Model>> {
        let mut query = DriveFolder::find()
            .filter(drive_folder::Column::UserId.eq(user_id))
            .order_by_asc(drive_folder::Column::Name)
            .limit(limit);

        if let Some(parent) = parent_id {
            query = query.filter(drive_folder::Column::ParentId.eq(parent));
        } else {
            query = query.filter(drive_folder::Column::ParentId.is_null());
        }

        query
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find folder by name in a parent folder.
    pub async fn find_by_name(
        &self,
        user_id: &str,
        name: &str,
        parent_id: Option<&str>,
    ) -> AppResult<Option<drive_folder::Model>> {
        let mut query = DriveFolder::find()
            .filter(drive_folder::Column::UserId.eq(user_id))
            .filter(drive_folder::Column::Name.eq(name));

        if let Some(parent) = parent_id {
            query = query.filter(drive_folder::Column::ParentId.eq(parent));
        } else {
            query = query.filter(drive_folder::Column::ParentId.is_null());
        }

        query
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
