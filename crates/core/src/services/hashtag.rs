//! Hashtag service.

use misskey_common::{AppError, AppResult};
use misskey_db::{entities::hashtag, repositories::HashtagRepository};

/// Hashtag service for business logic.
#[derive(Clone)]
pub struct HashtagService {
    hashtag_repo: HashtagRepository,
}

impl HashtagService {
    /// Create a new hashtag service.
    #[must_use]
    pub const fn new(hashtag_repo: HashtagRepository) -> Self {
        Self { hashtag_repo }
    }

    /// Get a hashtag by name.
    pub async fn get(&self, name: &str) -> AppResult<hashtag::Model> {
        self.hashtag_repo
            .find_by_name(name)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Hashtag not found: {name}")))
    }

    /// Get trending hashtags.
    pub async fn get_trending(&self, limit: u64) -> AppResult<Vec<hashtag::Model>> {
        self.hashtag_repo.find_trending(limit).await
    }

    /// Get popular hashtags.
    pub async fn get_popular(&self, limit: u64) -> AppResult<Vec<hashtag::Model>> {
        self.hashtag_repo.find_popular(limit).await
    }

    /// Search hashtags by prefix.
    pub async fn search(&self, query: &str, limit: u64) -> AppResult<Vec<hashtag::Model>> {
        self.hashtag_repo.search(query, limit).await
    }

    /// Update hashtag counts when a note is created.
    pub async fn on_note_created(&self, tags: &[String], is_local: bool) -> AppResult<()> {
        for tag in tags {
            self.hashtag_repo
                .increment_notes_count(tag, is_local)
                .await?;
        }
        Ok(())
    }

    /// Update hashtag counts when a note is deleted.
    pub async fn on_note_deleted(&self, tags: &[String], is_local: bool) -> AppResult<()> {
        for tag in tags {
            self.hashtag_repo
                .decrement_notes_count(tag, is_local)
                .await?;
        }
        Ok(())
    }

    /// Mark a hashtag as trending.
    pub async fn set_trending(&self, name: &str, is_trending: bool) -> AppResult<()> {
        self.hashtag_repo.set_trending(name, is_trending).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

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
    async fn test_get_trending() {
        let tag1 = create_test_hashtag("h1", "rust", 100);
        let tag2 = create_test_hashtag("h2", "programming", 50);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[tag1, tag2]])
                .into_connection(),
        );

        let repo = HashtagRepository::new(db);
        let service = HashtagService::new(repo);

        let result = service.get_trending(10).await.unwrap();
        assert_eq!(result.len(), 2);
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
        let service = HashtagService::new(repo);

        let result = service.search("rust", 10).await.unwrap();
        assert_eq!(result.len(), 2);
    }
}
