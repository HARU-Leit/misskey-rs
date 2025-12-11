//! Event publisher service.
//!
//! Provides an abstraction for publishing real-time events.
//! The actual implementation is provided by the queue crate (Redis Pub/Sub).

use async_trait::async_trait;
use misskey_common::AppResult;
use std::sync::Arc;

/// Event types for real-time updates.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A new note was created.
    NoteCreated {
        id: String,
        user_id: String,
        text: Option<String>,
        visibility: String,
    },
    /// A note was deleted.
    NoteDeleted { id: String, user_id: String },
    /// A note was updated.
    NoteUpdated { id: String },
    /// A user followed another user.
    Followed {
        follower_id: String,
        followee_id: String,
    },
    /// A user unfollowed another user.
    Unfollowed {
        follower_id: String,
        followee_id: String,
    },
    /// A reaction was added to a note.
    ReactionAdded {
        note_id: String,
        user_id: String,
        reaction: String,
        note_author_id: String,
    },
    /// A reaction was removed from a note.
    ReactionRemoved {
        note_id: String,
        user_id: String,
        reaction: String,
        note_author_id: String,
    },
    /// A new notification was created.
    Notification {
        id: String,
        user_id: String,
        notification_type: String,
        source_user_id: Option<String>,
        note_id: Option<String>,
    },
    /// A new direct message was received.
    DirectMessage {
        id: String,
        sender_id: String,
        recipient_id: String,
        text: Option<String>,
    },
}

/// Trait for publishing real-time events.
///
/// This allows the core services to publish events
/// without directly depending on the queue/pubsub implementation.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a note created event.
    async fn publish_note_created(
        &self,
        id: &str,
        user_id: &str,
        text: Option<&str>,
        visibility: &str,
    ) -> AppResult<()>;

    /// Publish a note deleted event.
    async fn publish_note_deleted(&self, id: &str, user_id: &str) -> AppResult<()>;

    /// Publish a note updated event.
    async fn publish_note_updated(&self, id: &str) -> AppResult<()>;

    /// Publish a followed event.
    async fn publish_followed(&self, follower_id: &str, followee_id: &str) -> AppResult<()>;

    /// Publish an unfollowed event.
    async fn publish_unfollowed(&self, follower_id: &str, followee_id: &str) -> AppResult<()>;

    /// Publish a reaction added event.
    async fn publish_reaction_added(
        &self,
        note_id: &str,
        user_id: &str,
        reaction: &str,
        note_author_id: &str,
    ) -> AppResult<()>;

    /// Publish a reaction removed event.
    async fn publish_reaction_removed(
        &self,
        note_id: &str,
        user_id: &str,
        reaction: &str,
        note_author_id: &str,
    ) -> AppResult<()>;

    /// Publish a notification event.
    async fn publish_notification(
        &self,
        id: &str,
        user_id: &str,
        notification_type: &str,
        source_user_id: Option<&str>,
        note_id: Option<&str>,
    ) -> AppResult<()>;

    /// Publish a direct message event.
    async fn publish_direct_message(
        &self,
        id: &str,
        sender_id: &str,
        recipient_id: &str,
        text: Option<&str>,
    ) -> AppResult<()>;

    /// Publish a note created event to a specific channel timeline.
    async fn publish_channel_note_created(
        &self,
        channel_id: &str,
        note_id: &str,
        user_id: &str,
        text: Option<&str>,
        visibility: &str,
    ) -> AppResult<()>;
}

/// A no-op implementation of EventPublisher for testing or when real-time events are disabled.
#[derive(Clone, Default)]
pub struct NoOpEventPublisher;

#[async_trait]
impl EventPublisher for NoOpEventPublisher {
    async fn publish_note_created(
        &self,
        _id: &str,
        _user_id: &str,
        _text: Option<&str>,
        _visibility: &str,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn publish_note_deleted(&self, _id: &str, _user_id: &str) -> AppResult<()> {
        Ok(())
    }

    async fn publish_note_updated(&self, _id: &str) -> AppResult<()> {
        Ok(())
    }

    async fn publish_followed(&self, _follower_id: &str, _followee_id: &str) -> AppResult<()> {
        Ok(())
    }

    async fn publish_unfollowed(&self, _follower_id: &str, _followee_id: &str) -> AppResult<()> {
        Ok(())
    }

    async fn publish_reaction_added(
        &self,
        _note_id: &str,
        _user_id: &str,
        _reaction: &str,
        _note_author_id: &str,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn publish_reaction_removed(
        &self,
        _note_id: &str,
        _user_id: &str,
        _reaction: &str,
        _note_author_id: &str,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn publish_notification(
        &self,
        _id: &str,
        _user_id: &str,
        _notification_type: &str,
        _source_user_id: Option<&str>,
        _note_id: Option<&str>,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn publish_direct_message(
        &self,
        _id: &str,
        _sender_id: &str,
        _recipient_id: &str,
        _text: Option<&str>,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn publish_channel_note_created(
        &self,
        _channel_id: &str,
        _note_id: &str,
        _user_id: &str,
        _text: Option<&str>,
        _visibility: &str,
    ) -> AppResult<()> {
        Ok(())
    }
}

/// Wrapper for boxed EventPublisher trait object.
pub type EventPublisherService = Arc<dyn EventPublisher>;
