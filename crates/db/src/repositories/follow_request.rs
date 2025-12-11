//! Follow request repository.

use std::sync::Arc;

use crate::entities::{FollowRequest, follow_request};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

/// Follow request repository for database operations.
#[derive(Clone)]
pub struct FollowRequestRepository {
    db: Arc<DatabaseConnection>,
}

impl FollowRequestRepository {
    /// Create a new follow request repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a follow request by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<follow_request::Model>> {
        FollowRequest::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a follow request by follower and followee.
    pub async fn find_by_pair(
        &self,
        follower_id: &str,
        followee_id: &str,
    ) -> AppResult<Option<follow_request::Model>> {
        FollowRequest::find()
            .filter(follow_request::Column::FollowerId.eq(follower_id))
            .filter(follow_request::Column::FolloweeId.eq(followee_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a follow request exists.
    pub async fn exists(&self, follower_id: &str, followee_id: &str) -> AppResult<bool> {
        Ok(self.find_by_pair(follower_id, followee_id).await?.is_some())
    }

    /// Create a new follow request.
    pub async fn create(
        &self,
        model: follow_request::ActiveModel,
    ) -> AppResult<follow_request::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a follow request.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let request = self.find_by_id(id).await?;
        if let Some(r) = request {
            r.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Delete a follow request by pair.
    pub async fn delete_by_pair(&self, follower_id: &str, followee_id: &str) -> AppResult<()> {
        let request = self.find_by_pair(follower_id, followee_id).await?;
        if let Some(r) = request {
            r.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get pending follow requests for a user (paginated).
    pub async fn find_received(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<follow_request::Model>> {
        let mut query = FollowRequest::find()
            .filter(follow_request::Column::FolloweeId.eq(user_id))
            .order_by_desc(follow_request::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(follow_request::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get sent follow requests by a user (paginated).
    pub async fn find_sent(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<follow_request::Model>> {
        let mut query = FollowRequest::find()
            .filter(follow_request::Column::FollowerId.eq(user_id))
            .order_by_desc(follow_request::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(follow_request::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count pending follow requests for a user.
    pub async fn count_received(&self, user_id: &str) -> AppResult<u64> {
        FollowRequest::find()
            .filter(follow_request::Column::FolloweeId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a follow request by followee (used for Accept/Reject processing).
    pub async fn find_by_followee(
        &self,
        followee_id: &str,
    ) -> AppResult<Option<follow_request::Model>> {
        FollowRequest::find()
            .filter(follow_request::Column::FolloweeId.eq(followee_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
