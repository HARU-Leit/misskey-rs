//! Abuse report entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Abuse report status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[derive(Default)]
pub enum ReportStatus {
    #[sea_orm(string_value = "pending")]
    #[default]
    Pending,
    #[sea_orm(string_value = "resolved")]
    Resolved,
    #[sea_orm(string_value = "rejected")]
    Rejected,
}


/// Abuse report model.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "abuse_report")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// The user who submitted the report.
    pub reporter_id: String,
    /// The user being reported.
    pub target_user_id: String,
    /// Optional note being reported.
    pub target_note_id: Option<String>,
    /// Reason for the report.
    pub comment: String,
    /// Current status of the report.
    pub status: ReportStatus,
    /// Admin who handled the report.
    pub assignee_id: Option<String>,
    /// Resolution comment by admin.
    pub resolution_comment: Option<String>,
    /// When the report was created.
    pub created_at: DateTimeWithTimeZone,
    /// When the report was resolved.
    pub resolved_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
