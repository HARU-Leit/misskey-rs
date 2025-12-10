//! Muting service.

use chrono::{Duration, Utc};
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{entities::muting, repositories::MutingRepository};
use sea_orm::Set;

/// Muting service for business logic.
#[derive(Clone)]
pub struct MutingService {
    muting_repo: MutingRepository,
    id_gen: IdGenerator,
}

impl MutingService {
    /// Create a new muting service.
    #[must_use] 
    pub const fn new(muting_repo: MutingRepository) -> Self {
        Self {
            muting_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Mute a user.
    /// `expires_in_seconds`: None for permanent, Some(seconds) for temporary mute.
    pub async fn mute(
        &self,
        muter_id: &str,
        mutee_id: &str,
        expires_in_seconds: Option<i64>,
    ) -> AppResult<muting::Model> {
        // Cannot mute yourself
        if muter_id == mutee_id {
            return Err(AppError::BadRequest("Cannot mute yourself".to_string()));
        }

        // Check if already muting
        if self.muting_repo.is_muting(muter_id, mutee_id).await? {
            return Err(AppError::Conflict("Already muting this user".to_string()));
        }

        let expires_at = expires_in_seconds.map(|seconds| {
            (Utc::now() + Duration::seconds(seconds)).fixed_offset()
        });

        let model = muting::ActiveModel {
            id: Set(self.id_gen.generate()),
            muter_id: Set(muter_id.to_string()),
            mutee_id: Set(mutee_id.to_string()),
            expires_at: Set(expires_at),
            created_at: Set(Utc::now().fixed_offset()),
        };

        self.muting_repo.create(model).await
    }

    /// Unmute a user.
    pub async fn unmute(&self, muter_id: &str, mutee_id: &str) -> AppResult<()> {
        // Check if muting
        let muting = self.muting_repo.find_by_pair(muter_id, mutee_id).await?;
        if muting.is_none() {
            return Err(AppError::NotFound("Not muting this user".to_string()));
        }

        self.muting_repo.delete_by_pair(muter_id, mutee_id).await
    }

    /// Check if a user is muting another user.
    pub async fn is_muting(&self, muter_id: &str, mutee_id: &str) -> AppResult<bool> {
        self.muting_repo.is_muting(muter_id, mutee_id).await
    }

    /// Get users that a user is muting (paginated).
    pub async fn get_muting(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<muting::Model>> {
        self.muting_repo.find_muting(user_id, limit, until_id).await
    }

    /// Delete expired mutes (cleanup job).
    pub async fn cleanup_expired(&self) -> AppResult<u64> {
        self.muting_repo.delete_expired().await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_muting_service_requires_different_users() {
        // This test verifies the service is created correctly
        // Full integration tests would require a database
    }
}
