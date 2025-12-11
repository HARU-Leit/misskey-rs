//! Registration approval entity for manual account approval workflow.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Approval status for registration requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[derive(Default)]
pub enum ApprovalStatus {
    #[sea_orm(string_value = "pending")]
    #[default]
    Pending,
    #[sea_orm(string_value = "approved")]
    Approved,
    #[sea_orm(string_value = "rejected")]
    Rejected,
}

/// Registration approval request.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "registration_approval")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User ID of the account pending approval
    #[sea_orm(unique)]
    pub user_id: String,

    /// Reason provided by user for registration (optional)
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,

    /// Current approval status
    pub status: ApprovalStatus,

    /// Admin who reviewed the request
    #[sea_orm(nullable)]
    pub reviewed_by: Option<String>,

    /// Note from reviewer (optional, e.g., rejection reason)
    #[sea_orm(column_type = "Text", nullable)]
    pub review_note: Option<String>,

    /// When the registration was submitted
    pub created_at: DateTimeWithTimeZone,

    /// When the request was reviewed
    #[sea_orm(nullable)]
    pub reviewed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ReviewedBy",
        to = "super::user::Column::Id"
    )]
    Reviewer,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
