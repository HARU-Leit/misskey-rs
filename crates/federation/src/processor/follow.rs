//! Follow activity processor.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{follow_request, following, user},
    repositories::{FollowRequestRepository, FollowingRepository, UserRepository},
};
use sea_orm::Set;
use serde_json::{Value, json};
use tracing::info;
use url::Url;

use super::ActorFetcher;
use crate::FollowActivity;
use crate::client::ApClient;

/// Processor for Follow activities.
#[derive(Clone)]
pub struct FollowProcessor {
    user_repo: UserRepository,
    following_repo: FollowingRepository,
    follow_request_repo: FollowRequestRepository,
    actor_fetcher: ActorFetcher,
    id_gen: IdGenerator,
    base_url: Option<Url>,
}

/// Result of processing a Follow activity.
#[derive(Debug)]
pub enum FollowProcessResult {
    /// Follow was accepted immediately.
    Accepted {
        /// The followee user ID (local user).
        followee_id: String,
        /// The follower user ID (remote user).
        follower_id: String,
        /// The Accept activity to send back (if `base_url` was provided).
        accept_activity: Option<AcceptActivityInfo>,
    },
    /// Follow request created (target has locked account).
    Pending {
        /// The followee user ID (local user).
        followee_id: String,
        /// The follower user ID (remote user).
        follower_id: String,
    },
    /// Follow was rejected.
    Rejected { reason: String },
}

/// Information about an Accept activity to be queued.
#[derive(Debug, Clone)]
pub struct AcceptActivityInfo {
    /// The user ID of the accepter (local user).
    pub accepter_id: String,
    /// The inbox URL to send the Accept activity to.
    pub inbox_url: String,
    /// The serialized Accept activity.
    pub activity: Value,
}

impl FollowProcessor {
    /// Create a new follow processor.
    #[must_use]
    pub fn new(
        user_repo: UserRepository,
        following_repo: FollowingRepository,
        follow_request_repo: FollowRequestRepository,
        ap_client: ApClient,
    ) -> Self {
        Self {
            user_repo: user_repo.clone(),
            following_repo,
            follow_request_repo,
            actor_fetcher: ActorFetcher::new(user_repo, ap_client),
            id_gen: IdGenerator::new(),
            base_url: None,
        }
    }

    /// Create a new follow processor with a base URL for generating Accept activities.
    #[must_use]
    pub fn with_base_url(
        user_repo: UserRepository,
        following_repo: FollowingRepository,
        follow_request_repo: FollowRequestRepository,
        ap_client: ApClient,
        base_url: Url,
    ) -> Self {
        Self {
            user_repo: user_repo.clone(),
            following_repo,
            follow_request_repo,
            actor_fetcher: ActorFetcher::new(user_repo, ap_client),
            id_gen: IdGenerator::new(),
            base_url: Some(base_url),
        }
    }

    /// Process an incoming Follow activity from a remote actor.
    ///
    /// This handles:
    /// 1. Looking up or creating the remote actor
    /// 2. Finding the local user being followed
    /// 3. Creating a follow relationship or follow request
    pub async fn process(&self, activity: &FollowActivity) -> AppResult<FollowProcessResult> {
        info!(
            actor = %activity.actor,
            object = %activity.object,
            "Processing Follow activity"
        );

        // Parse the object URL to extract the local user ID
        let object_url = &activity.object;
        let local_user_id = self.extract_local_user_id(object_url)?;

        // Find the local user
        let followee = self
            .user_repo
            .find_by_id(&local_user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Local user not found: {local_user_id}")))?;

        // Check if local user is suspended
        if followee.is_suspended {
            return Ok(FollowProcessResult::Rejected {
                reason: "Target user is suspended".to_string(),
            });
        }

        // Find or create the remote actor
        let follower = self.find_or_fetch_remote_actor(&activity.actor).await?;

        // Check if already following
        if self
            .following_repo
            .is_following(&follower.id, &followee.id)
            .await?
        {
            info!("Already following, accepting anyway");
            // Build Accept activity for already following case
            let accept_activity = self.build_accept_activity(&followee, &follower, &activity.id);
            return Ok(FollowProcessResult::Accepted {
                followee_id: followee.id.clone(),
                follower_id: follower.id.clone(),
                accept_activity,
            });
        }

        // Check if target has locked account
        if followee.is_locked {
            // Check if there's already a pending request
            if self
                .follow_request_repo
                .exists(&follower.id, &followee.id)
                .await?
            {
                info!("Follow request already pending");
                return Ok(FollowProcessResult::Pending {
                    followee_id: followee.id.clone(),
                    follower_id: follower.id.clone(),
                });
            }

            // Create follow request
            let request = follow_request::ActiveModel {
                id: Set(self.id_gen.generate()),
                follower_id: Set(follower.id.clone()),
                followee_id: Set(followee.id.clone()),
                follower_host: Set(follower.host.clone()),
                followee_host: Set(None), // Local user
                follower_inbox: Set(follower.inbox.clone()),
                follower_shared_inbox: Set(follower.shared_inbox.clone()),
                ..Default::default()
            };

            self.follow_request_repo.create(request).await?;

            info!(
                follower = %follower.id,
                followee = %followee.id,
                "Created follow request"
            );

            // Return result indicating notification should be created for follow request
            return Ok(FollowProcessResult::Pending {
                followee_id: followee.id.clone(),
                follower_id: follower.id.clone(),
            });
        }

        // Auto-accept: create following relationship
        self.create_following(&follower, &followee).await?;

        info!(
            follower = %follower.id,
            followee = %followee.id,
            "Follow accepted"
        );

        // Build Accept activity to send back
        let accept_activity = self.build_accept_activity(&followee, &follower, &activity.id);

        Ok(FollowProcessResult::Accepted {
            followee_id: followee.id.clone(),
            follower_id: follower.id.clone(),
            accept_activity,
        })
    }

    /// Build an Accept activity for a follow.
    fn build_accept_activity(
        &self,
        accepter: &user::Model,
        follower: &user::Model,
        follow_activity_id: &Url,
    ) -> Option<AcceptActivityInfo> {
        let base_url = self.base_url.as_ref()?;
        let inbox_url = follower.inbox.as_ref()?;

        let actor_url = format!("{}/users/{}", base_url, accepter.id);
        let follower_uri = follower
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/users/{}", base_url, follower.id));
        let activity_id = format!("{}/accept/{}", actor_url, self.id_gen.generate());

        let activity = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": activity_id,
            "type": "Accept",
            "actor": actor_url,
            "object": {
                "id": follow_activity_id.as_str(),
                "type": "Follow",
                "actor": follower_uri,
                "object": actor_url
            }
        });

        Some(AcceptActivityInfo {
            accepter_id: accepter.id.clone(),
            inbox_url: inbox_url.clone(),
            activity,
        })
    }

    /// Extract the local user ID from an actor URL.
    ///
    /// Expected format: `https://example.com/users/{user_id}`
    fn extract_local_user_id(&self, url: &Url) -> AppResult<String> {
        let path = url.path();

        // Try /users/{id} format
        if let Some(id) = path.strip_prefix("/users/") {
            return Ok(id.to_string());
        }

        Err(AppError::BadRequest(format!(
            "Cannot extract user ID from URL: {url}"
        )))
    }

    /// Find an existing remote actor or fetch from remote server.
    async fn find_or_fetch_remote_actor(&self, actor_url: &Url) -> AppResult<user::Model> {
        self.actor_fetcher.find_or_fetch(actor_url).await
    }

    /// Create a following relationship.
    async fn create_following(
        &self,
        follower: &user::Model,
        followee: &user::Model,
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
}
