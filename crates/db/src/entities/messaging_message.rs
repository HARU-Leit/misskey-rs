//! Messaging message entity for direct messages.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "messaging_message")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Sender user ID
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Recipient user ID (for 1:1 messages)
    #[sea_orm(nullable, indexed)]
    pub recipient_id: Option<String>,

    /// Group ID (for group messages, future use)
    #[sea_orm(nullable, indexed)]
    pub group_id: Option<String>,

    /// Message text content
    #[sea_orm(column_type = "Text", nullable)]
    pub text: Option<String>,

    /// Attached file ID
    #[sea_orm(nullable)]
    pub file_id: Option<String>,

    /// Has the recipient read this message?
    #[sea_orm(default_value = false)]
    pub is_read: bool,

    /// `ActivityPub` URI for federated messages
    #[sea_orm(nullable)]
    pub uri: Option<String>,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    Sender,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::RecipientId",
        to = "super::user::Column::Id"
    )]
    Recipient,

    #[sea_orm(
        belongs_to = "super::drive_file::Entity",
        from = "Column::FileId",
        to = "super::drive_file::Column::Id"
    )]
    File,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sender.def()
    }
}

impl Related<super::drive_file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::File.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
