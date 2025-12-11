//! Scheduled note service.

use chrono::{DateTime, Duration, Utc};
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::scheduled_note::{self, ScheduledStatus, ScheduledVisibility};
use misskey_db::repositories::ScheduledNoteRepository;
use sea_orm::Set;
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

/// Maximum number of pending scheduled notes per user.
const MAX_PENDING_NOTES_PER_USER: u64 = 100;

/// Maximum time in the future a note can be scheduled (30 days).
const MAX_SCHEDULE_DAYS: i64 = 30;

/// Minimum time in the future a note can be scheduled (1 minute).
const MIN_SCHEDULE_MINUTES: i64 = 1;

/// Maximum retry count before giving up.
pub const MAX_RETRY_COUNT: i32 = 3;

/// Input for creating a scheduled note.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduledNoteInput {
    #[validate(length(max = 3000))]
    pub text: Option<String>,
    #[validate(length(max = 512))]
    pub cw: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: ScheduledVisibility,
    #[serde(default)]
    pub visible_user_ids: Vec<String>,
    #[serde(default)]
    #[validate(length(max = 16))]
    pub file_ids: Vec<String>,
    pub reply_id: Option<String>,
    pub renote_id: Option<String>,
    pub poll: Option<serde_json::Value>,
    pub scheduled_at: DateTime<Utc>,
}

fn default_visibility() -> ScheduledVisibility {
    ScheduledVisibility::Public
}

/// Input for updating a scheduled note.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScheduledNoteInput {
    pub note_id: String,
    #[validate(length(max = 3000))]
    pub text: Option<Option<String>>,
    #[validate(length(max = 512))]
    pub cw: Option<Option<String>>,
    pub visibility: Option<ScheduledVisibility>,
    pub visible_user_ids: Option<Vec<String>>,
    #[validate(length(max = 16))]
    pub file_ids: Option<Vec<String>>,
    pub scheduled_at: Option<DateTime<Utc>>,
}

/// Service for managing scheduled notes.
#[derive(Clone)]
pub struct ScheduledNoteService {
    scheduled_note_repo: ScheduledNoteRepository,
    id_gen: IdGenerator,
}

