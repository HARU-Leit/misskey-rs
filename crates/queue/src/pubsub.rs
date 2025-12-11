//! Redis Pub/Sub for cross-instance event distribution.
//!
//! Enables real-time event synchronization across multiple server instances
//! using Redis Pub/Sub channels.

#![allow(missing_docs)]

use std::sync::Arc;

use async_trait::async_trait;
use fred::clients::{Client, SubscriberClient};
use fred::error::{Error as RedisError, ErrorKind as RedisErrorKind};
use fred::interfaces::{ClientLike, EventInterface, PubsubInterface};
use fred::types::config::Config as RedisConfig;
use misskey_common::AppResult;
use misskey_core::services::EventPublisher;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Pub/Sub channel names.
pub mod channels {
    /// Global timeline events.
    pub const GLOBAL_TIMELINE: &str = "misskey:timeline:global";
    /// Local timeline events.
    pub const LOCAL_TIMELINE: &str = "misskey:timeline:local";
    /// User-specific events (suffix with user ID).
    pub const USER_PREFIX: &str = "misskey:user:";
    /// Note events (create, delete, update).
    pub const NOTES: &str = "misskey:notes";
    /// Notification events.
    pub const NOTIFICATIONS: &str = "misskey:notifications";
    /// Follow/unfollow events.
    pub const FOLLOWS: &str = "misskey:follows";
    /// Reaction events.
    pub const REACTIONS: &str = "misskey:reactions";
    /// Direct message events.
    pub const MESSAGING: &str = "misskey:messaging";
}

/// Pub/Sub event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PubSubEvent {
    /// New note created.
    NoteCreated {
        id: String,
        user_id: String,
        text: Option<String>,
        visibility: String,
    },
    /// Note deleted.
    NoteDeleted { id: String, user_id: String },
    /// Note updated.
    NoteUpdated { id: String },
    /// New notification.
    Notification {
        id: String,
        user_id: String,
        notification_type: String,
        source_user_id: Option<String>,
        note_id: Option<String>,
    },
    /// User followed.
    Followed {
        follower_id: String,
        followee_id: String,
    },
    /// User unfollowed.
    Unfollowed {
        follower_id: String,
        followee_id: String,
    },
    /// Reaction added.
    ReactionAdded {
        note_id: String,
        user_id: String,
        reaction: String,
    },
    /// Reaction removed.
    ReactionRemoved {
        note_id: String,
        user_id: String,
        reaction: String,
    },
    /// Direct message received.
    DirectMessage {
        id: String,
        sender_id: String,
        recipient_id: String,
        text: Option<String>,
    },
    /// User updated profile.
    UserUpdated { user_id: String },
    /// Instance-level announcement.
    Announcement { id: String, text: String },
}

/// Redis Pub/Sub manager for event distribution.
#[derive(Clone)]
pub struct RedisPubSub {
    publisher: Client,
    subscriber: SubscriberClient,
    /// Local broadcast channel for events received from Redis.
    local_tx: broadcast::Sender<PubSubEvent>,
}

impl RedisPubSub {
    /// Create a new Redis Pub/Sub manager.
    pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
        let config = RedisConfig::from_url(redis_url)?;

        let publisher = Client::new(config.clone(), None, None, None);
        publisher.init().await?;

        let subscriber = SubscriberClient::new(config, None, None, None);
        subscriber.init().await?;

        let (local_tx, _) = broadcast::channel(1000);

        info!("Redis Pub/Sub initialized");

