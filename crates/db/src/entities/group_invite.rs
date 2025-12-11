//! Group invite entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Status of a group invitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[derive(Default)]
pub enum InviteStatus {
    /// Invitation is pending response.
    #[sea_orm(string_value = "pending")]
    #[default]
    Pending,
    /// Invitation was accepted.
    #[sea_orm(string_value = "accepted")]
    Accepted,
    /// Invitation was rejected by the invitee.
    #[sea_orm(string_value = "rejected")]
    Rejected,
    /// Invitation was cancelled by the inviter.
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
    /// Invitation expired.
    #[sea_orm(string_value = "expired")]
    Expired,
}

/// Type of invitation/join request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[derive(Default)]
pub enum InviteType {
    /// Group member invited the user.
    #[sea_orm(string_value = "invite")]
    #[default]
    Invite,
    /// User requested to join the group.
    #[sea_orm(string_value = "request")]
    Request,
}

/// Group invite - tracks invitations and join requests for groups.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "group_invite")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The group the invitation is for.
    #[sea_orm(indexed)]
    pub group_id: String,

    /// The user being invited (or requesting to join).
    #[sea_orm(indexed)]
    pub user_id: String,

    /// The user who sent the invitation (for invites) or null (for requests).
    #[sea_orm(indexed, nullable)]
    pub inviter_id: Option<String>,

    /// Type of invitation.
    pub invite_type: InviteType,

    /// Current status of the invitation.
    pub status: InviteStatus,

    /// Optional message with the invitation/request.
    #[sea_orm(column_type = "Text", nullable)]
    pub message: Option<String>,

    /// When the invitation expires (optional).
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeWithTimeZone>,

    /// When the invitation was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the status was last updated.
    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::group::Entity",
        from = "Column::GroupId",
        to = "super::group::Column::Id",
        on_delete = "Cascade"
    )]
    Group,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::InviterId",
        to = "super::user::Column::Id",
        on_delete = "SetNull"
    )]
    Inviter,
}

impl Related<super::group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
