//! Moderation service for handling abuse reports and user suspensions.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{abuse_report, user_suspension},
    repositories::{ModerationRepository, UserRepository},
};
use sea_orm::Set;

pub use misskey_db::entities::abuse_report::ReportStatus;

/// Input for creating an abuse report.
pub struct CreateReportInput {
    pub target_user_id: String,
    pub target_note_id: Option<String>,
    pub comment: String,
}

/// Input for resolving an abuse report.
pub struct ResolveReportInput {
    pub report_id: String,
    pub resolution: ReportStatus,
    pub comment: Option<String>,
}

/// Input for creating a suspension.
pub struct CreateSuspensionInput {
    pub user_id: String,
    pub reason: String,
    /// Duration in seconds, None for permanent.
    pub duration: Option<i64>,
}

/// Moderation service for handling reports and suspensions.
#[derive(Clone)]
pub struct ModerationService {
    moderation_repo: ModerationRepository,
    user_repo: UserRepository,
    id_gen: IdGenerator,
}

impl ModerationService {
    /// Create a new moderation service.
    #[must_use]
    pub const fn new(moderation_repo: ModerationRepository, user_repo: UserRepository) -> Self {
        Self {
            moderation_repo,
            user_repo,
            id_gen: IdGenerator::new(),
        }
    }

    // ========== Abuse Reports ==========

    /// Create a new abuse report.
    pub async fn create_report(
        &self,
        reporter_id: &str,
        input: CreateReportInput,
    ) -> AppResult<abuse_report::Model> {
        // Validate comment
        let comment = input.comment.trim();
        if comment.is_empty() {
            return Err(AppError::BadRequest(
                "Report comment is required".to_string(),
            ));
        }
        if comment.len() > 2000 {
            return Err(AppError::BadRequest("Report comment too long".to_string()));
        }

        // Can't report yourself
        if reporter_id == input.target_user_id {
            return Err(AppError::BadRequest("Cannot report yourself".to_string()));
        }

        // Check target user exists
        self.user_repo.get_by_id(&input.target_user_id).await?;

        let id = self.id_gen.generate();
        let model = abuse_report::ActiveModel {
            id: Set(id),
            reporter_id: Set(reporter_id.to_string()),
            target_user_id: Set(input.target_user_id),
            target_note_id: Set(input.target_note_id),
            comment: Set(comment.to_string()),
            status: Set(ReportStatus::Pending),
            assignee_id: Set(None),
            resolution_comment: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            resolved_at: Set(None),
        };

        self.moderation_repo.create_report(model).await
    }

    /// Get an abuse report by ID.
    pub async fn get_report(&self, id: &str) -> AppResult<abuse_report::Model> {
        self.moderation_repo.get_report(id).await
    }

