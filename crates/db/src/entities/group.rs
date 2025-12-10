//! Group entity for user communities.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Group join policy type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum GroupJoinPolicy {
    /// Anyone can join without approval.
    #[sea_orm(string_value = "open")]
    Open,
    /// Users need an invitation to join.
    #[sea_orm(string_value = "invite_only")]
    InviteOnly,
    /// Users can request to join, owner/moderators approve.
    #[sea_orm(string_value = "approval")]
    Approval,
}

impl Default for GroupJoinPolicy {
    fn default() -> Self {
        Self::InviteOnly
    }
}

/// Group entity - a community for users to share notes.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "group")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who owns the group.
    #[sea_orm(indexed)]
    pub owner_id: String,

    /// Group name.
    pub name: String,

    /// Group description (optional).
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// Banner image ID (optional).
    #[sea_orm(nullable)]
    pub banner_id: Option<String>,

    /// Avatar image ID (optional).
    #[sea_orm(nullable)]
    pub avatar_id: Option<String>,

    /// Join policy for the group.
    pub join_policy: GroupJoinPolicy,

    /// Whether the group is archived (soft deleted).
    #[sea_orm(default_value = false)]
    pub is_archived: bool,

    /// Whether the group is discoverable in search.
    #[sea_orm(default_value = true)]
    pub is_searchable: bool,

    /// Whether only members can post to group.
    #[sea_orm(default_value = true)]
    pub members_only_post: bool,

    /// Number of members (denormalized).
    #[sea_orm(default_value = 1)]
    pub members_count: i64,

    /// Number of notes in this group (denormalized).
    #[sea_orm(default_value = 0)]
    pub notes_count: i64,

    /// Group rules/guidelines (optional).
    #[sea_orm(column_type = "Text", nullable)]
    pub rules: Option<String>,

    /// Custom fields (JSON) for additional metadata.
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub metadata: Option<serde_json::Value>,

    /// When the group was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the group was last updated.
    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Owner,
    #[sea_orm(
        belongs_to = "super::drive_file::Entity",
        from = "Column::BannerId",
        to = "super::drive_file::Column::Id",
        on_delete = "SetNull"
    )]
    Banner,
    #[sea_orm(
        belongs_to = "super::drive_file::Entity",
        from = "Column::AvatarId",
        to = "super::drive_file::Column::Id",
        on_delete = "SetNull"
    )]
    Avatar,
    #[sea_orm(has_many = "super::group_member::Entity")]
    Members,
    #[sea_orm(has_many = "super::group_invite::Entity")]
    Invites,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl Related<super::group_member::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Members.def()
    }
}

impl Related<super::group_invite::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invites.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
