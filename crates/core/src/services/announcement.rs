//! Announcement service.

use chrono::{DateTime, Utc};
use misskey_common::{AppResult, id::IdGenerator};
use misskey_db::entities::announcement;
use misskey_db::repositories::AnnouncementRepository;

/// Service for managing announcements.
#[derive(Clone)]
pub struct AnnouncementService {
    announcement_repo: AnnouncementRepository,
    id_gen: IdGenerator,
}

impl AnnouncementService {
    /// Create a new announcement service.
    #[must_use]
    pub const fn new(announcement_repo: AnnouncementRepository) -> Self {
        Self {
            announcement_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// List all active announcements.
    pub async fn list_active(&self) -> AppResult<Vec<announcement::Model>> {
        self.announcement_repo.find_active().await
    }

    /// List all announcements (for admin).
    pub async fn list_all(&self, limit: u64, offset: u64) -> AppResult<Vec<announcement::Model>> {
        self.announcement_repo.find_all(limit, offset).await
    }

    /// Count all announcements.
    pub async fn count(&self) -> AppResult<u64> {
        self.announcement_repo.count().await
    }

    /// Get unread announcements for a user.
    pub async fn get_unread_for_user(&self, user_id: &str) -> AppResult<Vec<announcement::Model>> {
        self.announcement_repo.find_unread_for_user(user_id).await
    }

    /// Get an announcement by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<announcement::Model>> {
        self.announcement_repo.find_by_id(id).await
    }

    /// Check if a user has read an announcement.
    pub async fn has_read(&self, user_id: &str, announcement_id: &str) -> AppResult<bool> {
        self.announcement_repo
            .has_read(user_id, announcement_id)
            .await
    }

    /// Mark an announcement as read.
    pub async fn mark_as_read(&self, user_id: &str, announcement_id: &str) -> AppResult<()> {
        // Check if already read
        if self
            .announcement_repo
            .has_read(user_id, announcement_id)
            .await?
        {
            return Ok(());
        }

        let id = self.id_gen.generate();
        self.announcement_repo
            .mark_as_read(id, user_id.to_string(), announcement_id.to_string())
            .await?;

        Ok(())
    }

    /// Create a new announcement.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        title: String,
        text: String,
        image_url: Option<String>,
        is_active: bool,
        needs_confirmation_to_read: bool,
        display_order: i32,
        icon: Option<String>,
        foreground_color: Option<String>,
        background_color: Option<String>,
        starts_at: Option<DateTime<Utc>>,
        ends_at: Option<DateTime<Utc>>,
    ) -> AppResult<announcement::Model> {
        let id = self.id_gen.generate();

        self.announcement_repo
            .create(
                id,
                title,
                text,
                image_url,
                is_active,
                needs_confirmation_to_read,
                display_order,
                icon,
                foreground_color,
                background_color,
                starts_at,
                ends_at,
            )
            .await
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
        starts_at: Option<Option<DateTime<Utc>>>,
        ends_at: Option<Option<DateTime<Utc>>>,
    ) -> AppResult<announcement::Model> {
        self.announcement_repo
            .update(
                id,
                title,
                text,
                image_url,
                is_active,
                needs_confirmation_to_read,
                display_order,
                icon,
                foreground_color,
                background_color,
                starts_at,
                ends_at,
            )
            .await
    }

    /// Delete an announcement.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        self.announcement_repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use misskey_db::entities::{announcement, announcement_read};
    use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
    use std::sync::Arc;

    fn create_mock_announcement(id: &str, title: &str, is_active: bool) -> announcement::Model {
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
    async fn test_list_active_returns_active_announcements() {
        let ann1 = create_mock_announcement("ann1", "Active Announcement 1", true);
        let ann2 = create_mock_announcement("ann2", "Active Announcement 2", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[ann1.clone(), ann2.clone()]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let results = service.list_active().await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "ann1");
        assert_eq!(results[1].id, "ann2");
    }

    #[tokio::test]
    async fn test_list_all_with_pagination() {
        let ann1 = create_mock_announcement("ann1", "Announcement 1", true);
        let ann2 = create_mock_announcement("ann2", "Announcement 2", false);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[ann1, ann2]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let results = service.list_all(10, 0).await.unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_id_returns_announcement() {
        let ann = create_mock_announcement("ann1", "Test Announcement", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[ann.clone()]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let result = service.get_by_id("ann1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().title, "Test Announcement");
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_missing() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<announcement::Model>::new()])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let result = service.get_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_has_read_returns_false_when_not_read() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<announcement_read::Model>::new()])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let result = service.has_read("user1", "ann1").await.unwrap();

        assert!(!result);
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
        let service = AnnouncementService::new(repo);

        let result = service.has_read("user1", "ann1").await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_mark_as_read_skips_if_already_read() {
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
        let service = AnnouncementService::new(repo);

        // Should return Ok without inserting new record
        let result = service.mark_as_read("user1", "ann1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_count_returns_correct_count() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[maplit::btreemap! {
                    "num_items" => sea_orm::Value::BigInt(Some(42))
                }]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let count = service.count().await.unwrap();

        assert_eq!(count, 42);
    }

    #[tokio::test]
    async fn test_delete_removes_announcement() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_exec_results([
                    MockExecResult {
                        last_insert_id: 0,
                        rows_affected: 2, // read records deleted
                    },
                    MockExecResult {
                        last_insert_id: 0,
                        rows_affected: 1, // announcement deleted
                    },
                ])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let result = service.delete("ann1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_unread_for_user() {
        let ann1 = create_mock_announcement("ann1", "Unread Announcement", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                // First query: get read announcement IDs
                .append_query_results([Vec::<announcement_read::Model>::new()])
                // Second query: get active announcements not in read list
                .append_query_results([[ann1.clone()]])
                .into_connection(),
        );

        let repo = AnnouncementRepository::new(db);
        let service = AnnouncementService::new(repo);

        let results = service.get_unread_for_user("user1").await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Unread Announcement");
    }
}
