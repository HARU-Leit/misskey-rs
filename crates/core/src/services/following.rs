//! Following service.

use crate::services::delivery::DeliveryService;
use crate::services::event_publisher::EventPublisherService;
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{following, user},
    repositories::{FollowRequestRepository, FollowingRepository, UserRepository},
};
use sea_orm::Set;
use serde_json::json;

/// Following service for business logic.
#[derive(Clone)]
pub struct FollowingService {
    following_repo: FollowingRepository,
    follow_request_repo: FollowRequestRepository,
    user_repo: UserRepository,
    delivery: Option<DeliveryService>,
    event_publisher: Option<EventPublisherService>,
    server_url: String,
    id_gen: IdGenerator,
}

impl FollowingService {
    /// Create a new following service.
    #[must_use]
    pub fn new(
        following_repo: FollowingRepository,
        follow_request_repo: FollowRequestRepository,
        user_repo: UserRepository,
    ) -> Self {
        Self {
            following_repo,
            follow_request_repo,
            user_repo,
            delivery: None,
            event_publisher: None,
            server_url: String::new(),
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new following service with `ActivityPub` delivery support.
    #[must_use]
    pub fn with_delivery(
        following_repo: FollowingRepository,
        follow_request_repo: FollowRequestRepository,
        user_repo: UserRepository,
        delivery: DeliveryService,
        server_url: String,
    ) -> Self {
        Self {
            following_repo,
            follow_request_repo,
            user_repo,
            delivery: Some(delivery),
            event_publisher: None,
            server_url,
            id_gen: IdGenerator::new(),
        }
    }

    /// Set the delivery service.
    pub fn set_delivery(&mut self, delivery: DeliveryService, server_url: String) {
        self.delivery = Some(delivery);
        self.server_url = server_url;
    }

    /// Set the event publisher.
    pub fn set_event_publisher(&mut self, event_publisher: EventPublisherService) {
        self.event_publisher = Some(event_publisher);
    }

    /// Follow a user.
    ///
    /// If the target user has a locked account, this creates a follow request instead.
    pub async fn follow(&self, follower_id: &str, followee_id: &str) -> AppResult<FollowResult> {
        // Can't follow yourself
        if follower_id == followee_id {
            return Err(AppError::BadRequest("Cannot follow yourself".to_string()));
        }

        // Check if already following
        if self
            .following_repo
            .is_following(follower_id, followee_id)
            .await?
        {
            return Err(AppError::BadRequest("Already following".to_string()));
        }

        // Get both users
        let follower = self.user_repo.get_by_id(follower_id).await?;
        let followee = self.user_repo.get_by_id(followee_id).await?;

        // Check if the followee has a locked account
        if followee.is_locked {
            // Check if there's already a pending request
            if self
                .follow_request_repo
                .exists(follower_id, followee_id)
                .await?
            {
                return Err(AppError::BadRequest(
                    "Follow request already pending".to_string(),
                ));
            }

            // Create follow request
            let model = misskey_db::entities::follow_request::ActiveModel {
                id: Set(self.id_gen.generate()),
                follower_id: Set(follower_id.to_string()),
                followee_id: Set(followee_id.to_string()),
                follower_host: Set(follower.host.clone()),
                followee_host: Set(followee.host.clone()),
                follower_inbox: Set(follower.inbox.clone()),
                follower_shared_inbox: Set(follower.shared_inbox.clone()),
                ..Default::default()
            };

            self.follow_request_repo.create(model).await?;

            // Queue ActivityPub Follow activity for remote users
            if let Some(ref delivery) = self.delivery
                && followee.host.is_some()
                && let Some(ref inbox) = followee.inbox
                && let Err(e) = self
                    .queue_follow_activity(&follower, &followee, inbox, delivery)
                    .await
            {
                tracing::warn!(error = %e, "Failed to queue Follow activity");
            }

            return Ok(FollowResult::Pending);
        }

        // Create following relationship directly
        self.create_following(&follower, &followee).await?;

        // Queue ActivityPub Follow activity for remote users
        if let Some(ref delivery) = self.delivery
            && followee.host.is_some()
            && let Some(ref inbox) = followee.inbox
            && let Err(e) = self
                .queue_follow_activity(&follower, &followee, inbox, delivery)
                .await
        {
            tracing::warn!(error = %e, "Failed to queue Follow activity");
        }

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher
            && let Err(e) = event_publisher
                .publish_followed(follower_id, followee_id)
                .await
        {
            tracing::warn!(error = %e, "Failed to publish followed event");
        }

        Ok(FollowResult::Following)
    }

    /// Create a following relationship.
    async fn create_following(
        &self,
        follower: &misskey_db::entities::user::Model,
        followee: &misskey_db::entities::user::Model,
    ) -> AppResult<following::Model> {
        let model = following::ActiveModel {
            id: Set(self.id_gen.generate()),
            follower_id: Set(follower.id.clone()),
            followee_id: Set(followee.id.clone()),
            follower_host: Set(follower.host.clone()),
            followee_host: Set(followee.host.clone()),
            followee_inbox: Set(followee.inbox.clone()),
            followee_shared_inbox: Set(followee.shared_inbox.clone()),
            ..Default::default()
        };

        let following = self.following_repo.create(model).await?;

        // Update counts
        self.user_repo
            .increment_following_count(&follower.id)
            .await?;
        self.user_repo
            .increment_followers_count(&followee.id)
            .await?;

        Ok(following)
    }

    /// Unfollow a user.
    pub async fn unfollow(&self, follower_id: &str, followee_id: &str) -> AppResult<()> {
        // Check if following
        if !self
            .following_repo
            .is_following(follower_id, followee_id)
            .await?
        {
            return Err(AppError::BadRequest("Not following".to_string()));
        }

        // Get users for ActivityPub delivery
        let follower = self.user_repo.get_by_id(follower_id).await?;
        let followee = self.user_repo.get_by_id(followee_id).await?;

        self.following_repo
            .delete_by_pair(follower_id, followee_id)
            .await?;

        // Update counts
        self.user_repo
            .decrement_following_count(follower_id)
            .await?;
        self.user_repo
            .decrement_followers_count(followee_id)
            .await?;

        // Queue ActivityPub Undo activity for remote users
        if let Some(ref delivery) = self.delivery
            && followee.host.is_some()
            && let Some(ref inbox) = followee.inbox
            && let Err(e) = self
                .queue_undo_follow_activity(&follower, &followee, inbox, delivery)
                .await
        {
            tracing::warn!(error = %e, "Failed to queue Undo Follow activity");
        }

        // Publish real-time event
        if let Some(ref event_publisher) = self.event_publisher
            && let Err(e) = event_publisher
                .publish_unfollowed(follower_id, followee_id)
                .await
        {
            tracing::warn!(error = %e, "Failed to publish unfollowed event");
        }

        Ok(())
    }

    /// Accept a follow request.
    pub async fn accept_request(&self, followee_id: &str, follower_id: &str) -> AppResult<()> {
        // Find the request
        let request = self
            .follow_request_repo
            .find_by_pair(follower_id, followee_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Follow request not found".to_string()))?;

        // Get both users
        let follower = self.user_repo.get_by_id(follower_id).await?;
        let followee = self.user_repo.get_by_id(followee_id).await?;

        // Create following relationship
        self.create_following(&follower, &followee).await?;

        // Delete the request
        self.follow_request_repo.delete(&request.id).await?;

        // Queue ActivityPub Accept activity for remote followers
        if let Some(ref delivery) = self.delivery
            && follower.host.is_some()
            && let Some(ref inbox) = follower.inbox
            && let Err(e) = self
                .queue_accept_activity(&followee, &follower, inbox, delivery)
                .await
        {
            tracing::warn!(error = %e, "Failed to queue Accept activity");
        }

        // Publish real-time event (follow relationship established)
        if let Some(ref event_publisher) = self.event_publisher
            && let Err(e) = event_publisher
                .publish_followed(follower_id, followee_id)
                .await
        {
            tracing::warn!(error = %e, "Failed to publish followed event");
        }

        Ok(())
    }

    /// Reject a follow request.
    pub async fn reject_request(&self, followee_id: &str, follower_id: &str) -> AppResult<()> {
        // Get users for ActivityPub delivery
        let follower = self.user_repo.get_by_id(follower_id).await?;
        let followee = self.user_repo.get_by_id(followee_id).await?;

        self.follow_request_repo
            .delete_by_pair(follower_id, followee_id)
            .await?;

        // Queue ActivityPub Reject activity for remote followers
        if let Some(ref delivery) = self.delivery
            && follower.host.is_some()
            && let Some(ref inbox) = follower.inbox
            && let Err(e) = self
                .queue_reject_activity(&followee, &follower, inbox, delivery)
                .await
        {
            tracing::warn!(error = %e, "Failed to queue Reject activity");
        }

        Ok(())
    }

    /// Cancel a follow request.
    pub async fn cancel_request(&self, follower_id: &str, followee_id: &str) -> AppResult<()> {
        // Get users for ActivityPub delivery
        let follower = self.user_repo.get_by_id(follower_id).await?;
        let followee = self.user_repo.get_by_id(followee_id).await?;

        self.follow_request_repo
            .delete_by_pair(follower_id, followee_id)
            .await?;

        // Queue ActivityPub Undo activity for remote users
        if let Some(ref delivery) = self.delivery
            && followee.host.is_some()
            && let Some(ref inbox) = followee.inbox
            && let Err(e) = self
                .queue_undo_follow_activity(&follower, &followee, inbox, delivery)
                .await
        {
            tracing::warn!(error = %e, "Failed to queue Undo Follow activity");
        }

        Ok(())
    }

    /// Get followers of a user.
    pub async fn get_followers(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<following::Model>> {
        self.following_repo
            .find_followers(user_id, limit, until_id)
            .await
    }

    /// Get users that a user is following.
    pub async fn get_following(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<following::Model>> {
        self.following_repo
            .find_following(user_id, limit, until_id)
            .await
    }

    /// Check if a user is following another.
    pub async fn is_following(&self, follower_id: &str, followee_id: &str) -> AppResult<bool> {
        self.following_repo
            .is_following(follower_id, followee_id)
            .await
    }

    /// Get pending follow requests received by a user.
    pub async fn get_pending_requests(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<misskey_db::entities::follow_request::Model>> {
        self.follow_request_repo
            .find_received(user_id, limit, until_id)
            .await
    }

    // ==================== ActivityPub Delivery Helpers ====================

    /// Queue a Follow activity.
    async fn queue_follow_activity(
        &self,
        follower: &user::Model,
        followee: &user::Model,
        inbox: &str,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        let actor_url = format!("{}/users/{}", self.server_url, follower.id);
        let object_url = followee
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/users/{}", self.server_url, followee.id));
        let activity_id = format!(
            "{}/activities/follow/{}/{}",
            self.server_url, follower.id, followee.id
        );

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Follow",
            "id": activity_id,
            "actor": actor_url,
            "object": object_url,
        });

        delivery.queue_follow(&follower.id, inbox, activity).await?;
        tracing::debug!(follower_id = %follower.id, followee_id = %followee.id, "Queued Follow activity");
        Ok(())
    }

    /// Queue an Undo Follow activity.
    async fn queue_undo_follow_activity(
        &self,
        follower: &user::Model,
        followee: &user::Model,
        inbox: &str,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        let actor_url = format!("{}/users/{}", self.server_url, follower.id);
        let object_url = followee
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/users/{}", self.server_url, followee.id));
        let follow_id = format!(
            "{}/activities/follow/{}/{}",
            self.server_url, follower.id, followee.id
        );
        let undo_id = format!(
            "{}/activities/undo/follow/{}/{}",
            self.server_url, follower.id, followee.id
        );

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Undo",
            "id": undo_id,
            "actor": actor_url,
            "object": {
                "type": "Follow",
                "id": follow_id,
                "actor": actor_url,
                "object": object_url,
            },
        });

        delivery
            .queue_undo(&follower.id, vec![inbox.to_string()], activity)
            .await?;
        tracing::debug!(follower_id = %follower.id, followee_id = %followee.id, "Queued Undo Follow activity");
        Ok(())
    }

    /// Queue an Accept activity.
    async fn queue_accept_activity(
        &self,
        accepter: &user::Model,
        follower: &user::Model,
        inbox: &str,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        let actor_url = format!("{}/users/{}", self.server_url, accepter.id);
        let follower_url = follower
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/users/{}", self.server_url, follower.id));
        let follow_id = format!(
            "{}/activities/follow/{}/{}",
            self.server_url, follower.id, accepter.id
        );
        let accept_id = format!(
            "{}/activities/accept/{}/{}",
            self.server_url, accepter.id, follower.id
        );

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Accept",
            "id": accept_id,
            "actor": actor_url,
            "object": {
                "type": "Follow",
                "id": follow_id,
                "actor": follower_url,
                "object": actor_url,
            },
        });

        delivery
            .queue_accept_follow(&accepter.id, inbox, activity)
            .await?;
        tracing::debug!(accepter_id = %accepter.id, follower_id = %follower.id, "Queued Accept activity");
        Ok(())
    }

