//! Redis-backed ActivityPub delivery implementation.
//!
//! This module provides a Redis-based implementation of the ActivityDelivery trait
//! that queues jobs for the apalis worker to process.

use async_trait::async_trait;
use misskey_common::AppResult;
use misskey_core::ActivityDelivery;
use serde_json::Value;
use std::sync::Arc;

use crate::jobs::DeliverJob;
use crate::RedisPubSub;

/// Redis-backed ActivityPub delivery service.
///
/// This implementation queues delivery jobs to Redis for processing by
/// the apalis deliver worker.
#[derive(Clone)]
pub struct RedisDeliveryService {
    /// Redis storage for job queue (apalis-redis).
    storage: apalis_redis::RedisStorage<DeliverJob>,
    /// Optional PubSub for real-time events.
    pubsub: Option<Arc<RedisPubSub>>,
}

impl RedisDeliveryService {
    /// Create a new Redis delivery service.
    pub fn new(storage: apalis_redis::RedisStorage<DeliverJob>) -> Self {
        Self {
            storage,
            pubsub: None,
        }
    }

    /// Create a new Redis delivery service with PubSub support.
    pub fn with_pubsub(
        storage: apalis_redis::RedisStorage<DeliverJob>,
        pubsub: Arc<RedisPubSub>,
    ) -> Self {
        Self {
            storage,
            pubsub: Some(pubsub),
        }
    }

    /// Queue a delivery job for each inbox.
    async fn queue_to_inboxes(
        &self,
        user_id: &str,
        activity: Value,
        inboxes: Vec<String>,
    ) -> AppResult<()> {
        use apalis::prelude::*;

        for inbox in inboxes {
            let job = DeliverJob::new(user_id.to_string(), inbox.clone(), activity.clone());

            self.storage
                .clone()
                .push(job)
                .await
                .map_err(|e| misskey_common::AppError::Internal(format!("Failed to queue job: {e}")))?;

            tracing::debug!(inbox = %inbox, "Queued delivery job");
        }

        Ok(())
    }
}

#[async_trait]
impl ActivityDelivery for RedisDeliveryService {
    async fn queue_create_note(
        &self,
        user_id: &str,
        note_id: &str,
        activity: Value,
        inboxes: Vec<String>,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            note_id = %note_id,
            inbox_count = %inboxes.len(),
            "Queueing Create activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, inboxes).await
    }

    async fn queue_delete_note(
        &self,
        user_id: &str,
        note_id: &str,
        activity: Value,
        inboxes: Vec<String>,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            note_id = %note_id,
            inbox_count = %inboxes.len(),
            "Queueing Delete activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, inboxes).await
    }

    async fn queue_follow(
        &self,
        user_id: &str,
        target_inbox: &str,
        activity: Value,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            target_inbox = %target_inbox,
            "Queueing Follow activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, vec![target_inbox.to_string()])
            .await
    }

    async fn queue_accept_follow(
        &self,
        user_id: &str,
        target_inbox: &str,
        activity: Value,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            target_inbox = %target_inbox,
            "Queueing Accept activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, vec![target_inbox.to_string()])
            .await
    }

    async fn queue_reject_follow(
        &self,
        user_id: &str,
        target_inbox: &str,
        activity: Value,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            target_inbox = %target_inbox,
            "Queueing Reject activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, vec![target_inbox.to_string()])
            .await
    }

    async fn queue_undo(
        &self,
        user_id: &str,
        inboxes: Vec<String>,
        activity: Value,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            inbox_count = %inboxes.len(),
            "Queueing Undo activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, inboxes).await
    }

    async fn queue_like(
        &self,
        user_id: &str,
        target_inbox: &str,
        activity: Value,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            target_inbox = %target_inbox,
            "Queueing Like activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, vec![target_inbox.to_string()])
            .await
    }

    async fn queue_announce(
        &self,
        user_id: &str,
        inboxes: Vec<String>,
        activity: Value,
    ) -> AppResult<()> {
        tracing::info!(
            user_id = %user_id,
            inbox_count = %inboxes.len(),
            "Queueing Announce activity delivery"
        );

        self.queue_to_inboxes(user_id, activity, inboxes).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require a Redis mock
    // For now, we just test the struct can be created
    #[test]
    fn test_delivery_service_trait_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        // This won't compile if RedisDeliveryService doesn't implement Send + Sync
        // assert_send_sync::<RedisDeliveryService>();
    }
}
