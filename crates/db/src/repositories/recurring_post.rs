//! Repository for recurring post operations.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use misskey_common::id::IdGenerator;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};

use crate::entities::recurring_post::{
    ActiveModel, Column, Entity, Model, RecurringInterval, RecurringVisibility,
};

/// Input for creating a recurring post.
#[derive(Debug, Clone)]
pub struct CreateRecurringPostInput {
    pub user_id: String,
    pub text: Option<String>,
    pub cw: Option<String>,
    pub visibility: RecurringVisibility,
    pub local_only: bool,
    pub file_ids: Vec<String>,
    pub interval: RecurringInterval,
    pub day_of_week: Option<i16>,
    pub day_of_month: Option<i16>,
    pub hour_utc: i16,
    pub minute_utc: i16,
    pub timezone: String,
    pub max_posts: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Input for updating a recurring post.
#[derive(Debug, Clone, Default)]
pub struct UpdateRecurringPostInput {
    pub text: Option<Option<String>>,
    pub cw: Option<Option<String>>,
    pub visibility: Option<RecurringVisibility>,
    pub local_only: Option<bool>,
    pub file_ids: Option<Vec<String>>,
    pub interval: Option<RecurringInterval>,
    pub day_of_week: Option<Option<i16>>,
    pub day_of_month: Option<Option<i16>>,
    pub hour_utc: Option<i16>,
    pub minute_utc: Option<i16>,
    pub timezone: Option<String>,
    pub is_active: Option<bool>,
    pub max_posts: Option<Option<i32>>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// Repository for recurring post operations.
#[derive(Clone)]
pub struct RecurringPostRepository {
    db: Arc<DatabaseConnection>,
    id_gen: IdGenerator,
}

impl RecurringPostRepository {
    /// Create a new recurring post repository.
    #[must_use]
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new recurring post.
    pub async fn create(&self, input: CreateRecurringPostInput) -> Result<Model, DbErr> {
        let id = self.id_gen.generate();
        let now = Utc::now().fixed_offset();

        let model = ActiveModel {
            id: Set(id),
            user_id: Set(input.user_id),
            text: Set(input.text),
            cw: Set(input.cw),
            visibility: Set(input.visibility),
            local_only: Set(input.local_only),
            file_ids: Set(serde_json::json!(input.file_ids)),
            interval: Set(input.interval),
            day_of_week: Set(input.day_of_week),
            day_of_month: Set(input.day_of_month),
            hour_utc: Set(input.hour_utc),
            minute_utc: Set(input.minute_utc),
            timezone: Set(input.timezone),
            is_active: Set(true),
            last_posted_at: Set(None),
            next_post_at: Set(None), // Will be calculated by service
            post_count: Set(0),
            max_posts: Set(input.max_posts),
            expires_at: Set(input.expires_at.map(|dt| dt.fixed_offset())),
            created_at: Set(now),
            updated_at: Set(None),
        };

        model.insert(self.db.as_ref()).await
    }

    /// Get a recurring post by ID.
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Model>, DbErr> {
        Entity::find_by_id(id).one(self.db.as_ref()).await
    }

    /// Get all recurring posts for a user.
    pub async fn find_by_user(&self, user_id: &str) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_desc(Column::CreatedAt)
            .all(self.db.as_ref())
            .await
    }

