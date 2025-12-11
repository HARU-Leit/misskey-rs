//! Push subscription repository.

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::entities::push_subscription::{self, ActiveModel, Column, Entity, Model};
use misskey_common::{AppError, AppResult};

/// Repository for push subscription operations.
#[derive(Clone)]
pub struct PushSubscriptionRepository {
    db: Arc<DatabaseConnection>,
}

impl PushSubscriptionRepository {
    /// Create a new push subscription repository.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a push subscription by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<Model>> {
        Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a push subscription by ID or return an error.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Push subscription {} not found", id)))
    }

    /// Find a push subscription by endpoint.
    pub async fn find_by_endpoint(&self, endpoint: &str) -> AppResult<Option<Model>> {
        Entity::find()
            .filter(Column::Endpoint.eq(endpoint))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all subscriptions for a user.
    pub async fn find_by_user_id(&self, user_id: &str) -> AppResult<Vec<Model>> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Active.eq(true))
            .order_by_desc(Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find active subscriptions that should receive a notification.
    pub async fn find_active_for_notification(
        &self,
        user_id: &str,
        notification_type: &str,
    ) -> AppResult<Vec<Model>> {
        // Get all active subscriptions for the user
        let subscriptions = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Active.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Filter by notification type
        let now = Utc::now();
        let current_hour = now.hour() as i32;

        let filtered: Vec<Model> = subscriptions
            .into_iter()
            .filter(|sub| {
                // Check if notification type is enabled
                if let Some(types) = sub.types.as_array() {
                    let type_enabled = types.iter().any(|t| {
                        t.as_str() == Some(notification_type) || t.as_str() == Some("all")
                    });
                    if !type_enabled {
                        return false;
                    }
                }

                // Check quiet hours
                if let (Some(start), Some(end)) = (sub.quiet_hours_start, sub.quiet_hours_end) {
                    if start <= end {
                        // Normal range (e.g., 22-7 wraps)
                        if current_hour >= start && current_hour < end {
                            return false;
                        }
                    } else {
                        // Wrapped range (e.g., 22-7)
                        if current_hour >= start || current_hour < end {
                            return false;
                        }
                    }
                }

                true
            })
            .collect();

        Ok(filtered)
    }

    /// Create a new push subscription.
    pub async fn create(&self, subscription: ActiveModel) -> AppResult<Model> {
        subscription
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a push subscription.
    pub async fn update(&self, subscription: ActiveModel) -> AppResult<Model> {
        subscription
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a push subscription.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        Entity::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Delete all subscriptions for a user.
    pub async fn delete_by_user(&self, user_id: &str) -> AppResult<u64> {
        let result = Entity::delete_many()
            .filter(Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result.rows_affected)
    }

    /// Increment fail count for a subscription.
    pub async fn increment_fail_count(&self, id: &str) -> AppResult<Model> {
        let subscription = self.get_by_id(id).await?;
        let mut active: ActiveModel = subscription.into();

        let current_fail_count = active.fail_count.clone().unwrap();
        active.fail_count = Set(current_fail_count + 1);
        active.updated_at = Set(Some(Utc::now().into()));

        // Deactivate if too many failures (e.g., 5)
        if current_fail_count + 1 >= 5 {
            active.active = Set(false);
        }

        self.update(active).await
    }

    /// Reset fail count and update last pushed timestamp.
    pub async fn mark_push_success(&self, id: &str) -> AppResult<Model> {
        let subscription = self.get_by_id(id).await?;
        let mut active: ActiveModel = subscription.into();

        active.fail_count = Set(0);
        active.last_pushed_at = Set(Some(Utc::now().into()));
        active.updated_at = Set(Some(Utc::now().into()));

        self.update(active).await
    }

    /// Deactivate a subscription.
    pub async fn deactivate(&self, id: &str) -> AppResult<Model> {
        let subscription = self.get_by_id(id).await?;
        let mut active: ActiveModel = subscription.into();

        active.active = Set(false);
        active.updated_at = Set(Some(Utc::now().into()));

        self.update(active).await
    }

    /// Reactivate a subscription.
    pub async fn reactivate(&self, id: &str) -> AppResult<Model> {
        let subscription = self.get_by_id(id).await?;
        let mut active: ActiveModel = subscription.into();

        active.active = Set(true);
        active.fail_count = Set(0);
        active.updated_at = Set(Some(Utc::now().into()));

        self.update(active).await
    }

    /// Count subscriptions for a user.
    pub async fn count_by_user(&self, user_id: &str) -> AppResult<u64> {
        use sea_orm::PaginatorTrait;

        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

use chrono::Timelike;
