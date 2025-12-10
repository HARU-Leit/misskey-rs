//! Notification entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Notification types.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum NotificationType {
    #[sea_orm(string_value = "follow")]
    Follow,
    #[sea_orm(string_value = "mention")]
    Mention,
    #[sea_orm(string_value = "reply")]
    Reply,
    #[sea_orm(string_value = "renote")]
    Renote,
    #[sea_orm(string_value = "quote")]
    Quote,
    #[sea_orm(string_value = "reaction")]
    Reaction,
    #[sea_orm(string_value = "pollEnded")]
    PollEnded,
    #[sea_orm(string_value = "receiveFollowRequest")]
    ReceiveFollowRequest,
    #[sea_orm(string_value = "followRequestAccepted")]
    FollowRequestAccepted,
    #[sea_orm(string_value = "app")]
    App,
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notification")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The user receiving the notification
    pub notifiee_id: String,

    /// The user who triggered the notification (optional for some types)
    #[sea_orm(nullable)]
    pub notifier_id: Option<String>,

    /// Notification type
    pub notification_type: NotificationType,

    /// Related note ID (for mention, reply, renote, quote, reaction)
    #[sea_orm(nullable)]
    pub note_id: Option<String>,

    /// Related follow request ID
    #[sea_orm(nullable)]
    pub follow_request_id: Option<String>,

    /// Reaction emoji (for reaction notifications)
    #[sea_orm(nullable)]
    pub reaction: Option<String>,

    /// App-specific data
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub custom_data: Option<Json>,

    /// Has this notification been read?
    #[sea_orm(default_value = false)]
    pub is_read: bool,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::NotifieeId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Notifiee,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::NotifierId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Notifier,

    #[sea_orm(
        belongs_to = "super::note::Entity",
        from = "Column::NoteId",
        to = "super::note::Column::Id",
        on_delete = "Cascade"
    )]
    Note,
}

impl ActiveModelBehavior for ActiveModel {}
