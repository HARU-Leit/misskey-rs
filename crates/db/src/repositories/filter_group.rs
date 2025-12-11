//! Repository for filter group operations.

use std::sync::Arc;

use chrono::Utc;
use misskey_common::id::IdGenerator;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};

use crate::entities::filter_group::{ActiveModel, Column, Entity, Model};
use crate::entities::word_filter;

/// Input for creating a filter group.
#[derive(Debug, Clone)]
pub struct CreateFilterGroupInput {
    /// User ID.
    pub user_id: String,
    /// Group name.
    pub name: String,
    /// Group description.
    pub description: Option<String>,
    /// Initial active state.
    pub is_active: bool,
}

/// Input for updating a filter group.
#[derive(Debug, Clone, Default)]
pub struct UpdateFilterGroupInput {
    /// New name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<Option<String>>,
    /// New active state.
    pub is_active: Option<bool>,
    /// New display order.
    pub display_order: Option<i32>,
}

/// Repository for filter group operations.
#[derive(Clone)]
pub struct FilterGroupRepository {
    db: Arc<DatabaseConnection>,
    id_gen: IdGenerator,
}

impl FilterGroupRepository {
    /// Create a new filter group repository.
    #[must_use]
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new filter group.
    pub async fn create(&self, input: CreateFilterGroupInput) -> Result<Model, DbErr> {
        let id = self.id_gen.generate();
        let now = Utc::now().fixed_offset();

        // Get next display order
        let count = Entity::find()
            .filter(Column::UserId.eq(&input.user_id))
            .count(self.db.as_ref())
            .await?;

        let model = ActiveModel {
            id: Set(id),
            user_id: Set(input.user_id),
            name: Set(input.name),
            description: Set(input.description),
            is_active: Set(input.is_active),
            display_order: Set(count as i32),
            created_at: Set(now),
            updated_at: Set(None),
        };

        model.insert(self.db.as_ref()).await
    }

    /// Get a filter group by ID.
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Model>, DbErr> {
        Entity::find_by_id(id).one(self.db.as_ref()).await
    }

    /// Get all filter groups for a user.
    pub async fn find_by_user(&self, user_id: &str) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_asc(Column::DisplayOrder)
            .all(self.db.as_ref())
            .await
    }

    /// Get active filter groups for a user.
    pub async fn find_active_by_user(&self, user_id: &str) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::IsActive.eq(true))
            .order_by_asc(Column::DisplayOrder)
            .all(self.db.as_ref())
            .await
    }

    /// Update a filter group.
    pub async fn update(
        &self,
        id: &str,
        input: UpdateFilterGroupInput,
    ) -> Result<Option<Model>, DbErr> {
        let Some(existing) = Entity::find_by_id(id).one(self.db.as_ref()).await? else {
            return Ok(None);
        };

        let now = Utc::now().fixed_offset();

        let mut model: ActiveModel = existing.into();
        model.updated_at = Set(Some(now));

        if let Some(name) = input.name {
            model.name = Set(name);
        }
        if let Some(description) = input.description {
            model.description = Set(description);
        }
        if let Some(is_active) = input.is_active {
            model.is_active = Set(is_active);
        }
        if let Some(display_order) = input.display_order {
            model.display_order = Set(display_order);
        }

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Activate a filter group.
    pub async fn activate(&self, id: &str) -> Result<Option<Model>, DbErr> {
        self.update(id, UpdateFilterGroupInput {
            is_active: Some(true),
            ..Default::default()
        })
        .await
    }

    /// Deactivate a filter group.
    pub async fn deactivate(&self, id: &str) -> Result<Option<Model>, DbErr> {
        self.update(id, UpdateFilterGroupInput {
            is_active: Some(false),
            ..Default::default()
        })
        .await
    }

    /// Delete a filter group.
    /// Note: This will set `group_id` to NULL on associated word filters.
    pub async fn delete(&self, id: &str) -> Result<bool, DbErr> {
        let result = Entity::delete_by_id(id).exec(self.db.as_ref()).await?;
        Ok(result.rows_affected > 0)
    }

    /// Delete all filter groups for a user.
    pub async fn delete_by_user(&self, user_id: &str) -> Result<u64, DbErr> {
        let result = Entity::delete_many()
            .filter(Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await?;
        Ok(result.rows_affected)
    }

    /// Count filter groups for a user.
    pub async fn count_by_user(&self, user_id: &str) -> Result<u64, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
    }

    /// Reorder filter groups for a user.
    pub async fn reorder(&self, user_id: &str, group_ids: Vec<String>) -> Result<(), DbErr> {
        let now = Utc::now().fixed_offset();

        for (index, group_id) in group_ids.iter().enumerate() {
            // Verify the group belongs to the user before updating
            let Some(existing) = Entity::find_by_id(group_id).one(self.db.as_ref()).await? else {
                continue;
            };

            if existing.user_id != user_id {
                continue;
            }

            let mut model: ActiveModel = existing.into();
            model.display_order = Set(index as i32);
            model.updated_at = Set(Some(now));
            model.update(self.db.as_ref()).await?;
        }

        Ok(())
    }

    /// Get filters in a group.
    pub async fn get_filters(&self, group_id: &str) -> Result<Vec<word_filter::Model>, DbErr> {
        word_filter::Entity::find()
            .filter(word_filter::Column::GroupId.eq(group_id))
            .order_by_asc(word_filter::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
    }

    /// Add a filter to a group.
    pub async fn add_filter_to_group(
        &self,
        filter_id: &str,
        group_id: &str,
    ) -> Result<Option<word_filter::Model>, DbErr> {
        let Some(filter) = word_filter::Entity::find_by_id(filter_id)
            .one(self.db.as_ref())
            .await?
        else {
            return Ok(None);
        };

        let now = Utc::now().fixed_offset();
        let mut model: word_filter::ActiveModel = filter.into();
        model.group_id = Set(Some(group_id.to_string()));
        model.updated_at = Set(Some(now));

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Remove a filter from its group.
    pub async fn remove_filter_from_group(
        &self,
        filter_id: &str,
    ) -> Result<Option<word_filter::Model>, DbErr> {
        let Some(filter) = word_filter::Entity::find_by_id(filter_id)
            .one(self.db.as_ref())
            .await?
        else {
            return Ok(None);
        };

        let now = Utc::now().fixed_offset();
        let mut model: word_filter::ActiveModel = filter.into();
        model.group_id = Set(None);
        model.updated_at = Set(Some(now));

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Move multiple filters to a group.
    pub async fn move_filters_to_group(
        &self,
        filter_ids: &[String],
        group_id: &str,
    ) -> Result<u64, DbErr> {
        let now = Utc::now().fixed_offset();
        let mut count = 0u64;

        for filter_id in filter_ids {
            let Some(filter) = word_filter::Entity::find_by_id(filter_id)
                .one(self.db.as_ref())
                .await?
            else {
                continue;
            };

            let mut model: word_filter::ActiveModel = filter.into();
            model.group_id = Set(Some(group_id.to_string()));
            model.updated_at = Set(Some(now));
            model.update(self.db.as_ref()).await?;
            count += 1;
        }

        Ok(count)
    }

    /// Get ungrouped filters for a user.
    pub async fn get_ungrouped_filters(
        &self,
        user_id: &str,
    ) -> Result<Vec<word_filter::Model>, DbErr> {
        word_filter::Entity::find()
            .filter(word_filter::Column::UserId.eq(user_id))
            .filter(word_filter::Column::GroupId.is_null())
            .order_by_asc(word_filter::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
    }
}
