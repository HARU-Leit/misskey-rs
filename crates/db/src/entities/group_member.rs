//! Group member entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Role of a group member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum GroupRole {
    /// Regular member.
    #[sea_orm(string_value = "member")]
    Member,
    /// Moderator - can manage members and content.
    #[sea_orm(string_value = "moderator")]
    Moderator,
    /// Admin - full management except ownership transfer.
    #[sea_orm(string_value = "admin")]
    Admin,
    /// Owner - full control including transfer and deletion.
    #[sea_orm(string_value = "owner")]
    Owner,
}

impl Default for GroupRole {
    fn default() -> Self {
        Self::Member
    }
}

impl GroupRole {
    /// Check if the role has moderation capabilities.
    pub fn can_moderate(&self) -> bool {
        matches!(self, Self::Moderator | Self::Admin | Self::Owner)
    }

    /// Check if the role can manage members (kick, promote).
    pub fn can_manage_members(&self) -> bool {
        matches!(self, Self::Admin | Self::Owner)
    }

    /// Check if the role can manage group settings.
    pub fn can_manage_settings(&self) -> bool {
        matches!(self, Self::Admin | Self::Owner)
    }

    /// Check if this is the owner role.
    pub fn is_owner(&self) -> bool {
        matches!(self, Self::Owner)
    }
}

/// Group member - tracks which users are in which groups.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "group_member")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The user who is a member.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// The group they belong to.
    #[sea_orm(indexed)]
    pub group_id: String,

    /// Role of the member in the group.
    pub role: GroupRole,

    /// Whether the user is muted within this group.
    #[sea_orm(default_value = false)]
    pub is_muted: bool,

    /// Whether the user is banned from this group.
    #[sea_orm(default_value = false)]
    pub is_banned: bool,

    /// Custom nickname in this group (optional).
    #[sea_orm(nullable)]
    pub nickname: Option<String>,

    /// When the user joined the group.
    pub joined_at: DateTimeWithTimeZone,

    /// When the member record was last updated.
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
    #[sea_orm(
        belongs_to = "super::group::Entity",
        from = "Column::GroupId",
        to = "super::group::Column::Id",
        on_delete = "Cascade"
    )]
    Group,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
