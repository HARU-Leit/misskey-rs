//! Webhook entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Webhook event types.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(64))")]
pub enum WebhookEvent {
    #[sea_orm(string_value = "note")]
    Note,
    #[sea_orm(string_value = "reply")]
    Reply,
    #[sea_orm(string_value = "renote")]
    Renote,
    #[sea_orm(string_value = "mention")]
    Mention,
    #[sea_orm(string_value = "follow")]
    Follow,
    #[sea_orm(string_value = "followed")]
    Followed,
    #[sea_orm(string_value = "unfollow")]
    Unfollow,
    #[sea_orm(string_value = "reaction")]
    Reaction,
}

/// Webhook model.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook")]
pub struct Model {
    /// Unique identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who owns this webhook.
    pub user_id: String,

    /// Webhook name for display.
    pub name: String,

    /// Target URL to send events to.
    #[sea_orm(column_type = "Text")]
    pub url: String,

    /// Secret for signing webhook payloads.
    pub secret: String,

    /// Events this webhook is subscribed to (JSON array).
    #[sea_orm(column_type = "JsonBinary")]
    pub events: Json,

    /// Is this webhook active?
    #[sea_orm(default_value = true)]
    pub is_active: bool,

    /// Last time this webhook was triggered.
    #[sea_orm(nullable)]
    pub last_triggered_at: Option<DateTimeWithTimeZone>,

    /// Count of failed delivery attempts.
    #[sea_orm(default_value = 0)]
    pub failure_count: i32,

    /// Last error message (if any).
    #[sea_orm(column_type = "Text", nullable)]
    pub last_error: Option<String>,

    /// When this webhook was created.
    pub created_at: DateTimeWithTimeZone,

    /// When this webhook was last updated.
    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
