//! Note favorite (bookmark) repository.

use std::sync::Arc;

use crate::entities::{note_favorite, NoteFavorite};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// Note favorite repository for database operations.
#[derive(Clone)]
pub struct NoteFavoriteRepository {
    db: Arc<DatabaseConnection>,
}

impl NoteFavoriteRepository {
    /// Create a new note favorite repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a favorite by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<note_favorite::Model>> {
        NoteFavorite::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a favorite by user and note.
    pub async fn find_by_user_and_note(
        &self,
        user_id: &str,
        note_id: &str,
    ) -> AppResult<Option<note_favorite::Model>> {
        NoteFavorite::find()
            .filter(note_favorite::Column::UserId.eq(user_id))
            .filter(note_favorite::Column::NoteId.eq(note_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a note is favorited by user.
    pub async fn is_favorited(&self, user_id: &str, note_id: &str) -> AppResult<bool> {
        Ok(self.find_by_user_and_note(user_id, note_id).await?.is_some())
    }

    /// Create a new favorite.
    pub async fn create(
        &self,
        model: note_favorite::ActiveModel,
    ) -> AppResult<note_favorite::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a favorite by ID.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        NoteFavorite::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete a favorite by user and note.
    pub async fn delete_by_user_and_note(&self, user_id: &str, note_id: &str) -> AppResult<()> {
        NoteFavorite::delete_many()
            .filter(note_favorite::Column::UserId.eq(user_id))
            .filter(note_favorite::Column::NoteId.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get favorites by user (paginated, newest first).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note_favorite::Model>> {
        let mut query = NoteFavorite::find()
            .filter(note_favorite::Column::UserId.eq(user_id))
            .order_by_desc(note_favorite::Column::Id)
            .limit(limit);

        if let Some(until) = until_id {
            query = query.filter(note_favorite::Column::Id.lt(until));
        }

        query
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count favorites for a user.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        NoteFavorite::find()
            .filter(note_favorite::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_favorite(id: &str, user_id: &str, note_id: &str) -> note_favorite::Model {
        note_favorite::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            note_id: note_id.to_string(),
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let fav = create_test_favorite("fav1", "user1", "note1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[fav.clone()]])
                .into_connection(),
        );

        let repo = NoteFavoriteRepository::new(db);
        let result = repo.find_by_id("fav1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "fav1");
    }

    #[tokio::test]
    async fn test_is_favorited() {
        let fav = create_test_favorite("fav1", "user1", "note1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[fav.clone()]])
                .into_connection(),
        );

        let repo = NoteFavoriteRepository::new(db);
        let result = repo.is_favorited("user1", "note1").await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_is_not_favorited() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<note_favorite::Model>::new()])
                .into_connection(),
        );

        let repo = NoteFavoriteRepository::new(db);
        let result = repo.is_favorited("user1", "note1").await.unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let fav1 = create_test_favorite("fav1", "user1", "note1");
        let fav2 = create_test_favorite("fav2", "user1", "note2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[fav1, fav2]])
                .into_connection(),
        );

        let repo = NoteFavoriteRepository::new(db);
        let result = repo.find_by_user("user1", 10, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
