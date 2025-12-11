//! Reject activity processor.

use misskey_common::{AppError, AppResult};
use misskey_db::repositories::{FollowRequestRepository, UserRepository};
use tracing::info;

use crate::activities::RejectActivity;

/// Processor for Reject activities (follow rejection).
#[derive(Clone)]
pub struct RejectProcessor {
    user_repo: UserRepository,
    follow_request_repo: FollowRequestRepository,
}

impl RejectProcessor {
    /// Create a new reject processor.
    #[must_use]
    pub const fn new(
        user_repo: UserRepository,
        follow_request_repo: FollowRequestRepository,
    ) -> Self {
        Self {
            user_repo,
            follow_request_repo,
        }
    }

    /// Process an incoming Reject activity.
    ///
    /// When we receive a Reject, the actor is the remote user who rejected our follow.
    /// The object is the original Follow activity we sent.
    pub async fn process(&self, activity: &RejectActivity) -> AppResult<()> {
        info!(
            actor = %activity.actor,
            object = %activity.object,
            "Processing Reject activity"
        );

        // The actor is the remote user who rejected the follow
        let followee = self
            .user_repo
            .find_by_uri(activity.actor.as_str())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Followee not found: {}", activity.actor)))?;

        // Find the pending follow request TO this remote user
        if let Some(follow_request) = self
            .follow_request_repo
            .find_by_followee(&followee.id)
            .await?
        {
            // Delete the follow request
            self.follow_request_repo
                .delete_by_pair(&follow_request.follower_id, &followee.id)
                .await?;

            info!(
                follower = %follow_request.follower_id,
                followee = %followee.id,
                "Follow request rejected and deleted"
            );
        } else {
            info!(
                followee = %followee.id,
                "No pending follow request found"
            );
        }

        Ok(())
    }
}
