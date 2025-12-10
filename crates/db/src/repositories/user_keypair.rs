//! User keypair repository.

use std::sync::Arc;

use crate::entities::{user_keypair, UserKeypair};
use misskey_common::{AppError, AppResult};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

/// User keypair repository for database operations.
#[derive(Clone)]
pub struct UserKeypairRepository {
    db: Arc<DatabaseConnection>,
}

impl UserKeypairRepository {
    /// Create a new user keypair repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a keypair by user ID.
    pub async fn find_by_user_id(&self, user_id: &str) -> AppResult<Option<user_keypair::Model>> {
        UserKeypair::find_by_id(user_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a keypair by user ID, returning an error if not found.
    pub async fn get_by_user_id(&self, user_id: &str) -> AppResult<user_keypair::Model> {
        self.find_by_user_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Keypair for user {user_id} not found")))
    }

    /// Find a keypair by key ID (for signature verification).
    pub async fn find_by_key_id(&self, key_id: &str) -> AppResult<Option<user_keypair::Model>> {
        UserKeypair::find()
            .filter(user_keypair::Column::KeyId.eq(key_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new keypair.
    pub async fn create(&self, model: user_keypair::ActiveModel) -> AppResult<user_keypair::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a keypair by user ID.
    pub async fn delete_by_user_id(&self, user_id: &str) -> AppResult<()> {
        UserKeypair::delete_by_id(user_id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_keypair(user_id: &str) -> user_keypair::Model {
        user_keypair::Model {
            user_id: user_id.to_string(),
            public_key: "-----BEGIN PUBLIC KEY-----\ntest\n-----END PUBLIC KEY-----".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----".to_string(),
            key_id: format!("https://example.com/users/{user_id}#main-key"),
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_user_id_found() {
        let keypair = create_test_keypair("user1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[keypair.clone()]])
                .into_connection(),
        );

        let repo = UserKeypairRepository::new(db);
        let result = repo.find_by_user_id("user1").await.unwrap();

        assert!(result.is_some());
        let found = result.unwrap();
        assert_eq!(found.user_id, "user1");
    }

    #[tokio::test]
    async fn test_find_by_user_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<user_keypair::Model>::new()])
                .into_connection(),
        );

        let repo = UserKeypairRepository::new(db);
        let result = repo.find_by_user_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_key_id() {
        let keypair = create_test_keypair("user1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[keypair.clone()]])
                .into_connection(),
        );

        let repo = UserKeypairRepository::new(db);
        let result = repo
            .find_by_key_id("https://example.com/users/user1#main-key")
            .await
            .unwrap();

        assert!(result.is_some());
    }
}
