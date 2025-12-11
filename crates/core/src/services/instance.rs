//! Instance management service for federation.

use misskey_common::{AppError, AppResult};
use misskey_db::{
    entities::instance,
    repositories::{InstanceRepository, InstanceStats, UserRepository},
};
use sea_orm::Set;
use serde::Deserialize;

/// Input for updating instance moderation status.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstanceInput {
    pub host: String,
    #[serde(default)]
    pub is_blocked: Option<bool>,
    #[serde(default)]
    pub is_silenced: Option<bool>,
    #[serde(default)]
    pub is_suspended: Option<bool>,
    #[serde(default)]
    pub moderation_note: Option<String>,
}

/// Instance service for federation management.
#[derive(Clone)]
pub struct InstanceService {
    instance_repo: InstanceRepository,
    user_repo: UserRepository,
}

impl InstanceService {
    /// Create a new instance service.
    #[must_use]
    pub fn new(instance_repo: InstanceRepository, user_repo: UserRepository) -> Self {
        Self {
            instance_repo,
            user_repo,
        }
    }

    // ========== Query Methods ==========

    /// Get an instance by hostname.
    pub async fn get_by_host(&self, host: &str) -> AppResult<instance::Model> {
        self.instance_repo.get_by_host(host).await
    }

    /// Find an instance by hostname.
    pub async fn find_by_host(&self, host: &str) -> AppResult<Option<instance::Model>> {
        self.instance_repo.find_by_host(host).await
    }

    /// Find or create an instance by hostname.
    pub async fn find_or_create(&self, host: &str) -> AppResult<instance::Model> {
        self.instance_repo.find_or_create(host).await
    }

    /// List all instances with pagination.
    pub async fn list_all(
        &self,
        limit: u64,
        offset: u64,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> AppResult<Vec<instance::Model>> {
        self.instance_repo
            .find_all(limit, offset, sort_by, sort_order)
            .await
    }

    /// Search instances by hostname.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<instance::Model>> {
        self.instance_repo.search(query, limit, offset).await
    }

    /// List blocked instances.
    pub async fn list_blocked(&self, limit: u64, offset: u64) -> AppResult<Vec<instance::Model>> {
        self.instance_repo.find_blocked(limit, offset).await
    }

    /// List silenced instances.
    pub async fn list_silenced(&self, limit: u64, offset: u64) -> AppResult<Vec<instance::Model>> {
        self.instance_repo.find_silenced(limit, offset).await
    }

    /// List suspended instances.
    pub async fn list_suspended(&self, limit: u64, offset: u64) -> AppResult<Vec<instance::Model>> {
        self.instance_repo.find_suspended(limit, offset).await
    }

    /// Get federation statistics.
    pub async fn get_stats(&self) -> AppResult<InstanceStats> {
        self.instance_repo.get_stats().await
    }

    // ========== Check Methods ==========

    /// Check if an instance is blocked.
    pub async fn is_blocked(&self, host: &str) -> AppResult<bool> {
        self.instance_repo.is_blocked(host).await
    }

    /// Check if an instance is silenced.
    pub async fn is_silenced(&self, host: &str) -> AppResult<bool> {
        self.instance_repo.is_silenced(host).await
    }

    /// Check if federation should be allowed with this instance.
    /// Returns false if the instance is blocked.
    pub async fn should_federate(&self, host: &str) -> AppResult<bool> {
        Ok(!self.is_blocked(host).await?)
    }

    /// Check if content from this instance should appear in public timelines.
    /// Returns false if the instance is silenced or blocked.
    pub async fn should_show_in_public(&self, host: &str) -> AppResult<bool> {
        let instance = self.instance_repo.find_by_host(host).await?;
        match instance {
            Some(i) => Ok(!i.is_blocked && !i.is_silenced),
            None => Ok(true), // Unknown instances are allowed by default
        }
    }

    // ========== Moderation Methods ==========