impl ScheduledNoteService {
    /// Create a new scheduled note service.
    #[must_use]
    pub const fn new(scheduled_note_repo: ScheduledNoteRepository) -> Self {
        Self {
            scheduled_note_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Get a scheduled note by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<scheduled_note::Model>> {
        self.scheduled_note_repo.find_by_id(id).await
    }

    /// Get a scheduled note by ID with ownership check.
    pub async fn get_by_id_for_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<scheduled_note::Model> {
        let note = self.scheduled_note_repo.get_by_id(id).await?;

        if note.user_id != user_id {
            return Err(AppError::Forbidden(
                "Not the owner of this scheduled note".to_string(),
            ));
        }

        Ok(note)
    }

    /// List scheduled notes for a user.
    pub async fn list_notes(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<scheduled_note::Model>> {
        self.scheduled_note_repo
            .find_by_user(user_id, limit, offset)
            .await
    }

    /// List pending scheduled notes for a user.
    pub async fn list_pending_notes(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<scheduled_note::Model>> {
        self.scheduled_note_repo
            .find_pending_by_user(user_id, limit, offset)
            .await
    }

    /// Count scheduled notes for a user.
    pub async fn count_notes(&self, user_id: &str) -> AppResult<u64> {
        self.scheduled_note_repo.count_by_user(user_id).await
    }

    /// Count pending scheduled notes for a user.
    pub async fn count_pending_notes(&self, user_id: &str) -> AppResult<u64> {
        self.scheduled_note_repo
            .count_pending_by_user(user_id)
            .await
    }

    /// Create a new scheduled note.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateScheduledNoteInput,
    ) -> AppResult<scheduled_note::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check content exists
        if input.text.is_none() && input.file_ids.is_empty() && input.renote_id.is_none() {
            return Err(AppError::Validation(
                "Note must have text, files, or be a renote".to_string(),
            ));
        }

        // Check pending note limit
        let pending_count = self
            .scheduled_note_repo
            .count_pending_by_user(user_id)
            .await?;
        if pending_count >= MAX_PENDING_NOTES_PER_USER {
            return Err(AppError::Validation(format!(
                "Maximum of {} pending scheduled notes allowed",
                MAX_PENDING_NOTES_PER_USER
            )));
        }

        // Validate scheduled time
        let now = Utc::now();
        let min_time = now + Duration::minutes(MIN_SCHEDULE_MINUTES);
        let max_time = now + Duration::days(MAX_SCHEDULE_DAYS);

        if input.scheduled_at < min_time {
            return Err(AppError::Validation(format!(
                "Scheduled time must be at least {} minute(s) in the future",
                MIN_SCHEDULE_MINUTES
            )));
        }

        if input.scheduled_at > max_time {
            return Err(AppError::Validation(format!(
                "Scheduled time must be within {} days",
                MAX_SCHEDULE_DAYS
            )));
        }

        let id = self.id_gen.generate();

        let model = scheduled_note::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            text: Set(input.text),
            cw: Set(input.cw),
            visibility: Set(input.visibility),
            visible_user_ids: Set(json!(input.visible_user_ids)),
            file_ids: Set(json!(input.file_ids)),
            reply_id: Set(input.reply_id),
            renote_id: Set(input.renote_id),
            poll: Set(input.poll.map(Into::into)),
            scheduled_at: Set(input.scheduled_at.into()),
            status: Set(ScheduledStatus::Pending),
            posted_note_id: Set(None),
            error_message: Set(None),
            retry_count: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.scheduled_note_repo.create(model).await
    }

    /// Update a scheduled note.
    pub async fn update(
        &self,
        user_id: &str,
        input: UpdateScheduledNoteInput,
    ) -> AppResult<scheduled_note::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Get note and verify ownership
        let note = self.get_by_id_for_user(&input.note_id, user_id).await?;

        // Can only update pending notes
        if note.status != ScheduledStatus::Pending {
            return Err(AppError::Validation(
                "Can only update pending scheduled notes".to_string(),
            ));
        }

        // Validate scheduled time if provided
        if let Some(scheduled_at) = input.scheduled_at {
            let now = Utc::now();
            let min_time = now + Duration::minutes(MIN_SCHEDULE_MINUTES);
            let max_time = now + Duration::days(MAX_SCHEDULE_DAYS);

            if scheduled_at < min_time {
                return Err(AppError::Validation(format!(
                    "Scheduled time must be at least {} minute(s) in the future",
                    MIN_SCHEDULE_MINUTES
                )));
            }

            if scheduled_at > max_time {
                return Err(AppError::Validation(format!(
                    "Scheduled time must be within {} days",
                    MAX_SCHEDULE_DAYS
                )));
            }
        }

        let now = Utc::now();
        let mut active: scheduled_note::ActiveModel = note.into();

        if let Some(text) = input.text {
            active.text = Set(text);
        }
        if let Some(cw) = input.cw {
            active.cw = Set(cw);
        }
        if let Some(visibility) = input.visibility {
            active.visibility = Set(visibility);
        }
        if let Some(visible_user_ids) = input.visible_user_ids {
            active.visible_user_ids = Set(json!(visible_user_ids));
        }
        if let Some(file_ids) = input.file_ids {
            active.file_ids = Set(json!(file_ids));
        }
        if let Some(scheduled_at) = input.scheduled_at {
            active.scheduled_at = Set(scheduled_at.into());
        }

        active.updated_at = Set(Some(now.into()));

        self.scheduled_note_repo.update(active).await
    }

    /// Cancel a scheduled note.
    pub async fn cancel(&self, note_id: &str, user_id: &str) -> AppResult<scheduled_note::Model> {
        // Get note and verify ownership
        let note = self.get_by_id_for_user(note_id, user_id).await?;

        // Can only cancel pending notes
        if note.status != ScheduledStatus::Pending {
            return Err(AppError::Validation(
                "Can only cancel pending scheduled notes".to_string(),
            ));
        }

        self.scheduled_note_repo.mark_cancelled(note_id).await
    }

    /// Delete a scheduled note.
    pub async fn delete(&self, note_id: &str, user_id: &str) -> AppResult<()> {
        // Get note and verify ownership
        self.get_by_id_for_user(note_id, user_id).await?;

        self.scheduled_note_repo.delete(note_id).await
    }

    /// Retry a failed scheduled note.
    pub async fn retry(&self, note_id: &str, user_id: &str) -> AppResult<scheduled_note::Model> {
        // Get note and verify ownership
        let note = self.get_by_id_for_user(note_id, user_id).await?;

        // Can only retry failed notes
        if note.status != ScheduledStatus::Failed {
            return Err(AppError::Validation(
                "Can only retry failed scheduled notes".to_string(),
            ));
        }

        // Check retry count
        if note.retry_count >= MAX_RETRY_COUNT {
            return Err(AppError::Validation(format!(
                "Maximum retry count ({}) exceeded",
                MAX_RETRY_COUNT
            )));
        }

        self.scheduled_note_repo.reset_to_pending(note_id).await
    }

    // ==================== Processing Methods (for job queue) ====================

    /// Find notes that are due for posting.
    pub async fn find_due_notes(&self, limit: u64) -> AppResult<Vec<scheduled_note::Model>> {
        self.scheduled_note_repo.find_due_for_posting(limit).await
    }

    /// Find notes stuck in processing state.
    pub async fn find_stuck_notes(
        &self,
        older_than_minutes: i64,
        limit: u64,
    ) -> AppResult<Vec<scheduled_note::Model>> {
        self.scheduled_note_repo
            .find_stuck_processing(older_than_minutes, limit)
            .await
    }

    /// Mark a note as processing.
    pub async fn mark_processing(&self, id: &str) -> AppResult<scheduled_note::Model> {
        self.scheduled_note_repo.mark_processing(id).await
    }

    /// Mark a note as posted.
    pub async fn mark_posted(
        &self,
        id: &str,
        posted_note_id: &str,
    ) -> AppResult<scheduled_note::Model> {
        self.scheduled_note_repo
            .mark_posted(id, posted_note_id)
            .await
    }

    /// Mark a note as failed.
    pub async fn mark_failed(
        &self,
        id: &str,
        error_message: &str,
    ) -> AppResult<scheduled_note::Model> {
        self.scheduled_note_repo
            .mark_failed(id, error_message)
            .await
    }

    /// Cleanup old completed notes.
    pub async fn cleanup_old_notes(&self, older_than_days: i64) -> AppResult<u64> {
        self.scheduled_note_repo
            .delete_old_completed(older_than_days)
            .await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

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
            scheduled_at: (Utc::now() + Duration::hours(1)).into(),
            status: ScheduledStatus::Pending,
            posted_note_id: None,
            error_message: None,
            retry_count: 0,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let note = create_test_scheduled_note("note1", "user1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note.clone()]])
                .into_connection(),
        );

        let repo = ScheduledNoteRepository::new(db);
        let service = ScheduledNoteService::new(repo);

        let result = service.get_by_id("note1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "note1");
    }

    #[tokio::test]
    async fn test_get_by_id_for_user_wrong_owner() {
        let note = create_test_scheduled_note("note1", "user1");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note.clone()]])
                .into_connection(),
        );

        let repo = ScheduledNoteRepository::new(db);
        let service = ScheduledNoteService::new(repo);

        let result = service.get_by_id_for_user("note1", "user2").await;

        assert!(result.is_err());
    }
}
