//! Gallery repository.

use std::sync::Arc;

use crate::entities::{gallery_like, gallery_post, GalleryLike, GalleryPost};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

/// Maximum number of gallery posts per user.
pub const MAX_GALLERY_POSTS_PER_USER: usize = 300;

/// Gallery repository for database operations.
#[derive(Clone)]
pub struct GalleryRepository {
    db: Arc<DatabaseConnection>,
}

impl GalleryRepository {
    /// Create a new gallery repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ==================== Gallery Post Operations ====================

    /// Find a gallery post by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<gallery_post::Model>> {
        GalleryPost::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a gallery post by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<gallery_post::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Gallery post: {id}")))
    }

    /// Find all gallery posts for a user.
    pub async fn find_by_user_id(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<gallery_post::Model>> {
        GalleryPost::find()
            .filter(gallery_post::Column::UserId.eq(user_id))
            .order_by_desc(gallery_post::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find gallery posts with pagination.
    pub async fn find_with_pagination(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<gallery_post::Model>> {
        GalleryPost::find()
            .order_by_desc(gallery_post::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find featured gallery posts (most liked).
    pub async fn find_featured(&self, limit: u64) -> AppResult<Vec<gallery_post::Model>> {
        GalleryPost::find()
            .order_by_desc(gallery_post::Column::LikedCount)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find popular gallery posts (recent and liked).
    pub async fn find_popular(&self, limit: u64) -> AppResult<Vec<gallery_post::Model>> {
        GalleryPost::find()
            .filter(gallery_post::Column::LikedCount.gt(0))
            .order_by_desc(gallery_post::Column::LikedCount)
            .order_by_desc(gallery_post::Column::CreatedAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search gallery posts by tag.
    pub async fn find_by_tag(
        &self,
        tag: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<gallery_post::Model>> {
        // Use JSON contains for tag search
        GalleryPost::find()
            .filter(gallery_post::Column::Tags.contains(tag))
            .order_by_desc(gallery_post::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count gallery posts for a user.
    pub async fn count_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        GalleryPost::find()
            .filter(gallery_post::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new gallery post.
    pub async fn create(&self, model: gallery_post::ActiveModel) -> AppResult<gallery_post::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a gallery post.
    pub async fn update(&self, model: gallery_post::ActiveModel) -> AppResult<gallery_post::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a gallery post.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        GalleryPost::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Check if a user has reached the maximum number of gallery posts.
    pub async fn user_at_limit(&self, user_id: &str) -> AppResult<bool> {
        let count = self.count_by_user_id(user_id).await?;
        Ok(count as usize >= MAX_GALLERY_POSTS_PER_USER)
    }

    // ==================== Gallery Like Operations ====================

    /// Check if a user has liked a gallery post.
    pub async fn has_liked(&self, post_id: &str, user_id: &str) -> AppResult<bool> {
        let count = GalleryLike::find()
            .filter(gallery_like::Column::PostId.eq(post_id))
            .filter(gallery_like::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    /// Like a gallery post.
    pub async fn like(&self, model: gallery_like::ActiveModel) -> AppResult<gallery_like::Model> {
        let like = model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Update liked count
        let post_id = &like.post_id;
        let post = self.get_by_id(post_id).await?;
        let mut active: gallery_post::ActiveModel = post.into();
        active.liked_count = Set(active.liked_count.unwrap() + 1);
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(like)
    }

    /// Unlike a gallery post.
    pub async fn unlike(&self, post_id: &str, user_id: &str) -> AppResult<()> {
        let deleted = GalleryLike::delete_many()
            .filter(gallery_like::Column::PostId.eq(post_id))
            .filter(gallery_like::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if deleted.rows_affected > 0 {
            // Update liked count
            let post = self.get_by_id(post_id).await?;
            let mut active: gallery_post::ActiveModel = post.into();
            let current_count = active.liked_count.clone().unwrap();
            active.liked_count = Set(if current_count > 0 { current_count - 1 } else { 0 });
            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(())
    }

    /// Find users who liked a gallery post.
    pub async fn find_likes(
        &self,
        post_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<gallery_like::Model>> {
        GalleryLike::find()
            .filter(gallery_like::Column::PostId.eq(post_id))
            .order_by_desc(gallery_like::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find gallery posts liked by a user.
    pub async fn find_liked_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<gallery_like::Model>> {
        GalleryLike::find()
            .filter(gallery_like::Column::UserId.eq(user_id))
            .order_by_desc(gallery_like::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
