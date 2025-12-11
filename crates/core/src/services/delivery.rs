//! ActivityPub delivery service.
//!
//! Provides an abstraction for queueing ActivityPub activity delivery.
//! The actual implementation is provided by the queue crate.

use async_trait::async_trait;
use misskey_common::AppResult;
use serde_json::Value;
use std::sync::Arc;

/// Trait for ActivityPub delivery.
///
/// This allows the core services to queue ActivityPub activities
/// without directly depending on the queue implementation.
#[async_trait]
pub trait ActivityDelivery: Send + Sync {
    /// Queue a Create activity for a note.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user who created the note
    /// * `note_id` - The ID of the note
    /// * `activity` - The serialized Create activity
    /// * `inboxes` - List of inbox URLs to deliver to
    async fn queue_create_note(
        &self,
        user_id: &str,
        note_id: &str,
        activity: Value,
        inboxes: Vec<String>,
    ) -> AppResult<()>;

    /// Queue a Delete activity for a note.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user who deleted the note
    /// * `note_id` - The ID of the note
    /// * `activity` - The serialized Delete activity
    /// * `inboxes` - List of inbox URLs to deliver to
    async fn queue_delete_note(
        &self,
        user_id: &str,
        note_id: &str,
        activity: Value,
        inboxes: Vec<String>,
    ) -> AppResult<()>;

    /// Queue a Follow activity.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the follower
    /// * `target_inbox` - The inbox URL of the target user
    /// * `activity` - The serialized Follow activity
    async fn queue_follow(
        &self,
        user_id: &str,
        target_inbox: &str,
        activity: Value,
    ) -> AppResult<()>;

    /// Queue an Accept activity for a follow request.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user accepting the follow
    /// * `target_inbox` - The inbox URL of the follower
    /// * `activity` - The serialized Accept activity
    async fn queue_accept_follow(
        &self,
        user_id: &str,
        target_inbox: &str,
        activity: Value,
    ) -> AppResult<()>;

    /// Queue a Reject activity for a follow request.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user rejecting the follow
    /// * `target_inbox` - The inbox URL of the follower
    /// * `activity` - The serialized Reject activity
    async fn queue_reject_follow(
        &self,
        user_id: &str,
        target_inbox: &str,
        activity: Value,
    ) -> AppResult<()>;

    /// Queue an Undo activity.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user
    /// * `inboxes` - List of inbox URLs to deliver to
    /// * `activity` - The serialized Undo activity
    async fn queue_undo(
        &self,
        user_id: &str,
        inboxes: Vec<String>,
        activity: Value,
    ) -> AppResult<()>;

    /// Queue a Like activity.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user who reacted
    /// * `target_inbox` - The inbox URL of the note author
    /// * `activity` - The serialized Like activity
    async fn queue_like(&self, user_id: &str, target_inbox: &str, activity: Value)
    -> AppResult<()>;

    /// Queue an Announce activity (boost/renote).
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user
    /// * `inboxes` - List of inbox URLs to deliver to
    /// * `activity` - The serialized Announce activity
    async fn queue_announce(
        &self,
        user_id: &str,
        inboxes: Vec<String>,
        activity: Value,
    ) -> AppResult<()>;

    /// Queue an Update activity for a note.
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user who updated the note
    /// * `note_id` - The ID of the note
    /// * `activity` - The serialized Update activity
    /// * `inboxes` - List of inbox URLs to deliver to
    async fn queue_update_note(
        &self,
        user_id: &str,
        note_id: &str,
        activity: Value,
        inboxes: Vec<String>,
    ) -> AppResult<()>;
}

/// A no-op implementation of ActivityDelivery for testing or when federation is disabled.
#[derive(Clone, Default)]
pub struct NoOpDelivery;

#[async_trait]
impl ActivityDelivery for NoOpDelivery {
    async fn queue_create_note(
        &self,
        _user_id: &str,
        _note_id: &str,
        _activity: Value,
        _inboxes: Vec<String>,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_delete_note(
        &self,
        _user_id: &str,
        _note_id: &str,
        _activity: Value,
        _inboxes: Vec<String>,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_follow(
        &self,
        _user_id: &str,
        _target_inbox: &str,
        _activity: Value,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_accept_follow(
        &self,
        _user_id: &str,
        _target_inbox: &str,
        _activity: Value,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_reject_follow(
        &self,
        _user_id: &str,
        _target_inbox: &str,
        _activity: Value,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_undo(
        &self,
        _user_id: &str,
        _inboxes: Vec<String>,
        _activity: Value,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_like(
        &self,
        _user_id: &str,
        _target_inbox: &str,
        _activity: Value,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_announce(
        &self,
        _user_id: &str,
        _inboxes: Vec<String>,
        _activity: Value,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn queue_update_note(
        &self,
        _user_id: &str,
        _note_id: &str,
        _activity: Value,
        _inboxes: Vec<String>,
    ) -> AppResult<()> {
        Ok(())
    }
}

/// Wrapper for boxed ActivityDelivery trait object.
pub type DeliveryService = Arc<dyn ActivityDelivery>;