    /// Get active recurring posts for a user.
    pub async fn find_active_by_user(&self, user_id: &str) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::IsActive.eq(true))
            .order_by_desc(Column::CreatedAt)
            .all(self.db.as_ref())
            .await
    }

    /// Get recurring posts that are due for execution.
    pub async fn find_due_posts(&self, before: DateTime<Utc>) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::IsActive.eq(true))
            .filter(Column::NextPostAt.lte(before.fixed_offset()))
            .order_by_asc(Column::NextPostAt)
            .all(self.db.as_ref())
            .await
    }

    /// Update a recurring post.
    pub async fn update(
        &self,
        id: &str,
        input: UpdateRecurringPostInput,
    ) -> Result<Option<Model>, DbErr> {
        let Some(existing) = Entity::find_by_id(id).one(self.db.as_ref()).await? else {
            return Ok(None);
        };

        let now = Utc::now().fixed_offset();

        let mut model: ActiveModel = existing.into();
        model.updated_at = Set(Some(now));

        if let Some(text) = input.text {
            model.text = Set(text);
        }
        if let Some(cw) = input.cw {
            model.cw = Set(cw);
        }
        if let Some(visibility) = input.visibility {
            model.visibility = Set(visibility);
        }
        if let Some(local_only) = input.local_only {
            model.local_only = Set(local_only);
        }
        if let Some(file_ids) = input.file_ids {
            model.file_ids = Set(serde_json::json!(file_ids));
        }
        if let Some(interval) = input.interval {
            model.interval = Set(interval);
        }
        if let Some(day_of_week) = input.day_of_week {
            model.day_of_week = Set(day_of_week);
        }
        if let Some(day_of_month) = input.day_of_month {
            model.day_of_month = Set(day_of_month);
        }
        if let Some(hour_utc) = input.hour_utc {
            model.hour_utc = Set(hour_utc);
        }
        if let Some(minute_utc) = input.minute_utc {
            model.minute_utc = Set(minute_utc);
        }
        if let Some(timezone) = input.timezone {
            model.timezone = Set(timezone);
        }
        if let Some(is_active) = input.is_active {
            model.is_active = Set(is_active);
        }
        if let Some(max_posts) = input.max_posts {
            model.max_posts = Set(max_posts);
        }
        if let Some(expires_at) = input.expires_at {
            model.expires_at = Set(expires_at.map(|dt| dt.fixed_offset()));
        }

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Update the next post time.
    pub async fn update_next_post_at(
        &self,
        id: &str,
        next_post_at: Option<DateTime<Utc>>,
    ) -> Result<Option<Model>, DbErr> {
        let Some(existing) = Entity::find_by_id(id).one(self.db.as_ref()).await? else {
            return Ok(None);
        };

        let mut model: ActiveModel = existing.into();
        model.next_post_at = Set(next_post_at.map(|dt| dt.fixed_offset()));
        model.updated_at = Set(Some(Utc::now().fixed_offset()));

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Record that a post was executed.
    pub async fn record_post_execution(&self, id: &str) -> Result<Option<Model>, DbErr> {
        let Some(existing) = Entity::find_by_id(id).one(self.db.as_ref()).await? else {
            return Ok(None);
        };

        let now = Utc::now().fixed_offset();

        let mut model: ActiveModel = existing.clone().into();
        model.last_posted_at = Set(Some(now));
        model.post_count = Set(existing.post_count + 1);
        model.updated_at = Set(Some(now));

        // Check if max posts reached
        if let Some(max_posts) = existing.max_posts
            && existing.post_count + 1 >= max_posts {
                model.is_active = Set(false);
            }

        // Check if expired
        if let Some(expires_at) = existing.expires_at
            && now >= expires_at {
                model.is_active = Set(false);
            }

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Deactivate a recurring post.
    pub async fn deactivate(&self, id: &str) -> Result<Option<Model>, DbErr> {
        let Some(existing) = Entity::find_by_id(id).one(self.db.as_ref()).await? else {
            return Ok(None);
        };

        let mut model: ActiveModel = existing.into();
        model.is_active = Set(false);
        model.updated_at = Set(Some(Utc::now().fixed_offset()));

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Activate a recurring post.
    pub async fn activate(&self, id: &str) -> Result<Option<Model>, DbErr> {
        let Some(existing) = Entity::find_by_id(id).one(self.db.as_ref()).await? else {
            return Ok(None);
        };

        let mut model: ActiveModel = existing.into();
        model.is_active = Set(true);
        model.updated_at = Set(Some(Utc::now().fixed_offset()));

        model.update(self.db.as_ref()).await.map(Some)
    }

    /// Delete a recurring post.
    pub async fn delete(&self, id: &str) -> Result<bool, DbErr> {
        let result = Entity::delete_by_id(id).exec(self.db.as_ref()).await?;
        Ok(result.rows_affected > 0)
    }

    /// Delete all recurring posts for a user.
    pub async fn delete_by_user(&self, user_id: &str) -> Result<u64, DbErr> {
        let result = Entity::delete_many()
            .filter(Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await?;
        Ok(result.rows_affected)
    }

    /// Count recurring posts for a user.
    pub async fn count_by_user(&self, user_id: &str) -> Result<u64, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .count(self.db.as_ref())
            .await
    }

    /// Count active recurring posts for a user.
    pub async fn count_active_by_user(&self, user_id: &str) -> Result<u64, DbErr> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::IsActive.eq(true))
            .count(self.db.as_ref())
            .await
    }
}
