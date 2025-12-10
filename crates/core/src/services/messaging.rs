//! Messaging service for direct messages.

use crate::services::event_publisher::EventPublisherService;
use chrono::Utc;
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::messaging_message,
    repositories::{BlockingRepository, MessagingRepository, UserRepository},
};
use sea_orm::Set;

/// Input for creating a new message.
pub struct CreateMessageInput {
    pub text: Option<String>,
    pub file_id: Option<String>,
}

/// Conversation summary for listing.
pub struct ConversationSummary {
    pub partner_id: String,
    pub partner_username: String,
    pub partner_avatar_url: Option<String>,
    pub last_message: Option<messaging_message::Model>,
    pub unread_count: u64,
}

/// Messaging service.
#[derive(Clone)]
pub struct MessagingService {
    messaging_repo: MessagingRepository,
    user_repo: UserRepository,
    blocking_repo: BlockingRepository,
    event_publisher: Option<EventPublisherService>,
    id_gen: IdGenerator,
}

impl MessagingService {
    /// Create a new messaging service.
    #[must_use]
    pub const fn new(
        messaging_repo: MessagingRepository,
        user_repo: UserRepository,
        blocking_repo: BlockingRepository,
    ) -> Self {
        Self {
            messaging_repo,
            user_repo,
            blocking_repo,
            event_publisher: None,
            id_gen: IdGenerator::new(),
        }
    }

    /// Set the event publisher.
    pub fn set_event_publisher(&mut self, event_publisher: EventPublisherService) {
        self.event_publisher = Some(event_publisher);
    }

    /// Send a message to another user.
    pub async fn send_message(
        &self,
        sender_id: &str,
        recipient_id: &str,
        input: CreateMessageInput,
    ) -> AppResult<messaging_message::Model> {
        // Validate that at least text or file is provided
        if input.text.is_none() && input.file_id.is_none() {
            return Err(AppError::BadRequest(
                "Message must have text or file".to_string(),
            ));
        }

        // Check that recipient exists
        let _recipient = self
            .user_repo
            .find_by_id(recipient_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {recipient_id}")))?;

        // Check that sender is not trying to message themselves
        if sender_id == recipient_id {
            return Err(AppError::BadRequest(
                "Cannot send message to yourself".to_string(),
            ));
        }

        // Check if either user has blocked the other
        if self
            .blocking_repo
            .is_blocked_between(sender_id, recipient_id)
            .await?
        {
            return Err(AppError::Forbidden(
                "Cannot send message due to block relationship".to_string(),
            ));
        }

        // TODO: Check if recipient allows messages from non-followers

        let message_id = self.id_gen.generate();

        let model = messaging_message::ActiveModel {
            id: Set(message_id),
            user_id: Set(sender_id.to_string()),
            recipient_id: Set(Some(recipient_id.to_string())),
            group_id: Set(None),
            text: Set(input.text),
            file_id: Set(input.file_id),
            is_read: Set(false),
            uri: Set(None),
            created_at: Set(Utc::now().into()),
        };

        let message = self.messaging_repo.create(model).await?;

        // TODO: Create notification for recipient

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher {
            if let Err(e) = event_publisher
                .publish_direct_message(&message.id, sender_id, recipient_id, message.text.as_deref())
                .await
            {
                tracing::warn!(error = %e, "Failed to publish direct message event");
            }
        }

        Ok(message)
    }

    /// Get messages in a conversation with another user.
    pub async fn get_conversation(
        &self,
        user_id: &str,
        partner_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<messaging_message::Model>> {
        self.messaging_repo
            .find_conversation(user_id, partner_id, limit, until_id)
            .await
    }

    /// Get list of conversations (users the current user has messaged or received messages from).
    pub async fn get_conversations(
        &self,
        user_id: &str,
        limit: u64,
    ) -> AppResult<Vec<ConversationSummary>> {
        let partner_ids = self
            .messaging_repo
            .find_conversation_partners(user_id, limit)
            .await?;

        let mut summaries = Vec::new();

        for partner_id in partner_ids {
            if let Some(partner) = self.user_repo.find_by_id(&partner_id).await? {
                let last_message = self
                    .messaging_repo
                    .find_latest_in_conversation(user_id, &partner_id)
                    .await?;

                let unread_count = self
                    .messaging_repo
                    .count_unread_from(user_id, &partner_id)
                    .await?;

                summaries.push(ConversationSummary {
                    partner_id: partner.id,
                    partner_username: partner.username,
                    partner_avatar_url: partner.avatar_url,
                    last_message,
                    unread_count,
                });
            }
        }

        Ok(summaries)
    }

    /// Get a message by ID.
    pub async fn get_message(
        &self,
        message_id: &str,
    ) -> AppResult<Option<messaging_message::Model>> {
        self.messaging_repo.find_by_id(message_id).await
    }

    /// Mark messages from a user as read.
    pub async fn mark_as_read(&self, user_id: &str, partner_id: &str) -> AppResult<u64> {
        self.messaging_repo.mark_as_read(user_id, partner_id).await
    }

    /// Get unread message count for a user.
    pub async fn get_unread_count(&self, user_id: &str) -> AppResult<u64> {
        self.messaging_repo.count_unread(user_id).await
    }

    /// Delete a message.
    pub async fn delete_message(&self, user_id: &str, message_id: &str) -> AppResult<()> {
        let message = self
            .messaging_repo
            .find_by_id(message_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Message not found: {message_id}")))?;

        // Only the sender can delete a message
        if message.user_id != user_id {
            return Err(AppError::Forbidden(
                "Cannot delete another user's message".to_string(),
            ));
        }

        self.messaging_repo.delete(message_id).await
    }
}
