//! Following repository.

use std::sync::Arc;

use crate::entities::{following, Following};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

/// Following repository for database operations.
#[derive(Clone)]
pub struct FollowingRepository {
    db: Arc<DatabaseConnection>,
}

impl FollowingRepository {
    /// Create a new following repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a following relationship by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<following::Model>> {
        Following::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a following relationship by follower and followee.
    pub async fn find_by_pair(
        &self,
        follower_id: &str,
        followee_id: &str,
    ) -> AppResult<Option<following::Model>> {
        Following::find()
            .filter(following::Column::FollowerId.eq(follower_id))
            .filter(following::Column::FolloweeId.eq(followee_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a user is following another user.
    pub async fn is_following(&self, follower_id: &str, followee_id: &str) -> AppResult<bool> {
        Ok(self.find_by_pair(follower_id, followee_id).await?.is_some())
    }

    /// Create a new following relationship.
    pub async fn create(&self, model: following::ActiveModel) -> AppResult<following::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a following relationship.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let following = self.find_by_id(id).await?;
        if let Some(f) = following {
            f.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Delete a following relationship by pair.
    pub async fn delete_by_pair(&self, follower_id: &str, followee_id: &str) -> AppResult<()> {
        let following = self.find_by_pair(follower_id, followee_id).await?;
        if let Some(f) = following {
            f.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get users that a user is following (paginated).
    pub async fn find_following(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<following::Model>> {
        let mut query = Following::find()
            .filter(following::Column::FollowerId.eq(user_id))
            .order_by_desc(following::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(following::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get users that are following a user (paginated).
    pub async fn find_followers(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<following::Model>> {
        let mut query = Following::find()
            .filter(following::Column::FolloweeId.eq(user_id))
            .order_by_desc(following::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(following::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count followers of a user.
    pub async fn count_followers(&self, user_id: &str) -> AppResult<u64> {
        Following::find()
            .filter(following::Column::FolloweeId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count following of a user.
    pub async fn count_following(&self, user_id: &str) -> AppResult<u64> {
        Following::find()
            .filter(following::Column::FollowerId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get all users that a user is following (for export, with limit/offset).
    pub async fn get_following(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<following::Model>> {
        Following::find()
            .filter(following::Column::FollowerId.eq(user_id))
            .order_by_asc(following::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get all users that are following a user (for export, with limit/offset).
    pub async fn get_followers(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<following::Model>> {
        Following::find()
            .filter(following::Column::FolloweeId.eq(user_id))
            .order_by_asc(following::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_following(id: &str, follower_id: &str, followee_id: &str) -> following::Model {
        following::Model {
            id: id.to_string(),
            follower_id: follower_id.to_string(),
            followee_id: followee_id.to_string(),
            follower_host: None,
            followee_host: None,
            followee_inbox: None,
            followee_shared_inbox: None,
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_id_found() {
        let following = create_test_following("f1", "user1", "user2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[following.clone()]])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.find_by_id("f1").await.unwrap();

        assert!(result.is_some());
        let found = result.unwrap();
        assert_eq!(found.id, "f1");
        assert_eq!(found.follower_id, "user1");
        assert_eq!(found.followee_id, "user2");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<following::Model>::new()])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_pair_found() {
        let following = create_test_following("f1", "user1", "user2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[following.clone()]])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.find_by_pair("user1", "user2").await.unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_find_by_pair_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<following::Model>::new()])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.find_by_pair("user1", "user3").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_is_following_true() {
        let following = create_test_following("f1", "user1", "user2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[following.clone()]])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.is_following("user1", "user2").await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_is_following_false() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<following::Model>::new()])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.is_following("user1", "user3").await.unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn test_find_following() {
        let f1 = create_test_following("f1", "user1", "user2");
        let f2 = create_test_following("f2", "user1", "user3");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[f1, f2]])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.find_following("user1", 10, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_find_followers() {
        let f1 = create_test_following("f1", "user2", "user1");
        let f2 = create_test_following("f2", "user3", "user1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[f1, f2]])
                .into_connection(),
        );

        let repo = FollowingRepository::new(db);
        let result = repo.find_followers("user1", 10, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
