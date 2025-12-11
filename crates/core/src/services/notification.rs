//! Notification service.

use crate::services::event_publisher::EventPublisherService;
use crate::services::jobs::JobSender;
use crate::services::push_notification::{PushNotificationType, PushPayload};
use misskey_common::{AppResult, IdGenerator};
use misskey_db::{
    entities::notification::{self, NotificationType},
    repositories::NotificationRepository,
};
use sea_orm::Set;

/// Notification service for business logic.
#[derive(Clone)]
pub struct NotificationService {
    notification_repo: NotificationRepository,
    event_publisher: Option<EventPublisherService>,
    job_sender: Option<JobSender>,
    id_gen: IdGenerator,
}

impl NotificationService {
    /// Create a new notification service.
    #[must_use]
    pub const fn new(notification_repo: NotificationRepository) -> Self {
        Self {
            notification_repo,
            event_publisher: None,
            job_sender: None,
            id_gen: IdGenerator::new(),
        }
    }

    /// Set the event publisher.
    pub fn set_event_publisher(&mut self, event_publisher: EventPublisherService) {
        self.event_publisher = Some(event_publisher);
    }

    /// Set the job sender for push notifications.
    pub fn set_job_sender(&mut self, job_sender: JobSender) {
        self.job_sender = Some(job_sender);
    }

