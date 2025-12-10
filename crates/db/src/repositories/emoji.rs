//! Emoji repository.

use std::sync::Arc;

use crate::entities::{emoji, Emoji};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

/// Emoji repository for database operations.
#[derive(Clone)]
pub struct EmojiRepository {
    db: Arc<DatabaseConnection>,
}

impl EmojiRepository {
    /// Create a new emoji repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find an emoji by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<emoji::Model>> {
        Emoji::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find an emoji by name (shortcode).
    pub async fn find_by_name(&self, name: &str) -> AppResult<Option<emoji::Model>> {
        Emoji::find()
            .filter(emoji::Column::Name.eq(name))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find an emoji by name and host.
    pub async fn find_by_name_and_host(
        &self,
        name: &str,
        host: Option<&str>,
    ) -> AppResult<Option<emoji::Model>> {
        let mut query = Emoji::find().filter(emoji::Column::Name.eq(name));

        query = match host {
            Some(h) => query.filter(emoji::Column::Host.eq(h)),
            None => query.filter(emoji::Column::Host.is_null()),
        };

        query
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get all local emojis.
    pub async fn find_local(&self) -> AppResult<Vec<emoji::Model>> {
        Emoji::find()
            .filter(emoji::Column::Host.is_null())
            .order_by_asc(emoji::Column::Category)
            .order_by_asc(emoji::Column::Name)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get local emojis by category.
    pub async fn find_by_category(&self, category: &str) -> AppResult<Vec<emoji::Model>> {
        Emoji::find()
            .filter(emoji::Column::Host.is_null())
            .filter(emoji::Column::Category.eq(category))
            .order_by_asc(emoji::Column::Name)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get local emojis with pagination.
    pub async fn find_local_paginated(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<emoji::Model>> {
        Emoji::find()
            .filter(emoji::Column::Host.is_null())
            .order_by_asc(emoji::Column::Category)
            .order_by_asc(emoji::Column::Name)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get all unique categories.
    pub async fn find_categories(&self) -> AppResult<Vec<String>> {
        let emojis = Emoji::find()
            .filter(emoji::Column::Host.is_null())
            .filter(emoji::Column::Category.is_not_null())
            .select_only()
            .column(emoji::Column::Category)
            .distinct()
            .into_tuple::<Option<String>>()
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(emojis.into_iter().flatten().collect())
    }

    /// Search emojis by name.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<emoji::Model>> {
        let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));

        Emoji::find()
            .filter(emoji::Column::Host.is_null())
            .filter(emoji::Column::Name.like(&pattern))
            .order_by_asc(emoji::Column::Name)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count local emojis.
    pub async fn count_local(&self) -> AppResult<u64> {
        Emoji::find()
            .filter(emoji::Column::Host.is_null())
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new emoji.
    pub async fn create(&self, model: emoji::ActiveModel) -> AppResult<emoji::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an emoji.
    pub async fn update(&self, model: emoji::ActiveModel) -> AppResult<emoji::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete an emoji.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let emoji = self.find_by_id(id).await?;
        if let Some(e) = emoji {
            e.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Find emojis by multiple names.
    pub async fn find_by_names(&self, names: &[String]) -> AppResult<Vec<emoji::Model>> {
        if names.is_empty() {
            return Ok(vec![]);
        }

        Emoji::find()
            .filter(emoji::Column::Name.is_in(names.to_vec()))
            .filter(emoji::Column::Host.is_null())
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Import an emoji from a remote instance.
    pub async fn import_remote(
        &self,
        model: emoji::ActiveModel,
    ) -> AppResult<emoji::Model> {
        // Check if already exists
        let name = model.name.clone().unwrap();
        let host = model.host.clone().unwrap();

        if let Some(existing) = self.find_by_name_and_host(&name, host.as_deref()).await? {
            return Ok(existing);
        }

        self.create(model).await
    }
}
