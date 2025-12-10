//! Messaging message repository.

use crate::entities::messaging_message::{self, ActiveModel, Column, Entity as MessagingMessage};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use std::sync::Arc;

/// Repository for messaging message operations.
#[derive(Clone)]
pub struct MessagingRepository {
    db: Arc<DatabaseConnection>,
}

impl MessagingRepository {
    /// Create a new messaging repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Create a new message.
    pub async fn create(&self, model: ActiveModel) -> AppResult<messaging_message::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a message by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<messaging_message::Model>> {
        MessagingMessage::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find messages in a conversation between two users.
    pub async fn find_conversation(
        &self,
        user_id: &str,
        partner_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<messaging_message::Model>> {
        let mut query = MessagingMessage::find()
            .filter(
                // Messages sent by user to partner OR messages sent by partner to user
                sea_orm::Condition::any()
                    .add(
                        sea_orm::Condition::all()
                            .add(Column::UserId.eq(user_id))
                            .add(Column::RecipientId.eq(partner_id)),
                    )
                    .add(
                        sea_orm::Condition::all()
                            .add(Column::UserId.eq(partner_id))
                            .add(Column::RecipientId.eq(user_id)),
                    ),
            )
            .order_by_desc(Column::CreatedAt);

        if let Some(until) = until_id {
            // Get messages older than the specified ID
            if let Some(until_msg) = self.find_by_id(until).await? {
                query = query.filter(Column::CreatedAt.lt(until_msg.created_at));
            }
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get conversation partners for a user (users they've messaged or been messaged by).
    pub async fn find_conversation_partners(
        &self,
        user_id: &str,
        limit: u64,
    ) -> AppResult<Vec<String>> {
        use sea_orm::{ConnectionTrait, Statement};

        // Get unique partner IDs from both sent and received messages
        let sql = format!(
            r"
            SELECT DISTINCT partner_id FROM (
                SELECT recipient_id AS partner_id FROM messaging_message
                WHERE user_id = $1 AND recipient_id IS NOT NULL
                UNION
                SELECT user_id AS partner_id FROM messaging_message
                WHERE recipient_id = $1
            ) AS partners
            ORDER BY partner_id
            LIMIT {limit}
            "
        );

        let result = self
            .db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                &sql,
                [user_id.into()],
            ))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut partners = Vec::new();
        for row in result {
            if let Ok(partner_id) = row.try_get::<String>("", "partner_id") {
                partners.push(partner_id);
            }
        }

        Ok(partners)
    }

    /// Get unread message count for a user.
    pub async fn count_unread(&self, user_id: &str) -> AppResult<u64> {
        MessagingMessage::find()
            .filter(Column::RecipientId.eq(user_id))
            .filter(Column::IsRead.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get unread message count from a specific user.
    pub async fn count_unread_from(&self, user_id: &str, partner_id: &str) -> AppResult<u64> {
        MessagingMessage::find()
            .filter(Column::UserId.eq(partner_id))
            .filter(Column::RecipientId.eq(user_id))
            .filter(Column::IsRead.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Mark messages as read.
    pub async fn mark_as_read(&self, user_id: &str, partner_id: &str) -> AppResult<u64> {
        use sea_orm::sea_query::Expr;

        let result = MessagingMessage::update_many()
            .col_expr(Column::IsRead, Expr::value(true))
            .filter(Column::UserId.eq(partner_id))
            .filter(Column::RecipientId.eq(user_id))
            .filter(Column::IsRead.eq(false))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }

    /// Delete a message by ID.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        MessagingMessage::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Find messages by user (sent or received).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<messaging_message::Model>> {
        let mut query = MessagingMessage::find()
            .filter(
                sea_orm::Condition::any()
                    .add(Column::UserId.eq(user_id))
                    .add(Column::RecipientId.eq(user_id)),
            )
            .order_by_desc(Column::CreatedAt);

        if let Some(until) = until_id
            && let Some(until_msg) = self.find_by_id(until).await? {
                query = query.filter(Column::CreatedAt.lt(until_msg.created_at));
            }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find the latest message in a conversation.
    pub async fn find_latest_in_conversation(
        &self,
        user_id: &str,
        partner_id: &str,
    ) -> AppResult<Option<messaging_message::Model>> {
        MessagingMessage::find()
            .filter(
                sea_orm::Condition::any()
                    .add(
                        sea_orm::Condition::all()
                            .add(Column::UserId.eq(user_id))
                            .add(Column::RecipientId.eq(partner_id)),
                    )
                    .add(
                        sea_orm::Condition::all()
                            .add(Column::UserId.eq(partner_id))
                            .add(Column::RecipientId.eq(user_id)),
                    ),
            )
            .order_by_desc(Column::CreatedAt)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