    /// Create a follow notification.
    pub async fn create_follow_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
    ) -> AppResult<notification::Model> {
        // Don't notify yourself
        if notifiee_id == notifier_id {
            return self.create_internal(
                notifiee_id,
                Some(notifier_id),
                NotificationType::Follow,
                None,
                None,
            ).await;
        }

        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::Follow,
            None,
            None,
        )
        .await
    }

    /// Create a mention notification.
    pub async fn create_mention_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
        note_id: &str,
    ) -> AppResult<notification::Model> {
        // Don't notify yourself
        if notifiee_id == notifier_id {
            return self.create_internal(
                notifiee_id,
                Some(notifier_id),
                NotificationType::Mention,
                Some(note_id),
                None,
            ).await;
        }

        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::Mention,
            Some(note_id),
            None,
        )
        .await
    }

    /// Create a reply notification.
    pub async fn create_reply_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
        note_id: &str,
    ) -> AppResult<notification::Model> {
        // Don't notify yourself
        if notifiee_id == notifier_id {
            return self.create_internal(
                notifiee_id,
                Some(notifier_id),
                NotificationType::Reply,
                Some(note_id),
                None,
            ).await;
        }

        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::Reply,
            Some(note_id),
            None,
        )
        .await
    }

    /// Create a renote notification.
    pub async fn create_renote_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
        note_id: &str,
    ) -> AppResult<notification::Model> {
        // Don't notify yourself
        if notifiee_id == notifier_id {
            return self.create_internal(
                notifiee_id,
                Some(notifier_id),
                NotificationType::Renote,
                Some(note_id),
                None,
            ).await;
        }

        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::Renote,
            Some(note_id),
            None,
        )
        .await
    }

    /// Create a quote notification.
    pub async fn create_quote_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
        note_id: &str,
    ) -> AppResult<notification::Model> {
        // Don't notify yourself
        if notifiee_id == notifier_id {
            return self.create_internal(
                notifiee_id,
                Some(notifier_id),
                NotificationType::Quote,
                Some(note_id),
                None,
            ).await;
        }

        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::Quote,
            Some(note_id),
            None,
        )
        .await
    }

    /// Create a reaction notification.
    pub async fn create_reaction_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
        note_id: &str,
        reaction: &str,
    ) -> AppResult<notification::Model> {
        // Don't notify yourself
        if notifiee_id == notifier_id {
            return self.create_internal(
                notifiee_id,
                Some(notifier_id),
                NotificationType::Reaction,
                Some(note_id),
                Some(reaction),
            ).await;
        }

        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::Reaction,
            Some(note_id),
            Some(reaction),
        )
        .await
    }

    /// Create a follow request notification.
    pub async fn create_follow_request_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
    ) -> AppResult<notification::Model> {
        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::ReceiveFollowRequest,
            None,
            None,
        )
        .await
    }

    /// Create a follow request accepted notification.
    pub async fn create_follow_request_accepted_notification(
        &self,
        notifiee_id: &str,
        notifier_id: &str,
    ) -> AppResult<notification::Model> {
        self.create_internal(
            notifiee_id,
            Some(notifier_id),
            NotificationType::FollowRequestAccepted,
            None,
            None,
        )
        .await
    }

    /// Internal helper to create notifications.
    async fn create_internal(
        &self,
        notifiee_id: &str,
        notifier_id: Option<&str>,
        notification_type: NotificationType,
        note_id: Option<&str>,
        reaction: Option<&str>,
    ) -> AppResult<notification::Model> {
        let notification_id = self.id_gen.generate();
        let model = notification::ActiveModel {
            id: Set(notification_id.clone()),
            notifiee_id: Set(notifiee_id.to_string()),
            notifier_id: Set(notifier_id.map(std::string::ToString::to_string)),
            notification_type: Set(notification_type.clone()),
            note_id: Set(note_id.map(std::string::ToString::to_string)),
            follow_request_id: Set(None),
            reaction: Set(reaction.map(std::string::ToString::to_string)),
            custom_data: Set(None),
            is_read: Set(false),
            created_at: Set(chrono::Utc::now().into()),
        };

        let notification = self.notification_repo.create(model).await?;

        // Publish real-time event
        let type_str = match notification_type.clone() {
            NotificationType::Follow => "follow",
            NotificationType::Mention => "mention",
            NotificationType::Reply => "reply",
            NotificationType::Renote => "renote",
            NotificationType::Quote => "quote",
            NotificationType::Reaction => "reaction",
            NotificationType::PollEnded => "pollEnded",
            NotificationType::ReceiveFollowRequest => "receiveFollowRequest",
            NotificationType::FollowRequestAccepted => "followRequestAccepted",
            NotificationType::App => "app",
        };

        if let Some(ref event_publisher) = self.event_publisher {
            if let Err(e) = event_publisher
                .publish_notification(&notification_id, notifiee_id, type_str, notifier_id, note_id)
                .await
            {
                tracing::warn!(error = %e, "Failed to publish notification event");
            }
        }

        // Enqueue push notification job
        if let Some(ref job_sender) = self.job_sender {
            let push_type = match notification_type {
                NotificationType::Follow => PushNotificationType::Follow,
                NotificationType::Mention => PushNotificationType::Mention,
                NotificationType::Reply => PushNotificationType::Reply,
                NotificationType::Renote => PushNotificationType::Renote,
                NotificationType::Quote => PushNotificationType::Quote,
                NotificationType::Reaction => PushNotificationType::Reaction,
                NotificationType::PollEnded => PushNotificationType::PollEnded,
                NotificationType::ReceiveFollowRequest => PushNotificationType::FollowRequestReceived,
                NotificationType::FollowRequestAccepted => PushNotificationType::FollowRequestAccepted,
                NotificationType::App => PushNotificationType::App,
            };

            let payload = PushPayload {
                notification_type: type_str.to_string(),
                title: format!("New {}", type_str),
                body: format!("You have a new {} notification", type_str),
                icon: None,
                url: None,
                data: None,
            };

            if let Err(e) = job_sender
                .push_notification(notifiee_id.to_string(), push_type, payload)
                .await
            {
                tracing::warn!(error = %e, "Failed to enqueue push notification job");
            }
        }

        Ok(notification)
    }

    /// Get notifications for a user.
    pub async fn get_notifications(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
        unread_only: bool,
    ) -> AppResult<Vec<notification::Model>> {
        self.notification_repo
            .find_by_user(user_id, limit, until_id, unread_only)
            .await
    }

    /// Mark a notification as read.
    pub async fn mark_as_read(&self, user_id: &str, notification_id: &str) -> AppResult<()> {
        // Verify the notification belongs to the user
        let notification = self.notification_repo.find_by_id(notification_id).await?;
        if let Some(n) = notification
            && n.notifiee_id == user_id {
                self.notification_repo.mark_as_read(notification_id).await?;
            }
        Ok(())
    }

    /// Mark all notifications as read for a user.
    pub async fn mark_all_as_read(&self, user_id: &str) -> AppResult<u64> {
        self.notification_repo.mark_all_as_read(user_id).await
    }

    /// Count unread notifications for a user.
    pub async fn count_unread(&self, user_id: &str) -> AppResult<u64> {
        self.notification_repo.count_unread(user_id).await
    }

    /// Delete a notification.
    pub async fn delete(&self, user_id: &str, notification_id: &str) -> AppResult<()> {
        // Verify the notification belongs to the user
        let notification = self.notification_repo.find_by_id(notification_id).await?;
        if let Some(n) = notification
            && n.notifiee_id == user_id {
                self.notification_repo.delete(notification_id).await?;
            }
        Ok(())
    }

    /// Delete all notifications for a user.
    pub async fn delete_all(&self, user_id: &str) -> AppResult<u64> {
        self.notification_repo.delete_all_for_user(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_enum() {
        // Verify notification types exist
        let _ = NotificationType::Follow;
        let _ = NotificationType::Mention;
        let _ = NotificationType::Reply;
        let _ = NotificationType::Renote;
        let _ = NotificationType::Quote;
        let _ = NotificationType::Reaction;
        let _ = NotificationType::PollEnded;
        let _ = NotificationType::ReceiveFollowRequest;
        let _ = NotificationType::FollowRequestAccepted;
        let _ = NotificationType::App;
    }
}