    /// Update instance moderation status (admin only).
    pub async fn update_instance(
        &self,
        moderator_id: &str,
        input: UpdateInstanceInput,
    ) -> AppResult<instance::Model> {
        // Verify moderator is admin
        let moderator = self.user_repo.get_by_id(moderator_id).await?;
        if !moderator.is_admin && !moderator.is_moderator {
            return Err(AppError::Forbidden(
                "Only admins can manage instances".to_string(),
            ));
        }

        // Validate hostname
        let host = input.host.trim().to_lowercase();
        if host.is_empty() {
            return Err(AppError::BadRequest("Host is required".to_string()));
        }

        // Find or create the instance
        let instance = self.instance_repo.find_or_create(&host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            is_blocked: Set(input.is_blocked.unwrap_or(instance.is_blocked)),
            is_silenced: Set(input.is_silenced.unwrap_or(instance.is_silenced)),
            is_suspended: Set(input.is_suspended.unwrap_or(instance.is_suspended)),
            moderation_note: Set(input.moderation_note.or(instance.moderation_note)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.instance_repo.update(model).await
    }

    /// Block an instance (admin only).
    pub async fn block_instance(
        &self,
        moderator_id: &str,
        host: &str,
    ) -> AppResult<instance::Model> {
        self.update_instance(
            moderator_id,
            UpdateInstanceInput {
                host: host.to_string(),
                is_blocked: Some(true),
                is_silenced: None,
                is_suspended: None,
                moderation_note: None,
            },
        )
        .await
    }

    /// Unblock an instance (admin only).
    pub async fn unblock_instance(
        &self,
        moderator_id: &str,
        host: &str,
    ) -> AppResult<instance::Model> {
        self.update_instance(
            moderator_id,
            UpdateInstanceInput {
                host: host.to_string(),
                is_blocked: Some(false),
                is_silenced: None,
                is_suspended: None,
                moderation_note: None,
            },
        )
        .await
    }

    /// Silence an instance (admin only).
    pub async fn silence_instance(
        &self,
        moderator_id: &str,
        host: &str,
    ) -> AppResult<instance::Model> {
        self.update_instance(
            moderator_id,
            UpdateInstanceInput {
                host: host.to_string(),
                is_blocked: None,
                is_silenced: Some(true),
                is_suspended: None,
                moderation_note: None,
            },
        )
        .await
    }

    /// Unsilence an instance (admin only).
    pub async fn unsilence_instance(
        &self,
        moderator_id: &str,
        host: &str,
    ) -> AppResult<instance::Model> {
        self.update_instance(
            moderator_id,
            UpdateInstanceInput {
                host: host.to_string(),
                is_blocked: None,
                is_silenced: Some(false),
                is_suspended: None,
                moderation_note: None,
            },
        )
        .await
    }

    // ========== Federation Event Methods ==========

    /// Record that we received something from an instance.
    pub async fn touch_communication(&self, host: &str) -> AppResult<()> {
        self.instance_repo.touch_last_communicated(host).await
    }

    /// Increment user count when we discover a new user from this instance.
    pub async fn register_user(&self, host: &str) -> AppResult<()> {
        self.instance_repo.increment_users_count(host).await
    }

    /// Increment note count when we receive a new note from this instance.
    pub async fn register_note(&self, host: &str) -> AppResult<()> {
        self.instance_repo.increment_notes_count(host).await
    }

    /// Update instance info from nodeinfo.
    pub async fn update_nodeinfo(
        &self,
        host: &str,
        software_name: Option<String>,
        software_version: Option<String>,
        name: Option<String>,
        description: Option<String>,
        maintainer_email: Option<String>,
        maintainer_name: Option<String>,
        icon_url: Option<String>,
        theme_color: Option<String>,
    ) -> AppResult<instance::Model> {
        self.instance_repo
            .update_info(
                host,
                software_name,
                software_version,
                name,
                description,
                maintainer_email,
                maintainer_name,
                icon_url,
                theme_color,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_instance_input() {
        let input = UpdateInstanceInput {
            host: "example.com".to_string(),
            is_blocked: Some(true),
            is_silenced: None,
            is_suspended: None,
            moderation_note: Some("Spam instance".to_string()),
        };
        assert_eq!(input.host, "example.com");
        assert_eq!(input.is_blocked, Some(true));
    }
}
