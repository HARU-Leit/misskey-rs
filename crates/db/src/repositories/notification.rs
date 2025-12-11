//! Notification repository.

use std::sync::Arc;

use crate::entities::{Notification, notification};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

/// Notification repository for database operations.
#[derive(Clone)]
pub struct NotificationRepository {
    db: Arc<DatabaseConnection>,
}

impl NotificationRepository {
    /// Create a new notification repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a notification by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<notification::Model>> {
        Notification::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new notification.
    pub async fn create(&self, model: notification::ActiveModel) -> AppResult<notification::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a notification.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let notification = self.find_by_id(id).await?;
        if let Some(n) = notification {
            n.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get notifications for a user (paginated).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
        unread_only: bool,
    ) -> AppResult<Vec<notification::Model>> {
        let mut query = Notification::find()
            .filter(notification::Column::NotifieeId.eq(user_id))
            .order_by_desc(notification::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(notification::Column::Id.lt(id));
        }

        if unread_only {
            query = query.filter(notification::Column::IsRead.eq(false));
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Mark a notification as read.
    pub async fn mark_as_read(&self, id: &str) -> AppResult<()> {
        let notification = self.find_by_id(id).await?;
        if let Some(n) = notification {
            let mut active: notification::ActiveModel = n.into();
            active.is_read = Set(true);
            active
                .update(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Mark all notifications as read for a user.
    pub async fn mark_all_as_read(&self, user_id: &str) -> AppResult<u64> {
        use sea_orm::UpdateResult;

        let result: UpdateResult = Notification::update_many()
            .filter(notification::Column::NotifieeId.eq(user_id))
            .filter(notification::Column::IsRead.eq(false))
            .col_expr(notification::Column::IsRead, true.into())
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }

    /// Count unread notifications for a user.
    pub async fn count_unread(&self, user_id: &str) -> AppResult<u64> {
        Notification::find()
            .filter(notification::Column::NotifieeId.eq(user_id))
            .filter(notification::Column::IsRead.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete all notifications for a user.
    pub async fn delete_all_for_user(&self, user_id: &str) -> AppResult<u64> {
        use sea_orm::DeleteResult;

        let result: DeleteResult = Notification::delete_many()
            .filter(notification::Column::NotifieeId.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
