//! User repository.

use std::sync::Arc;

use crate::entities::{User, user};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, sea_query::Expr,
};

/// User repository for database operations.
#[derive(Clone)]
pub struct UserRepository {
    db: Arc<DatabaseConnection>,
}

impl UserRepository {
    /// Create a new user repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a user by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<user::Model>> {
        User::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a user by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<user::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::UserNotFound(id.to_string()))
    }

    /// Find users by IDs.
    pub async fn find_by_ids(&self, ids: &[String]) -> AppResult<Vec<user::Model>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        User::find()
            .filter(user::Column::Id.is_in(ids.to_vec()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a user by username and host.
    pub async fn find_by_username_and_host(
        &self,
        username: &str,
        host: Option<&str>,
    ) -> AppResult<Option<user::Model>> {
        let mut query =
            User::find().filter(user::Column::UsernameLower.eq(username.to_lowercase()));

        query = match host {
            Some(h) => query.filter(user::Column::Host.eq(h)),
            None => query.filter(user::Column::Host.is_null()),
        };

        query
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a user by token.
    pub async fn find_by_token(&self, token: &str) -> AppResult<Option<user::Model>> {
        User::find()
            .filter(user::Column::Token.eq(token))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a user by `ActivityPub` URI.
    pub async fn find_by_uri(&self, uri: &str) -> AppResult<Option<user::Model>> {
        User::find()
            .filter(user::Column::Uri.eq(uri))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new user.
    pub async fn create(&self, model: user::ActiveModel) -> AppResult<user::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a user.
    pub async fn update(&self, model: user::ActiveModel) -> AppResult<user::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get local users (paginated).
    pub async fn find_local_users(&self, limit: u64, offset: u64) -> AppResult<Vec<user::Model>> {
        User::find()
            .filter(user::Column::Host.is_null())
            .filter(user::Column::IsSuspended.eq(false))
            .order_by_desc(user::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Increment notes count atomically (single UPDATE query, no fetch).
    pub async fn increment_notes_count(&self, user_id: &str) -> AppResult<()> {
        User::update_many()
            .col_expr(
                user::Column::NotesCount,
                Expr::col(user::Column::NotesCount).add(1),
            )
            .filter(user::Column::Id.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Decrement notes count atomically (single UPDATE query, no fetch).
    pub async fn decrement_notes_count(&self, user_id: &str) -> AppResult<()> {
        User::update_many()
            .col_expr(
                user::Column::NotesCount,
                Expr::cust("GREATEST(notes_count - 1, 0)"),
            )
            .filter(user::Column::Id.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Increment followers count atomically (single UPDATE query, no fetch).
    pub async fn increment_followers_count(&self, user_id: &str) -> AppResult<()> {
        User::update_many()
            .col_expr(
                user::Column::FollowersCount,
                Expr::col(user::Column::FollowersCount).add(1),
            )
            .filter(user::Column::Id.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Decrement followers count atomically (single UPDATE query, no fetch).
    pub async fn decrement_followers_count(&self, user_id: &str) -> AppResult<()> {
        User::update_many()
            .col_expr(
                user::Column::FollowersCount,
                Expr::cust("GREATEST(followers_count - 1, 0)"),
            )
            .filter(user::Column::Id.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Increment following count atomically (single UPDATE query, no fetch).
    pub async fn increment_following_count(&self, user_id: &str) -> AppResult<()> {
        User::update_many()
            .col_expr(
                user::Column::FollowingCount,
                Expr::col(user::Column::FollowingCount).add(1),
            )
            .filter(user::Column::Id.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Decrement following count atomically (single UPDATE query, no fetch).
    pub async fn decrement_following_count(&self, user_id: &str) -> AppResult<()> {
        User::update_many()
            .col_expr(
                user::Column::FollowingCount,
                Expr::cust("GREATEST(following_count - 1, 0)"),
            )
            .filter(user::Column::Id.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Mark a user as deleted/suspended (for remote actor deletion).
    pub async fn mark_as_deleted(&self, user_id: &str) -> AppResult<()> {
        let user = self.get_by_id(user_id).await?;
        let mut active: user::ActiveModel = user.into();
        active.is_suspended = Set(true);
        active.updated_at = Set(Some(chrono::Utc::now().into()));
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Count total local users.
    pub async fn count_local_users(&self) -> AppResult<u64> {
        User::find()
            .filter(user::Column::Host.is_null())
            .filter(user::Column::IsSuspended.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count local users active in the last month.
    pub async fn count_active_local_users_month(&self) -> AppResult<u64> {
        use chrono::{Duration, Utc};
        let one_month_ago = Utc::now() - Duration::days(30);

        User::find()
            .filter(user::Column::Host.is_null())
            .filter(user::Column::IsSuspended.eq(false))
            .filter(user::Column::UpdatedAt.gte(one_month_ago))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count local users active in the last half year.
    pub async fn count_active_local_users_halfyear(&self) -> AppResult<u64> {
        use chrono::{Duration, Utc};
        let six_months_ago = Utc::now() - Duration::days(180);

        User::find()
            .filter(user::Column::Host.is_null())
            .filter(user::Column::IsSuspended.eq(false))
            .filter(user::Column::UpdatedAt.gte(six_months_ago))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search users by username or display name.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
        local_only: bool,
    ) -> AppResult<Vec<user::Model>> {
        use sea_orm::Condition;

        let search_pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let query_lower = query.to_lowercase();

        let mut condition = Condition::all()
            .add(user::Column::IsSuspended.eq(false))
            .add(
                Condition::any()
                    .add(user::Column::UsernameLower.like(format!("%{query_lower}%")))
                    .add(user::Column::Name.like(&search_pattern)),
            );

        if local_only {
            condition = condition.add(user::Column::Host.is_null());
        }

        User::find()
            .filter(condition)
            .order_by_desc(user::Column::FollowersCount)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get IDs of all bot users.
    ///
    /// Used for filtering out bot notes from timelines when `hide_bots` is enabled.
    pub async fn find_bot_user_ids(&self) -> AppResult<Vec<String>> {
        User::find()
            .filter(user::Column::IsBot.eq(true))
            .select_only()
            .column(user::Column::Id)
            .into_tuple::<String>()
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
    use std::sync::Arc;

    fn create_test_user(id: &str, username: &str) -> user::Model {
        user::Model {
            id: id.to_string(),
            username: username.to_string(),
            username_lower: username.to_lowercase(),
            host: None,
            name: Some("Test User".to_string()),
            description: None,
            avatar_url: None,
            banner_url: None,
            is_bot: false,
            is_cat: false,
            is_locked: false,
            is_suspended: false,
            is_silenced: false,
            is_admin: false,
            is_moderator: false,
            followers_count: 0,
            following_count: 0,
            notes_count: 0,
            inbox: None,
            shared_inbox: None,
            featured: None,
            uri: None,
            last_fetched_at: None,
            token: Some("test_token".to_string()),
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_id_found() {
        let user = create_test_user("user1", "testuser");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[user.clone()]])
                .into_connection(),
        );

        let repo = UserRepository::new(db);
        let result = repo.find_by_id("user1").await.unwrap();

        assert!(result.is_some());
        let found_user = result.unwrap();
        assert_eq!(found_user.id, "user1");
        assert_eq!(found_user.username, "testuser");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<user::Model>::new()])
                .into_connection(),
        );

        let repo = UserRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_not_found_returns_error() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<user::Model>::new()])
                .into_connection(),
        );

        let repo = UserRepository::new(db);
        let result = repo.get_by_id("nonexistent").await;

        assert!(result.is_err());
        match result {
            Err(AppError::UserNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected UserNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_find_by_username_and_host_local() {
        let user = create_test_user("user1", "testuser");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[user.clone()]])
                .into_connection(),
        );

        let repo = UserRepository::new(db);
        let result = repo
            .find_by_username_and_host("testuser", None)
            .await
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().username, "testuser");
    }

    #[tokio::test]
    async fn test_find_by_token() {
        let user = create_test_user("user1", "testuser");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[user.clone()]])
                .into_connection(),
        );

        let repo = UserRepository::new(db);
        let result = repo.find_by_token("test_token").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().token, Some("test_token".to_string()));
    }

    #[tokio::test]
    async fn test_create_user() {
        let user = create_test_user("user1", "newuser");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[user.clone()]])
                .append_exec_results([MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 1,
                }])
                .into_connection(),
        );

        let repo = UserRepository::new(db);

        let active = user::ActiveModel {
            id: Set("user1".to_string()),
            username: Set("newuser".to_string()),
            username_lower: Set("newuser".to_string()),
            ..Default::default()
        };

        let result = repo.create(active).await.unwrap();
        assert_eq!(result.username, "newuser");
    }

    #[tokio::test]
    async fn test_find_local_users() {
        let user1 = create_test_user("user1", "user1");
        let user2 = create_test_user("user2", "user2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[user1, user2]])
                .into_connection(),
        );

        let repo = UserRepository::new(db);
        let result = repo.find_local_users(10, 0).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
