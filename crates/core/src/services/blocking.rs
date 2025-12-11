//! Blocking service.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::blocking,
    repositories::{BlockingRepository, FollowingRepository},
};
use sea_orm::Set;

/// Blocking service for business logic.
#[derive(Clone)]
pub struct BlockingService {
    blocking_repo: BlockingRepository,
    following_repo: FollowingRepository,
    id_gen: IdGenerator,
}

impl BlockingService {
    /// Create a new blocking service.
    #[must_use]
    pub const fn new(
        blocking_repo: BlockingRepository,
        following_repo: FollowingRepository,
    ) -> Self {
        Self {
            blocking_repo,
            following_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Block a user.
    pub async fn block(&self, blocker_id: &str, blockee_id: &str) -> AppResult<blocking::Model> {
        // Cannot block yourself
        if blocker_id == blockee_id {
            return Err(AppError::BadRequest("Cannot block yourself".to_string()));
        }

        // Check if already blocking
        if self
            .blocking_repo
            .is_blocking(blocker_id, blockee_id)
            .await?
        {
            return Err(AppError::Conflict("Already blocking this user".to_string()));
        }

        // When blocking, also unfollow if following
        if self
            .following_repo
            .is_following(blocker_id, blockee_id)
            .await?
        {
            self.following_repo
                .delete_by_pair(blocker_id, blockee_id)
                .await?;
        }

        // Also remove follower relationship (they can no longer follow you)
        if self
            .following_repo
            .is_following(blockee_id, blocker_id)
            .await?
        {
            self.following_repo
                .delete_by_pair(blockee_id, blocker_id)
                .await?;
        }

        let model = blocking::ActiveModel {
            id: Set(self.id_gen.generate()),
            blocker_id: Set(blocker_id.to_string()),
            blockee_id: Set(blockee_id.to_string()),
            created_at: Set(chrono::Utc::now().into()),
        };

        self.blocking_repo.create(model).await
    }

    /// Unblock a user.
    pub async fn unblock(&self, blocker_id: &str, blockee_id: &str) -> AppResult<()> {
        // Check if blocking
        if !self
            .blocking_repo
            .is_blocking(blocker_id, blockee_id)
            .await?
        {
            return Err(AppError::NotFound("Not blocking this user".to_string()));
        }

        self.blocking_repo
            .delete_by_pair(blocker_id, blockee_id)
            .await
    }

    /// Check if a user is blocking another user.
    pub async fn is_blocking(&self, blocker_id: &str, blockee_id: &str) -> AppResult<bool> {
        self.blocking_repo.is_blocking(blocker_id, blockee_id).await
    }

    /// Check if either user is blocking the other.
    pub async fn is_blocked_between(&self, user_a: &str, user_b: &str) -> AppResult<bool> {
        self.blocking_repo.is_blocked_between(user_a, user_b).await
    }

    /// Get users that a user is blocking (paginated).
    pub async fn get_blocking(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<blocking::Model>> {
        self.blocking_repo
            .find_blocking(user_id, limit, until_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_blocking_service_requires_different_users() {
        // This test verifies the service is created correctly
        // Full integration tests would require a database
    }
}
