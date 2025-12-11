//! Reaction repository.

use std::sync::Arc;

use crate::entities::{Reaction, reaction};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

/// Reaction repository for database operations.
#[derive(Clone)]
pub struct ReactionRepository {
    db: Arc<DatabaseConnection>,
}

impl ReactionRepository {
    /// Create a new reaction repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a reaction by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<reaction::Model>> {
        Reaction::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a reaction by user and note.
    pub async fn find_by_user_and_note(
        &self,
        user_id: &str,
        note_id: &str,
    ) -> AppResult<Option<reaction::Model>> {
        Reaction::find()
            .filter(reaction::Column::UserId.eq(user_id))
            .filter(reaction::Column::NoteId.eq(note_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a user has reacted to a note.
    pub async fn has_reacted(&self, user_id: &str, note_id: &str) -> AppResult<bool> {
        Ok(self
            .find_by_user_and_note(user_id, note_id)
            .await?
            .is_some())
    }

    /// Create a new reaction.
    pub async fn create(&self, model: reaction::ActiveModel) -> AppResult<reaction::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a reaction.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let reaction = self.find_by_id(id).await?;
        if let Some(r) = reaction {
            r.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Delete a reaction by user and note.
    pub async fn delete_by_user_and_note(&self, user_id: &str, note_id: &str) -> AppResult<()> {
        let reaction = self.find_by_user_and_note(user_id, note_id).await?;
        if let Some(r) = reaction {
            r.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get reactions for a note (paginated).
    pub async fn find_by_note(
        &self,
        note_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<reaction::Model>> {
        let mut query = Reaction::find()
            .filter(reaction::Column::NoteId.eq(note_id))
            .order_by_desc(reaction::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(reaction::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get reactions by a user (paginated).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<reaction::Model>> {
        let mut query = Reaction::find()
            .filter(reaction::Column::UserId.eq(user_id))
            .order_by_desc(reaction::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(reaction::Column::Id.lt(id));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count reactions on a note.
    pub async fn count_by_note(&self, note_id: &str) -> AppResult<u64> {
        Reaction::find()
            .filter(reaction::Column::NoteId.eq(note_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_reaction(
        id: &str,
        user_id: &str,
        note_id: &str,
        reaction_str: &str,
    ) -> reaction::Model {
        reaction::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            note_id: note_id.to_string(),
            reaction: reaction_str.to_string(),
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_id_found() {
        let reaction = create_test_reaction("r1", "user1", "note1", "👍");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[reaction.clone()]])
                .into_connection(),
        );

        let repo = ReactionRepository::new(db);
        let result = repo.find_by_id("r1").await.unwrap();

        assert!(result.is_some());
        let found = result.unwrap();
        assert_eq!(found.id, "r1");
        assert_eq!(found.reaction, "👍");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<reaction::Model>::new()])
                .into_connection(),
        );

        let repo = ReactionRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_user_and_note() {
        let reaction = create_test_reaction("r1", "user1", "note1", "❤️");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[reaction.clone()]])
                .into_connection(),
        );

        let repo = ReactionRepository::new(db);
        let result = repo.find_by_user_and_note("user1", "note1").await.unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_has_reacted_true() {
        let reaction = create_test_reaction("r1", "user1", "note1", "👍");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[reaction.clone()]])
                .into_connection(),
        );

        let repo = ReactionRepository::new(db);
        let result = repo.has_reacted("user1", "note1").await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_has_reacted_false() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<reaction::Model>::new()])
                .into_connection(),
        );

        let repo = ReactionRepository::new(db);
        let result = repo.has_reacted("user1", "note2").await.unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn test_find_by_note() {
        let r1 = create_test_reaction("r1", "user1", "note1", "👍");
        let r2 = create_test_reaction("r2", "user2", "note1", "❤️");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[r1, r2]])
                .into_connection(),
        );

        let repo = ReactionRepository::new(db);
        let result = repo.find_by_note("note1", 10, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let r1 = create_test_reaction("r1", "user1", "note1", "👍");
        let r2 = create_test_reaction("r2", "user1", "note2", "😀");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[r1, r2]])
                .into_connection(),
        );

        let repo = ReactionRepository::new(db);
        let result = repo.find_by_user("user1", 10, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
