//! Clip repository.

use std::sync::Arc;

use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entities::{clip, clip_note, Clip, ClipNote};

/// Repository for clip operations.
#[derive(Clone)]
pub struct ClipRepository {
    db: Arc<DatabaseConnection>,
}

impl ClipRepository {
    /// Create a new clip repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ==================== Clip Operations ====================

    /// Find clip by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<clip::Model>> {
        Clip::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find clips by user ID.
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip::Model>> {
        Clip::find()
            .filter(clip::Column::UserId.eq(user_id))
            .order_by(clip::Column::DisplayOrder, Order::Asc)
            .order_by(clip::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find public clips by user ID (for viewing other users' clips).
    pub async fn find_public_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip::Model>> {
        Clip::find()
            .filter(clip::Column::UserId.eq(user_id))
            .filter(clip::Column::IsPublic.eq(true))
            .order_by(clip::Column::DisplayOrder, Order::Asc)
            .order_by(clip::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count clips by user ID.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        Clip::find()
            .filter(clip::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new clip.
    pub async fn create(
        &self,
        id: String,
        user_id: String,
        name: String,
        description: Option<String>,
        is_public: bool,
    ) -> AppResult<clip::Model> {
        let active_model = clip::ActiveModel {
            id: Set(id),
            user_id: Set(user_id),
            name: Set(name),
            description: Set(description),
            is_public: Set(is_public),
            notes_count: Set(0),
            display_order: Set(0),
            created_at: Set(Utc::now().into()),
            updated_at: Set(None),
        };

        active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a clip.
    pub async fn update(
        &self,
        id: &str,
        name: Option<String>,
        description: Option<Option<String>>,
        is_public: Option<bool>,
    ) -> AppResult<clip::Model> {
        let clip = Clip::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Clip not found: {id}")))?;

        let mut active: clip::ActiveModel = clip.into();

        if let Some(name) = name {
            active.name = Set(name);
        }
        if let Some(description) = description {
            active.description = Set(description);
        }
        if let Some(is_public) = is_public {
            active.is_public = Set(is_public);
        }

        active.updated_at = Set(Some(Utc::now().into()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update clip display order.
    pub async fn update_display_order(&self, id: &str, display_order: i32) -> AppResult<()> {
        let clip = Clip::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Clip not found: {id}")))?;

        let mut active: clip::ActiveModel = clip.into();
        active.display_order = Set(display_order);
        active.updated_at = Set(Some(Utc::now().into()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete a clip (and all its notes due to CASCADE).
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        Clip::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ==================== Clip Note Operations ====================

    /// Find clip note by ID.
    pub async fn find_clip_note_by_id(&self, id: &str) -> AppResult<Option<clip_note::Model>> {
        ClipNote::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find clip note by clip ID and note ID.
    pub async fn find_clip_note(
        &self,
        clip_id: &str,
        note_id: &str,
    ) -> AppResult<Option<clip_note::Model>> {
        ClipNote::find()
            .filter(clip_note::Column::ClipId.eq(clip_id))
            .filter(clip_note::Column::NoteId.eq(note_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a note is in a clip.
    pub async fn is_note_in_clip(&self, clip_id: &str, note_id: &str) -> AppResult<bool> {
        Ok(self.find_clip_note(clip_id, note_id).await?.is_some())
    }

    /// Find notes in a clip (paginated).
    pub async fn find_notes_in_clip(
        &self,
        clip_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip_note::Model>> {
        ClipNote::find()
            .filter(clip_note::Column::ClipId.eq(clip_id))
            .order_by(clip_note::Column::DisplayOrder, Order::Asc)
            .order_by(clip_note::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find which clips contain a note.
    pub async fn find_clips_containing_note(
        &self,
        note_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<clip_note::Model>> {
        ClipNote::find()
            .filter(clip_note::Column::NoteId.eq(note_id))
            .inner_join(Clip)
            .filter(clip::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Add a note to a clip.
    pub async fn add_note_to_clip(
        &self,
        id: String,
        clip_id: String,
        note_id: String,
        comment: Option<String>,
    ) -> AppResult<clip_note::Model> {
        // Get max display order
        let max_order: Option<i32> = ClipNote::find()
            .filter(clip_note::Column::ClipId.eq(&clip_id))
            .order_by(clip_note::Column::DisplayOrder, Order::Desc)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .map(|cn| cn.display_order);

        let display_order = max_order.unwrap_or(0) + 1;

        let active_model = clip_note::ActiveModel {
            id: Set(id),
            clip_id: Set(clip_id.clone()),
            note_id: Set(note_id),
            display_order: Set(display_order),
            comment: Set(comment),
            created_at: Set(Utc::now().into()),
        };

        let clip_note = active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Update notes count
        self.increment_notes_count(&clip_id).await?;

        Ok(clip_note)
    }

    /// Remove a note from a clip.
    pub async fn remove_note_from_clip(&self, clip_id: &str, note_id: &str) -> AppResult<()> {
        let deleted = ClipNote::delete_many()
            .filter(clip_note::Column::ClipId.eq(clip_id))
            .filter(clip_note::Column::NoteId.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if deleted.rows_affected > 0 {
            self.decrement_notes_count(clip_id).await?;
        }

        Ok(())
    }

    /// Update clip note display order.
    pub async fn update_clip_note_order(&self, id: &str, display_order: i32) -> AppResult<()> {
        let clip_note = ClipNote::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("ClipNote not found: {id}")))?;

        let mut active: clip_note::ActiveModel = clip_note.into();
        active.display_order = Set(display_order);

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Update clip note comment.
    pub async fn update_clip_note_comment(
        &self,
        id: &str,
        comment: Option<String>,
    ) -> AppResult<()> {
        let clip_note = ClipNote::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("ClipNote not found: {id}")))?;

        let mut active: clip_note::ActiveModel = clip_note.into();
        active.comment = Set(comment);

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Count notes in a clip.
    pub async fn count_notes_in_clip(&self, clip_id: &str) -> AppResult<u64> {
        ClipNote::find()
            .filter(clip_note::Column::ClipId.eq(clip_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    // ==================== Helper Methods ====================

    /// Increment notes count atomically.
    async fn increment_notes_count(&self, clip_id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Clip::update_many()
            .col_expr(
                clip::Column::NotesCount,
                Expr::col(clip::Column::NotesCount).add(1),
            )
            .filter(clip::Column::Id.eq(clip_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Decrement notes count atomically.
    async fn decrement_notes_count(&self, clip_id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Clip::update_many()
            .col_expr(
                clip::Column::NotesCount,
                Expr::cust("GREATEST(notes_count - 1, 0)"),
            )
            .filter(clip::Column::Id.eq(clip_id))
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
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};

    fn create_test_clip(id: &str, user_id: &str, name: &str, is_public: bool) -> clip::Model {
        clip::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            description: None,
            is_public,
            notes_count: 0,
            display_order: 0,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    fn create_test_clip_note(id: &str, clip_id: &str, note_id: &str) -> clip_note::Model {
        clip_note::Model {
            id: id.to_string(),
            clip_id: clip_id.to_string(),
            note_id: note_id.to_string(),
            display_order: 0,
            comment: None,
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let clip = create_test_clip("clip1", "user1", "My Clip", false);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip.clone()]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let result = repo.find_by_id("clip1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Clip");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<clip::Model>::new()])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let clip1 = create_test_clip("clip1", "user1", "Clip 1", false);
        let clip2 = create_test_clip("clip2", "user1", "Clip 2", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip1, clip2]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let result = repo.find_by_user("user1", 10, 0).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_find_public_by_user() {
        let clip = create_test_clip("clip1", "user1", "Public Clip", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let result = repo.find_public_by_user("user1", 10, 0).await.unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].is_public);
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

        let repo = ClipRepository::new(db);
        let count = repo.count_by_user("user1").await.unwrap();

        assert_eq!(count, 5);
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

        let repo = ClipRepository::new(db);
        let result = repo.delete("clip1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_is_note_in_clip() {
        let clip_note = create_test_clip_note("cn1", "clip1", "note1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip_note]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let result = repo.is_note_in_clip("clip1", "note1").await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_is_note_not_in_clip() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<clip_note::Model>::new()])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let result = repo.is_note_in_clip("clip1", "note1").await.unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn test_find_notes_in_clip() {
        let cn1 = create_test_clip_note("cn1", "clip1", "note1");
        let cn2 = create_test_clip_note("cn2", "clip1", "note2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[cn1, cn2]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let result = repo.find_notes_in_clip("clip1", 10, 0).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_count_notes_in_clip() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[maplit::btreemap! {
                    "num_items" => sea_orm::Value::BigInt(Some(3))
                }]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let count = repo.count_notes_in_clip("clip1").await.unwrap();

        assert_eq!(count, 3);
    }
}
