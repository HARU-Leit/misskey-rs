//! Webhook repository.

use std::sync::Arc;

use crate::entities::{webhook, Webhook};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

/// Maximum number of webhooks per user.
pub const MAX_WEBHOOKS_PER_USER: usize = 10;

/// Webhook repository for database operations.
#[derive(Clone)]
pub struct WebhookRepository {
    db: Arc<DatabaseConnection>,
}

impl WebhookRepository {
    /// Create a new webhook repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a webhook by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<webhook::Model>> {
        Webhook::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a webhook by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<webhook::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Webhook: {id}")))
    }

    /// Find all webhooks for a user.
    pub async fn find_by_user_id(&self, user_id: &str) -> AppResult<Vec<webhook::Model>> {
        Webhook::find()
            .filter(webhook::Column::UserId.eq(user_id))
            .order_by_desc(webhook::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all active webhooks for a user that subscribe to a specific event.
    pub async fn find_active_by_user_and_event(
        &self,
        user_id: &str,
        event: &str,
    ) -> AppResult<Vec<webhook::Model>> {
        // Get all active webhooks for the user, we'll filter events in code
        // since JSON array queries are complex
        let webhooks = Webhook::find()
            .filter(webhook::Column::UserId.eq(user_id))
            .filter(webhook::Column::IsActive.eq(true))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Filter by event subscription
        Ok(webhooks
            .into_iter()
            .filter(|w| {
                let events: Vec<String> = serde_json::from_value(w.events.clone()).unwrap_or_default();
                events.contains(&event.to_string())
            })
            .collect())
    }

    /// Count webhooks for a user.
    pub async fn count_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        Webhook::find()
            .filter(webhook::Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new webhook.
    pub async fn create(&self, model: webhook::ActiveModel) -> AppResult<webhook::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a webhook.
    pub async fn update(&self, model: webhook::ActiveModel) -> AppResult<webhook::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a webhook.
    pub async fn delete(&self, id: &str, user_id: &str) -> AppResult<()> {
        let webhook = self.get_by_id(id).await?;

        // Verify ownership
        if webhook.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only delete your own webhooks".to_string(),
            ));
        }

        Webhook::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Record a successful webhook delivery.
    pub async fn record_success(&self, id: &str) -> AppResult<()> {
        let webhook = self.get_by_id(id).await?;
        let mut active: webhook::ActiveModel = webhook.into();

        active.last_triggered_at = Set(Some(chrono::Utc::now().into()));
        active.failure_count = Set(0);
        active.last_error = Set(None);

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Record a failed webhook delivery.
    pub async fn record_failure(&self, id: &str, error: &str) -> AppResult<()> {
        let webhook = self.get_by_id(id).await?;
        let failure_count = webhook.failure_count + 1;

        let mut active: webhook::ActiveModel = webhook.into();
        active.last_triggered_at = Set(Some(chrono::Utc::now().into()));
        active.failure_count = Set(failure_count);
        active.last_error = Set(Some(error.to_string()));

        // Disable webhook after too many failures
        if failure_count >= 5 {
            active.is_active = Set(false);
        }

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Check if a user has reached the maximum number of webhooks.
    pub async fn user_at_limit(&self, user_id: &str) -> AppResult<bool> {
        let count = self.count_by_user_id(user_id).await?;
        Ok(count as usize >= MAX_WEBHOOKS_PER_USER)
    }
}