    /// Get pending abuse reports.
    pub async fn get_pending_reports(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<abuse_report::Model>> {
        self.moderation_repo
            .get_pending_reports(limit, offset)
            .await
    }

    /// Get all abuse reports.
    pub async fn get_reports(
        &self,
        status: Option<ReportStatus>,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<abuse_report::Model>> {
        self.moderation_repo
            .get_reports(status, limit, offset)
            .await
    }

    /// Resolve an abuse report.
    pub async fn resolve_report(
        &self,
        moderator_id: &str,
        input: ResolveReportInput,
    ) -> AppResult<abuse_report::Model> {
        // Verify moderator is admin/moderator
        let moderator = self.user_repo.get_by_id(moderator_id).await?;
        if !moderator.is_admin && !moderator.is_moderator {
            return Err(AppError::Forbidden(
                "Only moderators can resolve reports".to_string(),
            ));
        }

        // Can't set to pending
        if input.resolution == ReportStatus::Pending {
            return Err(AppError::BadRequest(
                "Cannot set report back to pending".to_string(),
            ));
        }

        let report = self.moderation_repo.get_report(&input.report_id).await?;

        // Check if already resolved
        if report.status != ReportStatus::Pending {
            return Err(AppError::BadRequest("Report already resolved".to_string()));
        }

        let mut model: abuse_report::ActiveModel = report.into();
        model.status = Set(input.resolution);
        model.assignee_id = Set(Some(moderator_id.to_string()));
        model.resolution_comment = Set(input.comment);
        model.resolved_at = Set(Some(chrono::Utc::now().into()));

        self.moderation_repo.update_report(model).await
    }

    /// Count pending reports.
    pub async fn count_pending_reports(&self) -> AppResult<u64> {
        self.moderation_repo.count_pending_reports().await
    }

    /// Get reports for a specific user.
    pub async fn get_reports_for_user(
        &self,
        user_id: &str,
        limit: u64,
    ) -> AppResult<Vec<abuse_report::Model>> {
        self.moderation_repo
            .get_reports_for_user(user_id, limit)
            .await
    }

    // ========== User Suspensions ==========

    /// Suspend a user.
    pub async fn suspend_user(
        &self,
        moderator_id: &str,
        input: CreateSuspensionInput,
    ) -> AppResult<user_suspension::Model> {
        // Verify moderator is admin/moderator
        let moderator = self.user_repo.get_by_id(moderator_id).await?;
        if !moderator.is_admin && !moderator.is_moderator {
            return Err(AppError::Forbidden(
                "Only moderators can suspend users".to_string(),
            ));
        }

        // Can't suspend yourself
        if moderator_id == input.user_id {
            return Err(AppError::BadRequest("Cannot suspend yourself".to_string()));
        }

        // Check target user exists
        let target = self.user_repo.get_by_id(&input.user_id).await?;

        // Can't suspend admins
        if target.is_admin {
            return Err(AppError::Forbidden("Cannot suspend an admin".to_string()));
        }

        // Check if already suspended
        if self.moderation_repo.is_suspended(&input.user_id).await? {
            return Err(AppError::BadRequest("User already suspended".to_string()));
        }

        // Validate reason
        let reason = input.reason.trim();
        if reason.is_empty() {
            return Err(AppError::BadRequest(
                "Suspension reason is required".to_string(),
            ));
        }

        let expires_at = input
            .duration
            .map(|d| chrono::Utc::now() + chrono::Duration::seconds(d));

        let id = self.id_gen.generate();
        let model = user_suspension::ActiveModel {
            id: Set(id),
            user_id: Set(input.user_id),
            moderator_id: Set(moderator_id.to_string()),
            reason: Set(reason.to_string()),
            created_at: Set(chrono::Utc::now().into()),
            expires_at: Set(expires_at.map(std::convert::Into::into)),
            lifted_at: Set(None),
            lifted_by: Set(None),
        };

        self.moderation_repo.create_suspension(model).await
    }

    /// Lift a user suspension.
    pub async fn unsuspend_user(
        &self,
        moderator_id: &str,
        user_id: &str,
    ) -> AppResult<user_suspension::Model> {
        // Verify moderator is admin/moderator
        let moderator = self.user_repo.get_by_id(moderator_id).await?;
        if !moderator.is_admin && !moderator.is_moderator {
            return Err(AppError::Forbidden(
                "Only moderators can unsuspend users".to_string(),
            ));
        }

        // Get active suspension
        let suspension = self
            .moderation_repo
            .get_active_suspension(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User is not suspended".to_string()))?;

        let mut model: user_suspension::ActiveModel = suspension.into();
        model.lifted_at = Set(Some(chrono::Utc::now().into()));
        model.lifted_by = Set(Some(moderator_id.to_string()));

        self.moderation_repo.update_suspension(model).await
    }

    /// Check if a user is suspended.
    pub async fn is_suspended(&self, user_id: &str) -> AppResult<bool> {
        self.moderation_repo.is_suspended(user_id).await
    }

    /// Get active suspension for a user.
    pub async fn get_active_suspension(
        &self,
        user_id: &str,
    ) -> AppResult<Option<user_suspension::Model>> {
        self.moderation_repo.get_active_suspension(user_id).await
    }

    /// Get suspension history for a user.
    pub async fn get_user_suspensions(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<user_suspension::Model>> {
        self.moderation_repo.get_user_suspensions(user_id).await
    }

    /// Get all active suspensions.
    pub async fn get_active_suspensions(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<user_suspension::Model>> {
        self.moderation_repo
            .get_active_suspensions(limit, offset)
            .await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_create_report_input() {
        let input = CreateReportInput {
            target_user_id: "user1".to_string(),
            target_note_id: Some("note1".to_string()),
            comment: "Spam content".to_string(),
        };
        assert_eq!(input.target_user_id, "user1");
        assert!(input.target_note_id.is_some());
    }

    #[test]
    fn test_create_suspension_input() {
        let input = CreateSuspensionInput {
            user_id: "user1".to_string(),
            reason: "Repeated violations".to_string(),
            duration: Some(86400), // 1 day
        };
        assert_eq!(input.user_id, "user1");
        assert_eq!(input.duration, Some(86400));
    }
}
