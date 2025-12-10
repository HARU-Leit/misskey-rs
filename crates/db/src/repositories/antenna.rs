//! Antenna repository.

use std::sync::Arc;

use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entities::{antenna, antenna_note, Antenna, AntennaNotes};

/// Repository for antenna operations.
#[derive(Clone)]
pub struct AntennaRepository {
    db: Arc<DatabaseConnection>,
}

impl AntennaRepository {
    /// Create a new antenna repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ==================== Antenna Operations ====================

    /// Find antenna by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<antenna::Model>> {
        Antenna::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get antenna by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<antenna::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Antenna not found: {id}")))
    }

    /// Find antennas by user ID.
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<antenna::Model>> {
        Antenna::find()
            .filter(antenna::Column::UserId.eq(user_id))
            .order_by(antenna::Column::DisplayOrder, Order::Asc)
            .order_by(antenna::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find active antennas by user ID.
    pub async fn find_active_by_user(&self, user_id: &str) -> AppResult<Vec<antenna::Model>> {
        Antenna::find()
            .filter(antenna::Column::UserId.eq(user_id))
            .filter(antenna::Column::IsActive.eq(true))
            .order_by(antenna::Column::DisplayOrder, Order::Asc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all active antennas (for note matching).
    pub async fn find_all_active(&self) -> AppResult<Vec<antenna::Model>> {
        Antenna::find()
            .filter(antenna::Column::IsActive.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count antennas by user ID.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        Antenna::find()
            .filter(antenna::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new antenna.
    pub async fn create(&self, model: antenna::ActiveModel) -> AppResult<antenna::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an antenna.
    pub async fn update(&self, model: antenna::ActiveModel) -> AppResult<antenna::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update antenna display order.
    pub async fn update_display_order(&self, id: &str, display_order: i32) -> AppResult<()> {
        let antenna = self.get_by_id(id).await?;
        let mut active: antenna::ActiveModel = antenna.into();
        active.display_order = Set(display_order);
        active.updated_at = Set(Some(Utc::now().into()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete an antenna.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        Antenna::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Update last used timestamp and increment notes count.
    pub async fn update_last_used(&self, id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Antenna::update_many()
            .col_expr(antenna::Column::LastUsedAt, Expr::value(Utc::now()))
            .col_expr(
                antenna::Column::NotesCount,
                Expr::col(antenna::Column::NotesCount).add(1),
            )
            .filter(antenna::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ==================== Antenna Note Operations ====================

    /// Add a note to an antenna.
    pub async fn add_note(
        &self,
        id: String,
        antenna_id: String,
        note_id: String,
    ) -> AppResult<antenna_note::Model> {
        let model = antenna_note::ActiveModel {
            id: Set(id),
            antenna_id: Set(antenna_id.clone()),
            note_id: Set(note_id),
            is_read: Set(false),
            created_at: Set(Utc::now().into()),
        };

        let antenna_note = model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Update last used timestamp
        let _ = self.update_last_used(&antenna_id).await;

        Ok(antenna_note)
    }

    /// Check if a note is already in an antenna.
    pub async fn is_note_in_antenna(&self, antenna_id: &str, note_id: &str) -> AppResult<bool> {
        let count = AntennaNotes::find()
            .filter(antenna_note::Column::AntennaId.eq(antenna_id))
            .filter(antenna_note::Column::NoteId.eq(note_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
    }

    /// Find notes in an antenna (paginated).
    pub async fn find_notes_in_antenna(
        &self,
        antenna_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<antenna_note::Model>> {
        let mut query = AntennaNotes::find()
            .filter(antenna_note::Column::AntennaId.eq(antenna_id))
            .order_by(antenna_note::Column::Id, Order::Desc)
            .limit(limit);

        if let Some(until) = until_id {
            query = query.filter(antenna_note::Column::Id.lt(until));
        }

        query
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count notes in an antenna.
    pub async fn count_notes_in_antenna(&self, antenna_id: &str) -> AppResult<u64> {
        AntennaNotes::find()
            .filter(antenna_note::Column::AntennaId.eq(antenna_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count unread notes in an antenna.
    pub async fn count_unread_notes(&self, antenna_id: &str) -> AppResult<u64> {
        AntennaNotes::find()
            .filter(antenna_note::Column::AntennaId.eq(antenna_id))
            .filter(antenna_note::Column::IsRead.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Mark notes in an antenna as read.
    pub async fn mark_notes_as_read(&self, antenna_id: &str) -> AppResult<()> {
        AntennaNotes::update_many()
            .col_expr(antenna_note::Column::IsRead, true.into())
            .filter(antenna_note::Column::AntennaId.eq(antenna_id))
            .filter(antenna_note::Column::IsRead.eq(false))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Clean up old antenna notes (for maintenance).
    pub async fn cleanup_old_notes(&self, older_than_days: i64, limit: u64) -> AppResult<u64> {
        use chrono::Duration;

        let cutoff = Utc::now() - Duration::days(older_than_days);

        // Get IDs to delete (limit to avoid long-running queries)
        let old_notes = AntennaNotes::find()
            .filter(antenna_note::Column::CreatedAt.lt(cutoff))
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let count = old_notes.len() as u64;

        for note in old_notes {
            AntennaNotes::delete_by_id(note.id)
                .exec(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::antenna::AntennaSource;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
    use serde_json::json;

    fn create_test_antenna(id: &str, user_id: &str, name: &str) -> antenna::Model {
        antenna::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            src: AntennaSource::All,
            user_list_id: None,
            keywords: json!([["test"]]),
            exclude_keywords: json!([]),
            users: json!([]),
            instances: json!([]),
            case_sensitive: false,
            with_replies: false,
            with_file: false,
            notify: false,
            local_only: false,
            is_active: true,
            display_order: 0,
            notes_count: 0,
            last_used_at: None,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    fn create_test_antenna_note(id: &str, antenna_id: &str, note_id: &str) -> antenna_note::Model {
        antenna_note::Model {
            id: id.to_string(),
            antenna_id: antenna_id.to_string(),
            note_id: note_id.to_string(),
            is_read: false,
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let antenna = create_test_antenna("ant1", "user1", "My Antenna");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[antenna.clone()]])
                .into_connection(),
        );

        let repo = AntennaRepository::new(db);
        let result = repo.find_by_id("ant1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Antenna");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<antenna::Model>::new()])
                .into_connection(),
        );

        let repo = AntennaRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let ant1 = create_test_antenna("ant1", "user1", "Antenna 1");
        let ant2 = create_test_antenna("ant2", "user1", "Antenna 2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[ant1, ant2]])
                .into_connection(),
        );

        let repo = AntennaRepository::new(db);
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

        let repo = AntennaRepository::new(db);
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

        let repo = AntennaRepository::new(db);
        let result = repo.delete("ant1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_notes_in_antenna() {
        let an1 = create_test_antenna_note("an1", "ant1", "note1");
        let an2 = create_test_antenna_note("an2", "ant1", "note2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[an1, an2]])
                .into_connection(),
        );

        let repo = AntennaRepository::new(db);
        let result = repo.find_notes_in_antenna("ant1", 10, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