    /// Queue a Reject activity.
    async fn queue_reject_activity(
        &self,
        rejecter: &user::Model,
        follower: &user::Model,
        inbox: &str,
        delivery: &DeliveryService,
    ) -> AppResult<()> {
        let actor_url = format!("{}/users/{}", self.server_url, rejecter.id);
        let follower_url = follower
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/users/{}", self.server_url, follower.id));
        let follow_id = format!(
            "{}/activities/follow/{}/{}",
            self.server_url, follower.id, rejecter.id
        );
        let reject_id = format!(
            "{}/activities/reject/{}/{}",
            self.server_url, rejecter.id, follower.id
        );

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Reject",
            "id": reject_id,
            "actor": actor_url,
            "object": {
                "type": "Follow",
                "id": follow_id,
                "actor": follower_url,
                "object": actor_url,
            },
        });

        delivery
            .queue_reject_follow(&rejecter.id, inbox, activity)
            .await?;
        tracing::debug!(rejecter_id = %rejecter.id, follower_id = %follower.id, "Queued Reject activity");
        Ok(())
    }
}

/// Result of a follow operation.
pub enum FollowResult {
    /// The user is now following the target.
    Following,
    /// A follow request was created (target has locked account).
    Pending,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use misskey_db::entities::user;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

    #[allow(dead_code)]
    fn create_test_user(id: &str, username: &str, is_locked: bool) -> user::Model {
        user::Model {
            id: id.to_string(),
            username: username.to_string(),
            username_lower: username.to_lowercase(),
            host: None,
            name: Some("Test User".to_string()),
            description: None,
            avatar_url: None,
            banner_url: None,
            is_bot: false,
            is_cat: false,
            is_locked,
            is_suspended: false,
            is_silenced: false,
            is_admin: false,
            is_moderator: false,
            followers_count: 0,
            following_count: 0,
            notes_count: 0,
            inbox: None,
            shared_inbox: None,
            featured: None,
            uri: None,
            last_fetched_at: None,
            token: Some("test_token".to_string()),
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    fn create_test_following(id: &str, follower_id: &str, followee_id: &str) -> following::Model {
        following::Model {
            id: id.to_string(),
            follower_id: follower_id.to_string(),
            followee_id: followee_id.to_string(),
            follower_host: None,
            followee_host: None,
            followee_inbox: None,
            followee_shared_inbox: None,
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_follow_yourself_returns_error() {
        let db1 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let db2 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let db3 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let following_repo = FollowingRepository::new(db1);
        let follow_request_repo = FollowRequestRepository::new(db2);
        let user_repo = UserRepository::new(db3);

        let service = FollowingService::new(following_repo, follow_request_repo, user_repo);
        let result = service.follow("user1", "user1").await;

        assert!(result.is_err());
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("Cannot follow yourself"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_follow_already_following_returns_error() {
        let following = create_test_following("f1", "user1", "user2");

        let db1 = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[following.clone()]])
                .into_connection(),
        );
        let db2 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let db3 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let following_repo = FollowingRepository::new(db1);
        let follow_request_repo = FollowRequestRepository::new(db2);
        let user_repo = UserRepository::new(db3);

        let service = FollowingService::new(following_repo, follow_request_repo, user_repo);
        let result = service.follow("user1", "user2").await;

        assert!(result.is_err());
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("Already following"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_is_following() {
        let following = create_test_following("f1", "user1", "user2");

        let db1 = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[following.clone()]])
                .into_connection(),
        );
        let db2 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let db3 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let following_repo = FollowingRepository::new(db1);
        let follow_request_repo = FollowRequestRepository::new(db2);
        let user_repo = UserRepository::new(db3);

        let service = FollowingService::new(following_repo, follow_request_repo, user_repo);
        let result = service.is_following("user1", "user2").await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_is_not_following() {
        let db1 = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<following::Model>::new()])
                .into_connection(),
        );
        let db2 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let db3 = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let following_repo = FollowingRepository::new(db1);
        let follow_request_repo = FollowRequestRepository::new(db2);
        let user_repo = UserRepository::new(db3);

        let service = FollowingService::new(following_repo, follow_request_repo, user_repo);
        let result = service.is_following("user1", "user2").await.unwrap();

        assert!(!result);
    }
}
