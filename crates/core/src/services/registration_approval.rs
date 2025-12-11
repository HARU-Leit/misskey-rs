//! Registration approval service for manual account approval workflow.

use misskey_common::{AppError, AppResult};
use misskey_db::entities::{registration_approval, registration_approval::ApprovalStatus};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set};
use std::sync::Arc;

/// Registration approval service for managing account approvals.
#[derive(Clone)]
pub struct RegistrationApprovalService {
    db: Arc<DatabaseConnection>,
}

impl RegistrationApprovalService {
    /// Create a new registration approval service.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Create a new registration approval request.
    pub async fn create(
        &self,
        user_id: &str,
        reason: Option<&str>,
    ) -> AppResult<registration_approval::Model> {
        let now = chrono::Utc::now();
        let id = crate::generate_id();

        let model = registration_approval::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            reason: Set(reason.map(String::from)),
            status: Set(ApprovalStatus::Pending),
            reviewed_by: Set(None),
            review_note: Set(None),
            created_at: Set(now.into()),
            reviewed_at: Set(None),
        };

        let result = model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result)
    }

    /// List registration approvals with optional status filter.
    pub async fn list(
        &self,
        status: Option<ApprovalStatus>,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<registration_approval::Model>> {
        let mut query = registration_approval::Entity::find()
            .order_by_desc(registration_approval::Column::CreatedAt);

        if let Some(s) = status {
            query = query.filter(registration_approval::Column::Status.eq(s));
        }

        let approvals = query
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(approvals)
    }

    /// Get a registration approval by user ID.
    pub async fn get_by_user_id(&self, user_id: &str) -> AppResult<registration_approval::Model> {
        let approval = registration_approval::Entity::find()
            .filter(registration_approval::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Registration approval not found".to_string()))?;

        Ok(approval)
    }

    /// Approve a registration request.
    pub async fn approve(
        &self,
        reviewer_id: &str,
        user_id: &str,
        note: Option<&str>,
    ) -> AppResult<registration_approval::Model> {
        let approval = self.get_by_user_id(user_id).await?;

        if approval.status != ApprovalStatus::Pending {
            return Err(AppError::BadRequest(
                "Registration already reviewed".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        let mut model: registration_approval::ActiveModel = approval.into();
        model.status = Set(ApprovalStatus::Approved);
        model.reviewed_by = Set(Some(reviewer_id.to_string()));
        model.review_note = Set(note.map(String::from));
        model.reviewed_at = Set(Some(now.into()));

        let result = model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Reject a registration request.
    pub async fn reject(
        &self,
        reviewer_id: &str,
        user_id: &str,
        note: Option<&str>,
    ) -> AppResult<registration_approval::Model> {
        let approval = self.get_by_user_id(user_id).await?;

        if approval.status != ApprovalStatus::Pending {
            return Err(AppError::BadRequest(
                "Registration already reviewed".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        let mut model: registration_approval::ActiveModel = approval.into();
        model.status = Set(ApprovalStatus::Rejected);
        model.reviewed_by = Set(Some(reviewer_id.to_string()));
        model.review_note = Set(note.map(String::from));
        model.reviewed_at = Set(Some(now.into()));

        let result = model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Check if a user has a pending approval.
    pub async fn is_pending(&self, user_id: &str) -> AppResult<bool> {
        let approval = registration_approval::Entity::find()
            .filter(registration_approval::Column::UserId.eq(user_id))
            .filter(registration_approval::Column::Status.eq(ApprovalStatus::Pending))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(approval.is_some())
    }

    /// Count pending approvals.
    pub async fn count_pending(&self) -> AppResult<u64> {
        let count = registration_approval::Entity::find()
            .filter(registration_approval::Column::Status.eq(ApprovalStatus::Pending))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count)
    }
}
