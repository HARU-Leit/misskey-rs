//! Page repository.

use std::sync::Arc;

use crate::entities::{page, page_like, Page, PageLike};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

/// Maximum number of pages per user.
pub const MAX_PAGES_PER_USER: usize = 100;

/// Page repository for database operations.
#[derive(Clone)]
pub struct PageRepository {
    db: Arc<DatabaseConnection>,
}

impl PageRepository {
    /// Create a new page repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a page by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<page::Model>> {
        Page::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a page by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<page::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Page: {id}")))
    }

    /// Find a page by user ID and name (unique identifier).
    pub async fn find_by_user_and_name(
        &self,
        user_id: &str,
        name: &str,
    ) -> AppResult<Option<page::Model>> {
        Page::find()
            .filter(page::Column::UserId.eq(user_id))
            .filter(page::Column::Name.eq(name))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all pages for a user.
    pub async fn find_by_user_id(&self, user_id: &str) -> AppResult<Vec<page::Model>> {
        Page::find()
            .filter(page::Column::UserId.eq(user_id))
            .order_by_desc(page::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find pages with pagination.
    pub async fn find_with_pagination(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<page::Model>> {
        Page::find()
            .filter(page::Column::Visibility.eq("public"))
            .order_by_desc(page::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find featured pages (most liked).
    pub async fn find_featured(&self, limit: u64) -> AppResult<Vec<page::Model>> {
        Page::find()
            .filter(page::Column::Visibility.eq("public"))
            .order_by_desc(page::Column::LikedCount)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count pages for a user.
    pub async fn count_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        Page::find()
            .filter(page::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new page.
    pub async fn create(&self, model: page::ActiveModel) -> AppResult<page::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a page.
    pub async fn update(&self, model: page::ActiveModel) -> AppResult<page::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a page.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        Page::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Check if a user has reached the maximum number of pages.
    pub async fn user_at_limit(&self, user_id: &str) -> AppResult<bool> {
        let count = self.count_by_user_id(user_id).await?;
        Ok(count as usize >= MAX_PAGES_PER_USER)
    }

    /// Check if a page name is already taken by a user.
    pub async fn name_exists(&self, user_id: &str, name: &str) -> AppResult<bool> {
        let page = self.find_by_user_and_name(user_id, name).await?;
        Ok(page.is_some())
    }

    /// Increment view count.
    pub async fn increment_view_count(&self, id: &str) -> AppResult<()> {
        let page = self.get_by_id(id).await?;
        let mut active: page::ActiveModel = page.into();
        active.view_count = Set(active.view_count.unwrap() + 1);
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ==================== Page Like Operations ====================

    /// Check if a user has liked a page.
    pub async fn has_liked(&self, page_id: &str, user_id: &str) -> AppResult<bool> {
        let count = PageLike::find()
            .filter(page_like::Column::PageId.eq(page_id))
            .filter(page_like::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    /// Like a page.
    pub async fn like(&self, model: page_like::ActiveModel) -> AppResult<page_like::Model> {
        let like = model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Update liked count
        let page_id = &like.page_id;
        let page = self.get_by_id(page_id).await?;
        let mut active: page::ActiveModel = page.into();
        active.liked_count = Set(active.liked_count.unwrap() + 1);
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(like)
    }

    /// Unlike a page.
    pub async fn unlike(&self, page_id: &str, user_id: &str) -> AppResult<()> {
        let deleted = PageLike::delete_many()
            .filter(page_like::Column::PageId.eq(page_id))
            .filter(page_like::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if deleted.rows_affected > 0 {
            // Update liked count
            let page = self.get_by_id(page_id).await?;
            let mut active: page::ActiveModel = page.into();
            let current_count = active.liked_count.clone().unwrap();
            active.liked_count = Set(if current_count > 0 { current_count - 1 } else { 0 });
            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }

    /// Find users who liked a page.
    pub async fn find_likes(&self, page_id: &str) -> AppResult<Vec<page_like::Model>> {
        PageLike::find()
            .filter(page_like::Column::PageId.eq(page_id))
            .order_by_desc(page_like::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find pages liked by a user.
    pub async fn find_liked_by_user(&self, user_id: &str) -> AppResult<Vec<page_like::Model>> {
        PageLike::find()
            .filter(page_like::Column::UserId.eq(user_id))
            .order_by_desc(page_like::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
