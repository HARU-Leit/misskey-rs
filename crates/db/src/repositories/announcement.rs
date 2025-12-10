//! Announcement repository.

use std::sync::Arc;

use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entities::{announcement, announcement_read, Announcement, AnnouncementRead};

/// Repository for announcement operations.
#[derive(Clone)]
pub struct AnnouncementRepository {
    db: Arc<DatabaseConnection>,
}

impl AnnouncementRepository {
    /// Create a new announcement repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find announcement by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<announcement::Model>> {
        Announcement::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all active announcements.
    pub async fn find_active(&self) -> AppResult<Vec<announcement::Model>> {
        let now = Utc::now();

        Announcement::find()
            .filter(announcement::Column::IsActive.eq(true))
            .filter(
                Condition::any()
                    .add(announcement::Column::StartsAt.is_null())
                    .add(announcement::Column::StartsAt.lte(now)),
            )
            .filter(
                Condition::any()
                    .add(announcement::Column::EndsAt.is_null())
                    .add(announcement::Column::EndsAt.gte(now)),
            )
            .order_by(announcement::Column::DisplayOrder, Order::Asc)
            .order_by(announcement::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all announcements (for admin).
    pub async fn find_all(&self, limit: u64, offset: u64) -> AppResult<Vec<announcement::Model>> {
        Announcement::find()
            .order_by(announcement::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count all announcements.
    pub async fn count(&self) -> AppResult<u64> {
        Announcement::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find unread announcements for a user.
    pub async fn find_unread_for_user(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<announcement::Model>> {
        let now = Utc::now();

        // Get all read announcement IDs for this user
        let read_ids: Vec<String> = AnnouncementRead::find()
            .filter(announcement_read::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .into_iter()
            .map(|r| r.announcement_id)
            .collect();

        // Find active announcements not in the read list
        let mut query = Announcement::find()
            .filter(announcement::Column::IsActive.eq(true))
            .filter(
                Condition::any()
                    .add(announcement::Column::StartsAt.is_null())
                    .add(announcement::Column::StartsAt.lte(now)),
            )
            .filter(
                Condition::any()
                    .add(announcement::Column::EndsAt.is_null())
                    .add(announcement::Column::EndsAt.gte(now)),
            );

        if !read_ids.is_empty() {
            query = query.filter(announcement::Column::Id.is_not_in(read_ids));
        }

        query
            .order_by(announcement::Column::DisplayOrder, Order::Asc)
            .order_by(announcement::Column::CreatedAt, Order::Desc)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a user has read an announcement.
    pub async fn has_read(&self, user_id: &str, announcement_id: &str) -> AppResult<bool> {
        let read = AnnouncementRead::find()
            .filter(announcement_read::Column::UserId.eq(user_id))
            .filter(announcement_read::Column::AnnouncementId.eq(announcement_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(read.is_some())
    }

    /// Mark an announcement as read by a user.
    pub async fn mark_as_read(
        &self,
        id: String,
        user_id: String,
        announcement_id: String,
    ) -> AppResult<announcement_read::Model> {
        let active_model = announcement_read::ActiveModel {
            id: Set(id),
            announcement_id: Set(announcement_id.clone()),
            user_id: Set(user_id),
            created_at: Set(Utc::now()),
        };

        let read = active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Increment the reads count
        let announcement = Announcement::find_by_id(&announcement_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(ann) = announcement {
            let mut active: announcement::ActiveModel = ann.into();
            active.reads_count = Set(active.reads_count.unwrap() + 1);
            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(read)
    }

    /// Create a new announcement.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        id: String,
        title: String,
        text: String,
        image_url: Option<String>,
        is_active: bool,
        needs_confirmation_to_read: bool,
        display_order: i32,
        icon: Option<String>,
        foreground_color: Option<String>,
        background_color: Option<String>,
        starts_at: Option<chrono::DateTime<Utc>>,
        ends_at: Option<chrono::DateTime<Utc>>,
    ) -> AppResult<announcement::Model> {
        let active_model = announcement::ActiveModel {
            id: Set(id),
            title: Set(title),
            text: Set(text),
            image_url: Set(image_url),
            is_active: Set(is_active),
            needs_confirmation_to_read: Set(needs_confirmation_to_read),
            display_order: Set(display_order),
            icon: Set(icon),
            foreground_color: Set(foreground_color),
            background_color: Set(background_color),
            starts_at: Set(starts_at),
            ends_at: Set(ends_at),
            reads_count: Set(0),
            created_at: Set(Utc::now()),
            updated_at: Set(None),
        };

        active_model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an announcement.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        id: &str,
        title: Option<String>,
        text: Option<String>,
        image_url: Option<Option<String>>,
        is_active: Option<bool>,
        needs_confirmation_to_read: Option<bool>,
        display_order: Option<i32>,
        icon: Option<Option<String>>,
        foreground_color: Option<Option<String>>,
        background_color: Option<Option<String>>,
        starts_at: Option<Option<chrono::DateTime<Utc>>>,
        ends_at: Option<Option<chrono::DateTime<Utc>>>,
    ) -> AppResult<announcement::Model> {
        let announcement = Announcement::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Announcement not found: {id}")))?;

        let mut active: announcement::ActiveModel = announcement.into();

        if let Some(title) = title {
            active.title = Set(title);
        }
        if let Some(text) = text {
            active.text = Set(text);
        }
        if let Some(image_url) = image_url {
            active.image_url = Set(image_url);
        }
        if let Some(is_active) = is_active {
            active.is_active = Set(is_active);
        }
        if let Some(needs_confirmation) = needs_confirmation_to_read {
            active.needs_confirmation_to_read = Set(needs_confirmation);
        }
        if let Some(order) = display_order {
            active.display_order = Set(order);
        }
        if let Some(icon) = icon {
            active.icon = Set(icon);
        }
        if let Some(fg) = foreground_color {
            active.foreground_color = Set(fg);
        }
        if let Some(bg) = background_color {
            active.background_color = Set(bg);
        }
        if let Some(starts) = starts_at {
            active.starts_at = Set(starts);
        }
        if let Some(ends) = ends_at {
            active.ends_at = Set(ends);
        }

        active.updated_at = Set(Some(Utc::now()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete an announcement.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        // First delete all read records
        AnnouncementRead::delete_many()
            .filter(announcement_read::Column::AnnouncementId.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Then delete the announcement
        Announcement::delete_by_id(id)
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

    fn create_test_announcement(id: &str, title: &str, is_active: bool) -> announcement::Model {
        announcement::Model {
            id: id.to_string(),
            title: title.to_string(),
            text: "Test announcement text".to_string(),
            image_url: None,
            is_active,
            needs_confirmation_to_read: false,
            display_order: 0,
            icon: None,
            foreground_color: None,
            background_color: None,
            starts_at: None,
            ends_at: None,
            reads_count: 0,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_id_returns_announcement() {
        let announcement = create_test_announcement("ann1", "Test Title", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[announcement.clone()]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let result = repo.find_by_id("ann1").await.unwrap();

        assert!(result.is_some());
        let found = result.unwrap();
        assert_eq!(found.id, "ann1");
        assert_eq!(found.title, "Test Title");
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<announcement::Model>::new()])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_active_returns_only_active() {
        let active1 = create_test_announcement("ann1", "Active 1", true);
        let active2 = create_test_announcement("ann2", "Active 2", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[active1, active2]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let results = repo.find_active().await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|a| a.is_active));
    }

    #[tokio::test]
    async fn test_count_returns_correct_count() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[maplit::btreemap! {
                    "num_items" => sea_orm::Value::BigInt(Some(5))
                }]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let count = repo.count().await.unwrap();

        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_has_read_returns_false_when_not_read() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<announcement_read::Model>::new()])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let has_read = repo.has_read("user1", "ann1").await.unwrap();

        assert!(!has_read);
    }

    #[tokio::test]
    async fn test_has_read_returns_true_when_read() {
        let read_record = announcement_read::Model {
            id: "read1".to_string(),
            announcement_id: "ann1".to_string(),
            user_id: "user1".to_string(),
            created_at: Utc::now(),
        };

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[read_record]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let has_read = repo.has_read("user1", "ann1").await.unwrap();

        assert!(has_read);
    }

    #[tokio::test]
    async fn test_delete_removes_read_records_first() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_exec_results([
                    MockExecResult {
                        last_insert_id: 0,
                        rows_affected: 3, // 3 read records deleted
                    },
                    MockExecResult {
                        last_insert_id: 0,
                        rows_affected: 1, // announcement deleted
                    },
                ])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let result = repo.delete("ann1").await;

        assert!(result.is_ok());
    }
}
