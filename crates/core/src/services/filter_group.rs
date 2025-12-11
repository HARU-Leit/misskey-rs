//! Filter group service for organizing word filters into presets.

use misskey_common::{AppError, AppResult};
use misskey_db::entities::filter_group;
use misskey_db::entities::word_filter;
use misskey_db::repositories::{
    CreateFilterGroupInput, FilterGroupRepository, UpdateFilterGroupInput,
};
use serde::Deserialize;
use validator::Validate;

/// Maximum number of filter groups per user.
const MAX_FILTER_GROUPS_PER_USER: u64 = 20;

/// Input for creating a filter group.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupInput {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(max = 500))]
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

const fn default_true() -> bool {
    true
}

/// Input for updating a filter group.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGroupInput {
    pub id: String,
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[validate(length(max = 500))]
    pub description: Option<Option<String>>,
    pub is_active: Option<bool>,
}

/// Service for managing filter groups.
#[derive(Clone)]
pub struct FilterGroupService {
    filter_group_repo: FilterGroupRepository,
}

impl FilterGroupService {
    /// Create a new filter group service.
    #[must_use]
    pub const fn new(filter_group_repo: FilterGroupRepository) -> Self {
        Self { filter_group_repo }
    }

    /// Get a filter group by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<filter_group::Model>> {
        self.filter_group_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a filter group by ID with ownership check.
    pub async fn get_by_id_for_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<filter_group::Model> {
        let group = self
            .filter_group_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Filter group not found".to_string()))?;

        if group.user_id != user_id {
            return Err(AppError::Forbidden(
                "Not the owner of this filter group".to_string(),
            ));
        }

        Ok(group)
    }

    /// List filter groups for a user.
    pub async fn list_groups(&self, user_id: &str) -> AppResult<Vec<filter_group::Model>> {
        self.filter_group_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List active filter groups for a user.
    pub async fn list_active_groups(&self, user_id: &str) -> AppResult<Vec<filter_group::Model>> {
        self.filter_group_repo
            .find_active_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count filter groups for a user.
    pub async fn count_groups(&self, user_id: &str) -> AppResult<u64> {
        self.filter_group_repo
            .count_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new filter group.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateGroupInput,
    ) -> AppResult<filter_group::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check group limit
        let count = self
            .filter_group_repo
            .count_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        if count >= MAX_FILTER_GROUPS_PER_USER {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_FILTER_GROUPS_PER_USER} filter groups allowed"
            )));
        }

        let db_input = CreateFilterGroupInput {
            user_id: user_id.to_string(),
            name: input.name,
            description: input.description,
            is_active: input.is_active,
        };

        self.filter_group_repo
            .create(db_input)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a filter group.
    pub async fn update(
        &self,
        user_id: &str,
        input: UpdateGroupInput,
    ) -> AppResult<filter_group::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Verify ownership
        self.get_by_id_for_user(&input.id, user_id).await?;

        let db_input = UpdateFilterGroupInput {
            name: input.name,
            description: input.description,
            is_active: input.is_active,
            display_order: None,
        };

        self.filter_group_repo
            .update(&input.id, db_input)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Filter group not found".to_string()))
    }

    /// Activate a filter group.
    pub async fn activate(&self, id: &str, user_id: &str) -> AppResult<filter_group::Model> {
        // Verify ownership
        self.get_by_id_for_user(id, user_id).await?;

        self.filter_group_repo
            .activate(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Filter group not found".to_string()))
    }

    /// Deactivate a filter group.
    pub async fn deactivate(&self, id: &str, user_id: &str) -> AppResult<filter_group::Model> {
        // Verify ownership
        self.get_by_id_for_user(id, user_id).await?;

        self.filter_group_repo
            .deactivate(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Filter group not found".to_string()))
    }

    /// Delete a filter group.
    pub async fn delete(&self, id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        self.get_by_id_for_user(id, user_id).await?;

        self.filter_group_repo
            .delete(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Reorder filter groups.
    pub async fn reorder(&self, user_id: &str, group_ids: Vec<String>) -> AppResult<()> {
        self.filter_group_repo
            .reorder(user_id, group_ids)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get filters in a group.
    pub async fn get_filters(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<word_filter::Model>> {
        // Verify ownership
        self.get_by_id_for_user(group_id, user_id).await?;

        self.filter_group_repo
            .get_filters(group_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get ungrouped filters for a user.
    pub async fn get_ungrouped_filters(&self, user_id: &str) -> AppResult<Vec<word_filter::Model>> {
        self.filter_group_repo
            .get_ungrouped_filters(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Add a filter to a group.
    pub async fn add_filter_to_group(
        &self,
        filter_id: &str,
        group_id: &str,
        user_id: &str,
    ) -> AppResult<word_filter::Model> {
        // Verify group ownership
        self.get_by_id_for_user(group_id, user_id).await?;

        self.filter_group_repo
            .add_filter_to_group(filter_id, group_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Filter not found".to_string()))
    }

    /// Remove a filter from its group.
    pub async fn remove_filter_from_group(
        &self,
        filter_id: &str,
        user_id: &str,
    ) -> AppResult<word_filter::Model> {
        // Verify filter belongs to user (by getting current group and checking)
        let filter = self
            .filter_group_repo
            .remove_filter_from_group(filter_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Filter not found".to_string()))?;

        // Verify ownership
        if filter.user_id != user_id {
            return Err(AppError::Forbidden(
                "Not the owner of this filter".to_string(),
            ));
        }

        Ok(filter)
    }

    /// Move multiple filters to a group.
    pub async fn move_filters_to_group(
        &self,
        filter_ids: Vec<String>,
        group_id: &str,
        user_id: &str,
    ) -> AppResult<u64> {
        // Verify group ownership
        self.get_by_id_for_user(group_id, user_id).await?;

        self.filter_group_repo
            .move_filters_to_group(&filter_ids, group_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