        Ok(Self {
            publisher,
            subscriber,
            local_tx,
        })
    }

    /// Subscribe to standard channels and start event loop.
    pub async fn start(&self) -> Result<(), RedisError> {
        // Subscribe to all standard channels
        self.subscriber.subscribe(channels::GLOBAL_TIMELINE).await?;
        self.subscriber.subscribe(channels::LOCAL_TIMELINE).await?;
        self.subscriber.subscribe(channels::NOTES).await?;
        self.subscriber.subscribe(channels::NOTIFICATIONS).await?;
        self.subscriber.subscribe(channels::FOLLOWS).await?;
        self.subscriber.subscribe(channels::REACTIONS).await?;

        info!("Subscribed to Redis Pub/Sub channels");

        // Spawn event loop
        let local_tx = self.local_tx.clone();
        let mut message_stream = self.subscriber.message_rx();

        tokio::spawn(async move {
            while let Ok(message) = message_stream.recv().await {
                if let Some(payload) = message.value.as_string() {
                    match serde_json::from_str::<PubSubEvent>(&payload) {
                        Ok(event) => {
                            debug!(?event, "Received Pub/Sub event");
                            if local_tx.send(event).is_err() {
                                warn!("No local subscribers for Pub/Sub event");
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse Pub/Sub message: {}", e);
                        }
                    }
                }
            }
            info!("Pub/Sub message stream ended");
        });

        Ok(())
    }

    /// Subscribe to user-specific events.
    pub async fn subscribe_user(&self, user_id: &str) -> Result<(), RedisError> {
        let channel = format!("{}{}", channels::USER_PREFIX, user_id);
        self.subscriber.subscribe(&channel).await?;
        debug!(user_id, "Subscribed to user channel");
        Ok(())
    }

    /// Unsubscribe from user-specific events.
    pub async fn unsubscribe_user(&self, user_id: &str) -> Result<(), RedisError> {
        let channel = format!("{}{}", channels::USER_PREFIX, user_id);
        self.subscriber.unsubscribe(&channel).await?;
        debug!(user_id, "Unsubscribed from user channel");
        Ok(())
    }

    /// Publish an event to a channel.
    pub async fn publish(&self, channel: &str, event: &PubSubEvent) -> Result<(), RedisError> {
        let payload = serde_json::to_string(event).map_err(|e| {
            RedisError::new(
                RedisErrorKind::InvalidArgument,
                format!("Serialization error: {e}"),
            )
        })?;
        let _: () = self.publisher.publish(channel, payload).await?;
        debug!(channel, ?event, "Published Pub/Sub event");
        Ok(())
    }

    /// Publish a note creation event.
    pub async fn publish_note_created(
        &self,
        id: &str,
        user_id: &str,
        text: Option<&str>,
        visibility: &str,
    ) -> Result<(), RedisError> {
        let event = PubSubEvent::NoteCreated {
            id: id.to_string(),
            user_id: user_id.to_string(),
            text: text.map(String::from),
            visibility: visibility.to_string(),
        };

        // Publish to notes channel
        self.publish(channels::NOTES, &event).await?;

        // Publish to appropriate timeline based on visibility
        match visibility {
            "public" => {
                self.publish(channels::GLOBAL_TIMELINE, &event).await?;
                self.publish(channels::LOCAL_TIMELINE, &event).await?;
            }
            "home" | "followers" => {
                self.publish(channels::LOCAL_TIMELINE, &event).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Publish a notification event to a specific user.
    pub async fn publish_notification(
        &self,
        id: &str,
        user_id: &str,
        notification_type: &str,
        source_user_id: Option<&str>,
        note_id: Option<&str>,
    ) -> Result<(), RedisError> {
        let event = PubSubEvent::Notification {
            id: id.to_string(),
            user_id: user_id.to_string(),
            notification_type: notification_type.to_string(),
            source_user_id: source_user_id.map(String::from),
            note_id: note_id.map(String::from),
        };

        // Publish to notifications channel
        self.publish(channels::NOTIFICATIONS, &event).await?;

        // Publish to user-specific channel
        let user_channel = format!("{}{}", channels::USER_PREFIX, user_id);
        self.publish(&user_channel, &event).await?;

        Ok(())
    }

    /// Publish a follow event.
    pub async fn publish_followed(
        &self,
        follower_id: &str,
        followee_id: &str,
    ) -> Result<(), RedisError> {
        let event = PubSubEvent::Followed {
            follower_id: follower_id.to_string(),
            followee_id: followee_id.to_string(),
        };

        self.publish(channels::FOLLOWS, &event).await?;

        // Notify the followee
        let user_channel = format!("{}{}", channels::USER_PREFIX, followee_id);
        self.publish(&user_channel, &event).await?;

        Ok(())
    }

    /// Publish a reaction event.
    pub async fn publish_reaction_added(
        &self,
        note_id: &str,
        user_id: &str,
        reaction: &str,
        note_author_id: &str,
    ) -> Result<(), RedisError> {
        let event = PubSubEvent::ReactionAdded {
            note_id: note_id.to_string(),
            user_id: user_id.to_string(),
            reaction: reaction.to_string(),
        };

        self.publish(channels::REACTIONS, &event).await?;

        // Notify the note author
        let user_channel = format!("{}{}", channels::USER_PREFIX, note_author_id);
        self.publish(&user_channel, &event).await?;

        Ok(())
    }

    /// Publish a note deleted event.
    pub async fn publish_note_deleted(&self, id: &str, user_id: &str) -> Result<(), RedisError> {
        let event = PubSubEvent::NoteDeleted {
            id: id.to_string(),
            user_id: user_id.to_string(),
        };

        self.publish(channels::NOTES, &event).await
    }

    /// Publish a note updated event.
    pub async fn publish_note_updated(&self, id: &str) -> Result<(), RedisError> {
        let event = PubSubEvent::NoteUpdated { id: id.to_string() };
        self.publish(channels::NOTES, &event).await
    }

    /// Publish an unfollow event.
    pub async fn publish_unfollowed(
        &self,
        follower_id: &str,
        followee_id: &str,
    ) -> Result<(), RedisError> {
        let event = PubSubEvent::Unfollowed {
            follower_id: follower_id.to_string(),
            followee_id: followee_id.to_string(),
        };

        self.publish(channels::FOLLOWS, &event).await?;

        // Notify the followee
        let user_channel = format!("{}{}", channels::USER_PREFIX, followee_id);
        self.publish(&user_channel, &event).await?;

        Ok(())
    }

    /// Publish a reaction removed event.
    pub async fn publish_reaction_removed(
        &self,
        note_id: &str,
        user_id: &str,
        reaction: &str,
        note_author_id: &str,
    ) -> Result<(), RedisError> {
        let event = PubSubEvent::ReactionRemoved {
            note_id: note_id.to_string(),
            user_id: user_id.to_string(),
            reaction: reaction.to_string(),
        };

        self.publish(channels::REACTIONS, &event).await?;

        // Notify the note author
        let user_channel = format!("{}{}", channels::USER_PREFIX, note_author_id);
        self.publish(&user_channel, &event).await?;

        Ok(())
    }

    /// Publish a direct message event.
    pub async fn publish_direct_message(
        &self,
        id: &str,
        sender_id: &str,
        recipient_id: &str,
        text: Option<&str>,
    ) -> Result<(), RedisError> {
        let event = PubSubEvent::DirectMessage {
            id: id.to_string(),
            sender_id: sender_id.to_string(),
            recipient_id: recipient_id.to_string(),
            text: text.map(String::from),
        };

        self.publish(channels::MESSAGING, &event).await?;

        // Notify the recipient
        let user_channel = format!("{}{}", channels::USER_PREFIX, recipient_id);
        self.publish(&user_channel, &event).await?;

        Ok(())
    }

    /// Get a receiver for local broadcast events.
    #[must_use]
    pub fn subscribe_local(&self) -> broadcast::Receiver<PubSubEvent> {
        self.local_tx.subscribe()
    }

    /// Get the number of local subscribers.
    #[must_use]
    pub fn local_subscriber_count(&self) -> usize {
        self.local_tx.receiver_count()
    }

    /// Shutdown the Pub/Sub manager.
    pub async fn shutdown(&self) -> Result<(), RedisError> {
        self.subscriber.quit().await?;
        self.publisher.quit().await?;
        info!("Redis Pub/Sub shutdown");
        Ok(())
    }
}

/// Implementation of EventPublisher for RedisPubSub.
/// This allows core services to publish events without depending on the queue crate directly.
#[async_trait]
impl EventPublisher for RedisPubSub {
    async fn publish_note_created(
        &self,
        id: &str,
        user_id: &str,
        text: Option<&str>,
        visibility: &str,
    ) -> AppResult<()> {
        self.publish_note_created(id, user_id, text, visibility)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_note_deleted(&self, id: &str, user_id: &str) -> AppResult<()> {
        RedisPubSub::publish_note_deleted(self, id, user_id)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_note_updated(&self, id: &str) -> AppResult<()> {
        RedisPubSub::publish_note_updated(self, id)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_followed(&self, follower_id: &str, followee_id: &str) -> AppResult<()> {
        RedisPubSub::publish_followed(self, follower_id, followee_id)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_unfollowed(&self, follower_id: &str, followee_id: &str) -> AppResult<()> {
        RedisPubSub::publish_unfollowed(self, follower_id, followee_id)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_reaction_added(
        &self,
        note_id: &str,
        user_id: &str,
        reaction: &str,
        note_author_id: &str,
    ) -> AppResult<()> {
        RedisPubSub::publish_reaction_added(self, note_id, user_id, reaction, note_author_id)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_reaction_removed(
        &self,
        note_id: &str,
        user_id: &str,
        reaction: &str,
        note_author_id: &str,
    ) -> AppResult<()> {
        RedisPubSub::publish_reaction_removed(self, note_id, user_id, reaction, note_author_id)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_notification(
        &self,
        id: &str,
        user_id: &str,
        notification_type: &str,
        source_user_id: Option<&str>,
        note_id: Option<&str>,
    ) -> AppResult<()> {
        RedisPubSub::publish_notification(
            self,
            id,
            user_id,
            notification_type,
            source_user_id,
            note_id,
        )
        .await
        .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }

    async fn publish_direct_message(
        &self,
        id: &str,
        sender_id: &str,
        recipient_id: &str,
        text: Option<&str>,
    ) -> AppResult<()> {
        RedisPubSub::publish_direct_message(self, id, sender_id, recipient_id, text)
            .await
            .map_err(|e| misskey_common::AppError::Internal(e.to_string()))
    }
}

/// Bridge between Redis Pub/Sub and SSE broadcaster.
pub struct PubSubSseBridge {
    pubsub: Arc<RedisPubSub>,
}

impl PubSubSseBridge {
    /// Create a new bridge.
    #[must_use]
    pub const fn new(pubsub: Arc<RedisPubSub>) -> Self {
        Self { pubsub }
    }

    /// Start the bridge, forwarding events from Redis to SSE.
    ///
    /// This method takes a callback that receives events and forwards them
    /// to the SSE broadcaster.
    pub async fn start<F>(&self, on_event: F)
    where
        F: Fn(PubSubEvent) + Send + Sync + 'static,
    {
        let mut rx = self.pubsub.subscribe_local();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => on_event(event),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("SSE bridge lagged by {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("SSE bridge channel closed");
                        break;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_names() {
        assert_eq!(channels::GLOBAL_TIMELINE, "misskey:timeline:global");
        assert_eq!(channels::LOCAL_TIMELINE, "misskey:timeline:local");
        assert_eq!(channels::USER_PREFIX, "misskey:user:");
    }

    #[test]
    fn test_pubsub_event_serialization() {
        let event = PubSubEvent::NoteCreated {
            id: "note1".to_string(),
            user_id: "user1".to_string(),
            text: Some("Hello".to_string()),
            visibility: "public".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"noteCreated\""));
        assert!(json.contains("\"id\":\"note1\""));

        let parsed: PubSubEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, PubSubEvent::NoteCreated { .. }));
    }

    #[test]
    fn test_notification_event_serialization() {
        let event = PubSubEvent::Notification {
            id: "notif1".to_string(),
            user_id: "user1".to_string(),
            notification_type: "follow".to_string(),
            source_user_id: Some("user2".to_string()),
            note_id: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"notification\""));

        let parsed: PubSubEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, PubSubEvent::Notification { .. }));
    }

    #[test]
    fn test_followed_event_serialization() {
        let event = PubSubEvent::Followed {
            follower_id: "user1".to_string(),
            followee_id: "user2".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        println!("JSON: {json}");
        assert!(json.contains("\"type\":\"followed\""));
        assert!(json.contains("\"follower_id\":\"user1\""));
        assert!(json.contains("\"followee_id\":\"user2\""));
    }

    #[test]
    fn test_reaction_event_serialization() {
        let event = PubSubEvent::ReactionAdded {
            note_id: "note1".to_string(),
            user_id: "user1".to_string(),
            reaction: "👍".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"reactionAdded\""));
        assert!(json.contains("\"reaction\":\"👍\""));
    }
}
