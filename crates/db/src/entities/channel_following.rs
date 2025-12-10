//! Channel following entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Channel following - tracks which users follow which channels.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "channel_following")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The follower user ID.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// The channel being followed.
    #[sea_orm(indexed)]
    pub channel_id: String,

    /// Whether the user has read the latest notes.
    #[sea_orm(default_value = false)]
    pub is_read: bool,

    /// When the follow was created.
    pub created_at: DateTimeWithTimeZone,
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
    #[sea_orm(
        belongs_to = "super::channel::Entity",
        from = "Column::ChannelId",
        to = "super::channel::Column::Id",
        on_delete = "Cascade"
    )]
    Channel,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Channel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
