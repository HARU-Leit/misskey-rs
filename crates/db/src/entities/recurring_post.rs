//! Recurring post entity for automatic repeated posting.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Interval type for recurring posts.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum RecurringInterval {
    #[sea_orm(string_value = "Daily")]
    Daily,
    #[sea_orm(string_value = "Weekly")]
    Weekly,
    #[sea_orm(string_value = "Monthly")]
    Monthly,
}

/// Visibility type for recurring posts.
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum RecurringVisibility {
    #[sea_orm(string_value = "Public")]
    Public,
    #[sea_orm(string_value = "Home")]
    Home,
    #[sea_orm(string_value = "Followers")]
    Followers,
    #[sea_orm(string_value = "Specified")]
    Specified,
}

/// Recurring post entity - configuration for automatic repeated posting.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "recurring_post")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created the recurring post.
    pub user_id: String,

    /// Post text content.
    pub text: Option<String>,

    /// Content warning.
    pub cw: Option<String>,

    /// Post visibility.
    pub visibility: RecurringVisibility,

    /// Whether to limit to local only.
    pub local_only: bool,

    /// Attached file IDs (JSON array).
    pub file_ids: Json,

    /// Posting interval.
    pub interval: RecurringInterval,

    /// Day of week for weekly posts (0=Sunday, 6=Saturday).
    pub day_of_week: Option<i16>,

    /// Day of month for monthly posts (1-31).
    pub day_of_month: Option<i16>,

    /// Hour to post (in UTC).
    pub hour_utc: i16,

    /// Minute to post.
    pub minute_utc: i16,

    /// User's timezone (IANA timezone name).
    pub timezone: String,

    /// Whether the recurring post is active.
    pub is_active: bool,

    /// When the post was last executed.
    pub last_posted_at: Option<DateTimeWithTimeZone>,

    /// When the post should next be executed.
    pub next_post_at: Option<DateTimeWithTimeZone>,

    /// Number of times this has been posted.
    pub post_count: i32,

    /// Maximum number of posts (null = unlimited).
    pub max_posts: Option<i32>,

    /// When the recurring post expires (null = never).
    pub expires_at: Option<DateTimeWithTimeZone>,

    /// When the recurring post was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the recurring post was last updated.
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
