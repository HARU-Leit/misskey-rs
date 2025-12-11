//! Scheduled note repository.

use std::sync::Arc;

use crate::entities::{ScheduledNote, scheduled_note};
use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use crate::entities::scheduled_note::ScheduledStatus;

/// Scheduled note repository for database operations.
#[derive(Clone)]
pub struct ScheduledNoteRepository {
    db: Arc<DatabaseConnection>,
}

impl ScheduledNoteRepository {
    /// Create a new scheduled note repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a scheduled note by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<scheduled_note::Model>> {
        ScheduledNote::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a scheduled note by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<scheduled_note::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Scheduled note {id} not found")))
    }

    /// Find all scheduled notes for a user.
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<scheduled_note::Model>> {
        ScheduledNote::find()
            .filter(scheduled_note::Column::UserId.eq(user_id))
            .order_by_asc(scheduled_note::Column::ScheduledAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find pending scheduled notes for a user.
    pub async fn find_pending_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<scheduled_note::Model>> {
        ScheduledNote::find()
            .filter(scheduled_note::Column::UserId.eq(user_id))
            .filter(scheduled_note::Column::Status.eq(ScheduledStatus::Pending))
            .order_by_asc(scheduled_note::Column::ScheduledAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count scheduled notes for a user.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        ScheduledNote::find()
            .filter(scheduled_note::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count pending scheduled notes for a user.
    pub async fn count_pending_by_user(&self, user_id: &str) -> AppResult<u64> {
        ScheduledNote::find()
            .filter(scheduled_note::Column::UserId.eq(user_id))
            .filter(scheduled_note::Column::Status.eq(ScheduledStatus::Pending))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find notes that are due for posting.
    pub async fn find_due_for_posting(&self, limit: u64) -> AppResult<Vec<scheduled_note::Model>> {
        let now = Utc::now();

        ScheduledNote::find()
            .filter(scheduled_note::Column::Status.eq(ScheduledStatus::Pending))
            .filter(scheduled_note::Column::ScheduledAt.lte(now))
            .order_by_asc(scheduled_note::Column::ScheduledAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find notes that are stuck in processing state (for recovery).
    pub async fn find_stuck_processing(
        &self,
        older_than_minutes: i64,
        limit: u64,
    ) -> AppResult<Vec<scheduled_note::Model>> {
        let cutoff = Utc::now() - chrono::Duration::minutes(older_than_minutes);

        ScheduledNote::find()
            .filter(scheduled_note::Column::Status.eq(ScheduledStatus::Processing))
            .filter(scheduled_note::Column::UpdatedAt.lt(cutoff))
            .order_by_asc(scheduled_note::Column::ScheduledAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new scheduled note.
    pub async fn create(
        &self,
        model: scheduled_note::ActiveModel,
    ) -> AppResult<scheduled_note::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a scheduled note.
    pub async fn update(
        &self,
        model: scheduled_note::ActiveModel,
    ) -> AppResult<scheduled_note::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Mark a scheduled note as processing.
    pub async fn mark_processing(&self, id: &str) -> AppResult<scheduled_note::Model> {
        let note = self.get_by_id(id).await?;
        let mut active: scheduled_note::ActiveModel = note.into();
        active.status = Set(ScheduledStatus::Processing);
        active.updated_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Mark a scheduled note as posted.
    pub async fn mark_posted(
        &self,
        id: &str,
        posted_note_id: &str,
    ) -> AppResult<scheduled_note::Model> {
        let note = self.get_by_id(id).await?;
        let mut active: scheduled_note::ActiveModel = note.into();
        active.status = Set(ScheduledStatus::Posted);
        active.posted_note_id = Set(Some(posted_note_id.to_string()));
        active.updated_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Mark a scheduled note as failed.
    pub async fn mark_failed(
        &self,
        id: &str,
        error_message: &str,
    ) -> AppResult<scheduled_note::Model> {
        let note = self.get_by_id(id).await?;
        let retry_count = note.retry_count;
        let mut active: scheduled_note::ActiveModel = note.into();
        active.status = Set(ScheduledStatus::Failed);
        active.error_message = Set(Some(error_message.to_string()));
        active.retry_count = Set(retry_count + 1);
        active.updated_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Mark a scheduled note as cancelled.
    pub async fn mark_cancelled(&self, id: &str) -> AppResult<scheduled_note::Model> {
        let note = self.get_by_id(id).await?;
        let mut active: scheduled_note::ActiveModel = note.into();
        active.status = Set(ScheduledStatus::Cancelled);
        active.updated_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Reset a failed note back to pending for retry.
    pub async fn reset_to_pending(&self, id: &str) -> AppResult<scheduled_note::Model> {
        let note = self.get_by_id(id).await?;
        let mut active: scheduled_note::ActiveModel = note.into();
        active.status = Set(ScheduledStatus::Pending);
        active.error_message = Set(None);
        active.updated_at = Set(Some(Utc::now().into()));
        self.update(active).await
    }

    /// Delete a scheduled note.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        ScheduledNote::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete old posted/cancelled notes (cleanup).
    pub async fn delete_old_completed(&self, older_than_days: i64) -> AppResult<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days);

        let result = ScheduledNote::delete_many()
            .filter(
                scheduled_note::Column::Status
                    .eq(ScheduledStatus::Posted)
                    .or(scheduled_note::Column::Status.eq(ScheduledStatus::Cancelled)),
            )
            .filter(scheduled_note::Column::UpdatedAt.lt(cutoff))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::entities::scheduled_note::{ScheduledStatus, ScheduledVisibility};
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use serde_json::json;

    fn create_test_scheduled_note(id: &str, user_id: &str) -> scheduled_note::Model {
        scheduled_note::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            text: Some("Test note".to_string()),
            cw: None,
            visibility: ScheduledVisibility::Public,
            visible_user_ids: json!([]),
            file_ids: json!([]),
            reply_id: None,
            renote_id: None,
            poll: None,
            scheduled_at: (Utc::now() + chrono::Duration::hours(1)).into(),
            status: ScheduledStatus::Pending,
            posted_note_id: None,
            error_message: None,
            retry_count: 0,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let note = create_test_scheduled_note("note1", "user1");
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note.clone()]])
                .into_connection(),
        );

        let repo = ScheduledNoteRepository::new(db);
        let result = repo.find_by_id("note1").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "note1");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<scheduled_note::Model>::new()])
                .into_connection(),
        );

        let repo = ScheduledNoteRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let notes = vec![
            create_test_scheduled_note("note1", "user1"),
            create_test_scheduled_note("note2", "user1"),
        ];
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([notes.clone()])
                .into_connection(),
        );

        let repo = ScheduledNoteRepository::new(db);
        let result = repo.find_by_user("user1", 10, 0).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_count_by_user() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[maplit::btreemap! {
                    "num_items" => sea_orm::Value::BigInt(Some(3))
                }]])
                .into_connection(),
        );

        let repo = ScheduledNoteRepository::new(db);
        let result = repo.count_by_user("user1").await.unwrap();
        assert_eq!(result, 3);
    }
}
