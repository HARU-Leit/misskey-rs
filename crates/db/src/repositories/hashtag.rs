//! Hashtag repository.

use std::sync::Arc;

use crate::entities::{Hashtag, hashtag};
use chrono::Utc;
use misskey_common::{AppError, AppResult, IdGenerator};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

/// Hashtag repository for database operations.
#[derive(Clone)]
pub struct HashtagRepository {
    db: Arc<DatabaseConnection>,
    id_gen: IdGenerator,
}

impl HashtagRepository {
    /// Create a new hashtag repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            id_gen: IdGenerator::new(),
        }
    }

    /// Find a hashtag by name.
    pub async fn find_by_name(&self, name: &str) -> AppResult<Option<hashtag::Model>> {
        let name_lower = name.to_lowercase();
        Hashtag::find()
            .filter(hashtag::Column::Name.eq(&name_lower))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get or create a hashtag.
    pub async fn get_or_create(&self, name: &str) -> AppResult<hashtag::Model> {
        let name_lower = name.to_lowercase();

        // Try to find existing
        if let Some(tag) = self.find_by_name(&name_lower).await? {
            return Ok(tag);
        }

        // Create new
        let model = hashtag::ActiveModel {
            id: Set(self.id_gen.generate()),
            name: Set(name_lower),
            notes_count: Set(0),
            users_count: Set(0),
            local_notes_count: Set(0),
            remote_notes_count: Set(0),
            is_trending: Set(false),
            last_used_at: Set(Some(Utc::now().into())),
            created_at: Set(Utc::now().into()),
        };

        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Increment note count for a hashtag.
    pub async fn increment_notes_count(
        &self,
        name: &str,
        is_local: bool,
    ) -> AppResult<hashtag::Model> {
        let tag = self.get_or_create(name).await?;

        let mut active: hashtag::ActiveModel = tag.into();
        let count = active.notes_count.clone().unwrap();
        active.notes_count = Set(count + 1);

        if is_local {
            let local_count = active.local_notes_count.clone().unwrap();
            active.local_notes_count = Set(local_count + 1);
        } else {
            let remote_count = active.remote_notes_count.clone().unwrap();
            active.remote_notes_count = Set(remote_count + 1);
        }

        active.last_used_at = Set(Some(Utc::now().into()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Decrement note count for a hashtag.
    pub async fn decrement_notes_count(&self, name: &str, is_local: bool) -> AppResult<()> {
        if let Some(tag) = self.find_by_name(name).await? {
            let mut active: hashtag::ActiveModel = tag.into();
            let count = active.notes_count.clone().unwrap();
            active.notes_count = Set(if count > 0 { count - 1 } else { 0 });

            if is_local {
                let local_count = active.local_notes_count.clone().unwrap();
                active.local_notes_count = Set(if local_count > 0 { local_count - 1 } else { 0 });
            } else {
                let remote_count = active.remote_notes_count.clone().unwrap();
                active.remote_notes_count = Set(if remote_count > 0 {
                    remote_count - 1
                } else {
                    0
                });
            }

            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get trending hashtags.
    pub async fn find_trending(&self, limit: u64) -> AppResult<Vec<hashtag::Model>> {
        Hashtag::find()
            .filter(hashtag::Column::IsTrending.eq(true))
            .order_by_desc(hashtag::Column::NotesCount)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get popular hashtags (by note count).
    pub async fn find_popular(&self, limit: u64) -> AppResult<Vec<hashtag::Model>> {
        Hashtag::find()
            .order_by_desc(hashtag::Column::NotesCount)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search hashtags by prefix.
    pub async fn search(&self, query: &str, limit: u64) -> AppResult<Vec<hashtag::Model>> {
        let query_lower = query.to_lowercase();
        let pattern = format!("{query_lower}%");

        Hashtag::find()
            .filter(hashtag::Column::Name.like(&pattern))
            .order_by_desc(hashtag::Column::NotesCount)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Set trending status for a hashtag.
    pub async fn set_trending(&self, name: &str, is_trending: bool) -> AppResult<()> {
        if let Some(tag) = self.find_by_name(name).await? {
            let mut active: hashtag::ActiveModel = tag.into();
            active.is_trending = Set(is_trending);
            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_hashtag(id: &str, name: &str, notes_count: i32) -> hashtag::Model {
        hashtag::Model {
            id: id.to_string(),
            name: name.to_string(),
            notes_count,
            users_count: 0,
            local_notes_count: notes_count,
            remote_notes_count: 0,
            is_trending: false,
            last_used_at: Some(Utc::now().into()),
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_name() {
        let tag = create_test_hashtag("h1", "rust", 10);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[tag.clone()]])
                .into_connection(),
        );

        let repo = HashtagRepository::new(db);
        let result = repo.find_by_name("rust").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "rust");
    }

    #[tokio::test]
    async fn test_find_popular() {
        let tag1 = create_test_hashtag("h1", "rust", 100);
        let tag2 = create_test_hashtag("h2", "programming", 50);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[tag1, tag2]])
                .into_connection(),
        );

        let repo = HashtagRepository::new(db);
        let result = repo.find_popular(10).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "rust");
    }

    #[tokio::test]
    async fn test_search() {
        let tag1 = create_test_hashtag("h1", "rustlang", 50);
        let tag2 = create_test_hashtag("h2", "rustacean", 30);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[tag1, tag2]])
                .into_connection(),
        );

        let repo = HashtagRepository::new(db);
        let result = repo.search("rust", 10).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
