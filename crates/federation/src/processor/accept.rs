//! Accept activity processor.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::following,
    repositories::{FollowRequestRepository, FollowingRepository, UserRepository},
};
use sea_orm::Set;
use tracing::info;

use crate::activities::AcceptActivity;

/// Processor for Accept activities (follow acceptance).
#[derive(Clone)]
pub struct AcceptProcessor {
    user_repo: UserRepository,
    following_repo: FollowingRepository,
    follow_request_repo: FollowRequestRepository,
    id_gen: IdGenerator,
}

impl AcceptProcessor {
    /// Create a new accept processor.
    #[must_use] 
    pub const fn new(
        user_repo: UserRepository,
        following_repo: FollowingRepository,
        follow_request_repo: FollowRequestRepository,
    ) -> Self {
        Self {
            user_repo,
            following_repo,
            follow_request_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Process an incoming Accept activity.
    ///
    /// When we receive an Accept, the actor is the remote user who accepted our follow.
    /// The object is the original Follow activity we sent.
    /// We find the pending follow request to our remote user and complete it.
    pub async fn process(&self, activity: &AcceptActivity) -> AppResult<following::Model> {
        info!(
            actor = %activity.actor,
            object = %activity.object,
            "Processing Accept activity"
        );

        // The actor is the remote user who accepted the follow
        let followee = self
            .user_repo
            .find_by_uri(activity.actor.as_str())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Followee not found: {}", activity.actor)))?;

        // Find the pending follow request TO this remote user
        // This returns the follow request where followee_id matches
        let follow_request = self
            .follow_request_repo
            .find_by_followee(&followee.id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "No pending follow request to user: {}",
                    followee.id
                ))
            })?;

        // Get the follower (our local user)
        let follower = self
            .user_repo
            .find_by_id(&follow_request.follower_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Follower not found: {}", follow_request.follower_id))
            })?;

        // Check if we're already following
        if self
            .following_repo
            .is_following(&follower.id, &followee.id)
            .await?
        {
            info!("Already following, ignoring Accept");
            // Delete the follow request if it exists
            self.follow_request_repo
                .delete_by_pair(&follower.id, &followee.id)
                .await?;
            // Return the existing following
            return self
                .following_repo
                .find_by_pair(&follower.id, &followee.id)
                .await?
                .ok_or_else(|| AppError::NotFound("Following not found".to_string()));
        }

        // Delete the pending follow request
        self.follow_request_repo
            .delete_by_pair(&follower.id, &followee.id)
            .await?;

        // Create the following relationship
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

        info!(
            follower = %follower.id,
            followee = %followee.id,
            "Follow accepted, created following relationship"
        );

        Ok(following)
    }
}
