//! Moderation repository for abuse reports and suspensions.

use std::sync::Arc;

use crate::entities::{
    abuse_report::{self, ReportStatus},
    user_suspension, AbuseReport, UserSuspension,
};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// Moderation repository for database operations.
#[derive(Clone)]
pub struct ModerationRepository {
    db: Arc<DatabaseConnection>,
}

impl ModerationRepository {
    /// Create a new moderation repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ========== Abuse Reports ==========

    /// Create a new abuse report.
    pub async fn create_report(
        &self,
        model: abuse_report::ActiveModel,
    ) -> AppResult<abuse_report::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an abuse report by ID.
    pub async fn get_report(&self, id: &str) -> AppResult<abuse_report::Model> {
        AbuseReport::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Report {id} not found")))
    }

    /// Get pending abuse reports.
    pub async fn get_pending_reports(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<abuse_report::Model>> {
        AbuseReport::find()
            .filter(abuse_report::Column::Status.eq(ReportStatus::Pending))
            .order_by_desc(abuse_report::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get all abuse reports with optional status filter.
    pub async fn get_reports(
        &self,
        status: Option<ReportStatus>,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<abuse_report::Model>> {
        let mut query = AbuseReport::find().order_by_desc(abuse_report::Column::CreatedAt);

        if let Some(s) = status {
            query = query.filter(abuse_report::Column::Status.eq(s));
        }

        query
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an abuse report.
    pub async fn update_report(
        &self,
        model: abuse_report::ActiveModel,
    ) -> AppResult<abuse_report::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count pending reports.
    pub async fn count_pending_reports(&self) -> AppResult<u64> {
        AbuseReport::find()
            .filter(abuse_report::Column::Status.eq(ReportStatus::Pending))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get reports for a specific user.
    pub async fn get_reports_for_user(
        &self,
        user_id: &str,
        limit: u64,
    ) -> AppResult<Vec<abuse_report::Model>> {
        AbuseReport::find()
            .filter(abuse_report::Column::TargetUserId.eq(user_id))
            .order_by_desc(abuse_report::Column::CreatedAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    // ========== User Suspensions ==========

    /// Create a new user suspension.
    pub async fn create_suspension(
        &self,
        model: user_suspension::ActiveModel,
    ) -> AppResult<user_suspension::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a suspension by ID.
    pub async fn get_suspension(&self, id: &str) -> AppResult<user_suspension::Model> {
        UserSuspension::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Suspension {id} not found")))
    }

    /// Get active suspension for a user.
    pub async fn get_active_suspension(
        &self,
        user_id: &str,
    ) -> AppResult<Option<user_suspension::Model>> {
        let now = chrono::Utc::now();

        UserSuspension::find()
            .filter(user_suspension::Column::UserId.eq(user_id))
            .filter(user_suspension::Column::LiftedAt.is_null())
            .filter(
                user_suspension::Column::ExpiresAt
                    .is_null()
                    .or(user_suspension::Column::ExpiresAt.gt(now)),
            )
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if a user is currently suspended.
    pub async fn is_suspended(&self, user_id: &str) -> AppResult<bool> {
        Ok(self.get_active_suspension(user_id).await?.is_some())
    }

    /// Get all suspensions for a user.
    pub async fn get_user_suspensions(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<user_suspension::Model>> {
        UserSuspension::find()
            .filter(user_suspension::Column::UserId.eq(user_id))
            .order_by_desc(user_suspension::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a suspension.
    pub async fn update_suspension(
        &self,
        model: user_suspension::ActiveModel,
    ) -> AppResult<user_suspension::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get all active suspensions.
    pub async fn get_active_suspensions(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<user_suspension::Model>> {
        let now = chrono::Utc::now();

        UserSuspension::find()
            .filter(user_suspension::Column::LiftedAt.is_null())
            .filter(
                user_suspension::Column::ExpiresAt
                    .is_null()
                    .or(user_suspension::Column::ExpiresAt.gt(now)),
            )
            .order_by_desc(user_suspension::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_report(id: &str, reporter_id: &str, target_id: &str) -> abuse_report::Model {
        abuse_report::Model {
            id: id.to_string(),
            reporter_id: reporter_id.to_string(),
            target_user_id: target_id.to_string(),
            target_note_id: None,
            comment: "Test report".to_string(),
            status: ReportStatus::Pending,
            assignee_id: None,
            resolution_comment: None,
            created_at: Utc::now().into(),
            resolved_at: None,
        }
    }

    #[tokio::test]
    async fn test_get_pending_reports() {
        let report1 = create_test_report("report1", "user1", "user2");
        let report2 = create_test_report("report2", "user3", "user4");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[report1, report2]])
                .into_connection(),
        );

        let repo = ModerationRepository::new(db);
        let result = repo.get_pending_reports(10, 0).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_report() {
        let report = create_test_report("report1", "user1", "user2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[report.clone()]])
                .into_connection(),
        );

        let repo = ModerationRepository::new(db);
        let result = repo.get_report("report1").await.unwrap();

        assert_eq!(result.id, "report1");
    }
}
