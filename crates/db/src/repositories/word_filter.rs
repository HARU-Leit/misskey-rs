//! Word filter repository.

use std::sync::Arc;

use crate::entities::{WordFilter, word_filter};
use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

/// Word filter repository for database operations.
#[derive(Clone)]
pub struct WordFilterRepository {
    db: Arc<DatabaseConnection>,
}

impl WordFilterRepository {
    /// Create a new word filter repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a filter by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<word_filter::Model>> {
        WordFilter::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a filter by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<word_filter::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Word filter {id} not found")))
    }

    /// Find all filters for a user.
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<word_filter::Model>> {
        WordFilter::find()
            .filter(word_filter::Column::UserId.eq(user_id))
            .order_by_desc(word_filter::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find active (non-expired) filters for a user.
    pub async fn find_active_by_user(&self, user_id: &str) -> AppResult<Vec<word_filter::Model>> {
        let now = Utc::now();

        WordFilter::find()
            .filter(word_filter::Column::UserId.eq(user_id))
            .filter(
                word_filter::Column::ExpiresAt
                    .is_null()
                    .or(word_filter::Column::ExpiresAt.gt(now)),
            )
            .order_by_asc(word_filter::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count filters for a user.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        WordFilter::find()
            .filter(word_filter::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new filter.
    pub async fn create(&self, model: word_filter::ActiveModel) -> AppResult<word_filter::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a filter.
    pub async fn update(&self, model: word_filter::ActiveModel) -> AppResult<word_filter::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a filter.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        WordFilter::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Increment the match count for a filter.
    pub async fn increment_match_count(&self, id: &str) -> AppResult<()> {
        let filter = self.get_by_id(id).await?;
        let mut active: word_filter::ActiveModel = filter.into();
        active.match_count = Set(active.match_count.unwrap() + 1);
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete expired filters.
    pub async fn delete_expired(&self) -> AppResult<u64> {
        let now = Utc::now();

        let result = WordFilter::delete_many()
            .filter(word_filter::Column::ExpiresAt.is_not_null())
            .filter(word_filter::Column::ExpiresAt.lt(now))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::word_filter::{FilterAction, FilterContext};
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_filter(id: &str, user_id: &str, phrase: &str) -> word_filter::Model {
        word_filter::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            phrase: phrase.to_string(),
            is_regex: false,
            case_sensitive: false,
            whole_word: true,
            action: FilterAction::Hide,
            context: FilterContext::All,
            expires_at: None,
            match_count: 0,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let filter = create_test_filter("filter1", "user1", "test");
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[filter.clone()]])
                .into_connection(),
        );

        let repo = WordFilterRepository::new(db);
        let result = repo.find_by_id("filter1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().phrase, "test");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<word_filter::Model>::new()])
                .into_connection(),
        );

        let repo = WordFilterRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let filters = vec![
            create_test_filter("filter1", "user1", "word1"),
            create_test_filter("filter2", "user1", "word2"),
        ];
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([filters.clone()])
                .into_connection(),
        );

        let repo = WordFilterRepository::new(db);
        let result = repo.find_by_user("user1", 10, 0).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_count_by_user() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[maplit::btreemap! {
                    "num_items" => sea_orm::Value::BigInt(Some(5))
                }]])
                .into_connection(),
        );

        let repo = WordFilterRepository::new(db);
        let result = repo.count_by_user("user1").await.unwrap();
        assert_eq!(result, 5);
    }
}
