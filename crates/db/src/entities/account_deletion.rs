//! Account deletion entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Status of an account deletion.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum DeletionStatus {
    /// Deletion is scheduled.
    #[sea_orm(string_value = "scheduled")]
    Scheduled,
    /// Deletion is in progress.
    #[sea_orm(string_value = "in_progress")]
    InProgress,
    /// Deletion completed.
    #[sea_orm(string_value = "completed")]
    Completed,
    /// Deletion was cancelled.
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}

/// An account deletion record.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "account_deletion")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User ID being deleted.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Current status.
    pub status: DeletionStatus,

    /// Reason for deletion (optional).
    #[sea_orm(nullable)]
    pub reason: Option<String>,

    /// Whether this is a soft delete (hide) or hard delete.
    #[sea_orm(default_value = false)]
    pub soft_delete: bool,

    /// When deletion was scheduled.
    pub scheduled_at: DateTimeWithTimeZone,

    /// When deletion actually completed.
    #[sea_orm(nullable)]
    pub completed_at: Option<DateTimeWithTimeZone>,

    /// When this record was created.
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
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
