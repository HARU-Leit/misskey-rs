//! Push subscription entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Push subscription entity for Web Push notifications.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "push_subscription")]
pub struct Model {
    /// Unique identifier
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User ID
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Push subscription endpoint URL
    #[sea_orm(column_type = "Text")]
    pub endpoint: String,

    /// Auth key for push subscription
    pub auth: String,

    /// P256DH key for push subscription
    pub p256dh: String,

    /// Notification types to receive (JSON array)
    #[sea_orm(column_type = "JsonBinary")]
    pub types: Json,

    /// Whether the subscription is active
    #[sea_orm(default_value = true)]
    pub active: bool,

    /// User agent of the device
    #[sea_orm(nullable)]
    pub user_agent: Option<String>,

    /// Device name (user-provided)
    #[sea_orm(nullable)]
    pub device_name: Option<String>,

    /// Quiet hours start (hour 0-23)
    #[sea_orm(nullable)]
    pub quiet_hours_start: Option<i32>,

    /// Quiet hours end (hour 0-23)
    #[sea_orm(nullable)]
    pub quiet_hours_end: Option<i32>,

    /// Last successful push timestamp
    #[sea_orm(nullable)]
    pub last_pushed_at: Option<DateTimeWithTimeZone>,

    /// Number of failed push attempts
    #[sea_orm(default_value = 0)]
    pub fail_count: i32,

    /// Timestamp when the subscription was created
    pub created_at: DateTimeWithTimeZone,

    /// Timestamp when the subscription was last updated
    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

/// Relations for push subscription.
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
