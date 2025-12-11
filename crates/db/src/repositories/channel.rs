//! Channel repository.

use std::sync::Arc;

use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entities::{Channel, ChannelFollowing, channel, channel_following};

/// Repository for channel operations.
#[derive(Clone)]
pub struct ChannelRepository {
    db: Arc<DatabaseConnection>,
}

impl ChannelRepository {
    /// Create a new channel repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ==================== Channel Operations ====================

    /// Find channel by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<channel::Model>> {
        Channel::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get channel by ID, returning error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<channel::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Channel not found: {id}")))
    }

    /// Find channel by `ActivityPub` URI.
    pub async fn find_by_uri(&self, uri: &str) -> AppResult<Option<channel::Model>> {
        Channel::find()
            .filter(channel::Column::Uri.eq(uri))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get channel by URI, returning error if not found.
    pub async fn get_by_uri(&self, uri: &str) -> AppResult<channel::Model> {
        self.find_by_uri(uri)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Channel not found with URI: {uri}")))
    }

    /// Find local channels (host is null).
    pub async fn find_local(&self, limit: u64, offset: u64) -> AppResult<Vec<channel::Model>> {
        Channel::find()
            .filter(channel::Column::Host.is_null())
            .filter(channel::Column::IsArchived.eq(false))
            .order_by(channel::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find remote channels from a specific host.
    pub async fn find_by_host(
        &self,
        host: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<channel::Model>> {
        Channel::find()
            .filter(channel::Column::Host.eq(host))
            .filter(channel::Column::IsArchived.eq(false))
            .order_by(channel::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find channels by user ID (owned channels).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<channel::Model>> {
        Channel::find()
            .filter(channel::Column::UserId.eq(user_id))
            .filter(channel::Column::IsArchived.eq(false))
            .order_by(channel::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find featured (popular) channels.
    pub async fn find_featured(&self, limit: u64, offset: u64) -> AppResult<Vec<channel::Model>> {
        Channel::find()
            .filter(channel::Column::IsArchived.eq(false))
            .order_by(channel::Column::UsersCount, Order::Desc)
            .order_by(channel::Column::NotesCount, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search channels by name.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<channel::Model>> {
        Channel::find()
            .filter(channel::Column::Name.contains(query))
            .filter(channel::Column::IsArchived.eq(false))
            .filter(channel::Column::IsSearchable.eq(true))
            .order_by(channel::Column::UsersCount, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count channels by user ID.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        Channel::find()
            .filter(channel::Column::UserId.eq(user_id))
            .filter(channel::Column::IsArchived.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new channel.
    pub async fn create(&self, model: channel::ActiveModel) -> AppResult<channel::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a channel.
    pub async fn update(&self, model: channel::ActiveModel) -> AppResult<channel::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Archive a channel (soft delete).
    pub async fn archive(&self, id: &str) -> AppResult<channel::Model> {
        let channel = self.get_by_id(id).await?;
        let mut active: channel::ActiveModel = channel.into();
        active.is_archived = Set(true);
        active.updated_at = Set(Some(Utc::now().into()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a channel permanently.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        Channel::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Increment notes count atomically.
    pub async fn increment_notes_count(&self, id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Channel::update_many()
            .col_expr(
                channel::Column::NotesCount,
                Expr::col(channel::Column::NotesCount).add(1),
            )
            .col_expr(channel::Column::LastNotedAt, Expr::value(Utc::now()))
            .filter(channel::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Decrement notes count atomically.
    pub async fn decrement_notes_count(&self, id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Channel::update_many()
            .col_expr(
                channel::Column::NotesCount,
                Expr::cust("GREATEST(notes_count - 1, 0)"),
            )
            .filter(channel::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ==================== Channel Following Operations ====================

    /// Check if user is following a channel.
    pub async fn is_following(&self, user_id: &str, channel_id: &str) -> AppResult<bool> {
        let count = ChannelFollowing::find()
            .filter(channel_following::Column::UserId.eq(user_id))
            .filter(channel_following::Column::ChannelId.eq(channel_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
    }

    /// Get following record.
    pub async fn get_following(
        &self,
        user_id: &str,
        channel_id: &str,
    ) -> AppResult<Option<channel_following::Model>> {
        ChannelFollowing::find()
            .filter(channel_following::Column::UserId.eq(user_id))
            .filter(channel_following::Column::ChannelId.eq(channel_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Follow a channel.
    pub async fn follow(
        &self,
        id: String,
        user_id: String,
        channel_id: String,
    ) -> AppResult<channel_following::Model> {
        let model = channel_following::ActiveModel {
            id: Set(id),
            user_id: Set(user_id),
            channel_id: Set(channel_id.clone()),
            is_read: Set(false),
            created_at: Set(Utc::now().into()),
        };

        let following = model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Increment users count
        self.increment_users_count(&channel_id).await?;

        Ok(following)
    }

    /// Unfollow a channel.
    pub async fn unfollow(&self, user_id: &str, channel_id: &str) -> AppResult<()> {
        let deleted = ChannelFollowing::delete_many()
            .filter(channel_following::Column::UserId.eq(user_id))
            .filter(channel_following::Column::ChannelId.eq(channel_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if deleted.rows_affected > 0 {
            self.decrement_users_count(channel_id).await?;
        }

        Ok(())
    }

    /// Find channels followed by a user.
    pub async fn find_followed_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<channel::Model>> {
        // Get followed channel IDs
        let following = ChannelFollowing::find()
            .filter(channel_following::Column::UserId.eq(user_id))
            .order_by(channel_following::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let channel_ids: Vec<String> = following.iter().map(|f| f.channel_id.clone()).collect();

        if channel_ids.is_empty() {
            return Ok(vec![]);
        }

        Channel::find()
            .filter(channel::Column::Id.is_in(channel_ids))
            .filter(channel::Column::IsArchived.eq(false))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count channels followed by a user.
    pub async fn count_followed_by_user(&self, user_id: &str) -> AppResult<u64> {
        ChannelFollowing::find()
            .filter(channel_following::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Increment users count atomically.
    async fn increment_users_count(&self, channel_id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Channel::update_many()
            .col_expr(
                channel::Column::UsersCount,
                Expr::col(channel::Column::UsersCount).add(1),
            )
            .filter(channel::Column::Id.eq(channel_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Decrement users count atomically.
    async fn decrement_users_count(&self, channel_id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Channel::update_many()
            .col_expr(
                channel::Column::UsersCount,
                Expr::cust("GREATEST(users_count - 1, 0)"),
            )
            .filter(channel::Column::Id.eq(channel_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    fn create_test_channel(id: &str, user_id: &str, name: &str) -> channel::Model {
        channel::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            description: None,
            banner_id: None,
            color: None,
            is_archived: false,
            is_searchable: true,
            allow_anyone_to_post: true,
            notes_count: 0,
            users_count: 0,
            last_noted_at: None,
            created_at: Utc::now().into(),
            updated_at: None,
            // Federation fields
            uri: None,
            public_key_pem: None,
            private_key_pem: None,
            inbox: None,
            shared_inbox: None,
            host: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let channel = create_test_channel("ch1", "user1", "My Channel");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[channel.clone()]])
                .into_connection(),
        );

        let repo = ChannelRepository::new(db);
        let result = repo.find_by_id("ch1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Channel");
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let ch1 = create_test_channel("ch1", "user1", "Channel 1");
        let ch2 = create_test_channel("ch2", "user1", "Channel 2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[ch1, ch2]])
                .into_connection(),
        );

        let repo = ChannelRepository::new(db);
        let result = repo.find_by_user("user1", 10, 0).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_exec_results([MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 1,
                }])
                .into_connection(),
        );

        let repo = ChannelRepository::new(db);
        let result = repo.delete("ch1").await;

        assert!(result.is_ok());
    }
}
