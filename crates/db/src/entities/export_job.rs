//! Export job entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Status of an export job.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ExportStatus {
    /// Job is queued for processing.
    #[sea_orm(string_value = "pending")]
    Pending,
    /// Job is currently being processed.
    #[sea_orm(string_value = "processing")]
    Processing,
    /// Job completed successfully.
    #[sea_orm(string_value = "completed")]
    Completed,
    /// Job failed.
    #[sea_orm(string_value = "failed")]
    Failed,
}

/// Export format.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ExportFormat {
    /// JSON format.
    #[sea_orm(string_value = "json")]
    Json,
    /// CSV format.
    #[sea_orm(string_value = "csv")]
    Csv,
    /// `ActivityPub` format.
    #[sea_orm(string_value = "activitypub")]
    ActivityPub,
}

/// An export job.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "export_job")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created this export job.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Data types to export (JSON array of strings).
    #[sea_orm(column_type = "JsonBinary")]
    pub data_types: Json,

    /// Export format.
    pub format: ExportFormat,

    /// Current status.
    pub status: ExportStatus,

    /// Progress (0-100).
    #[sea_orm(default_value = 0)]
    pub progress: i32,

    /// Error message if failed.
    #[sea_orm(nullable)]
    pub error_message: Option<String>,

    /// Path to the exported file.
    #[sea_orm(nullable)]
    pub file_path: Option<String>,

    /// Download URL (when completed).
    #[sea_orm(nullable)]
    pub download_url: Option<String>,

    /// When this job was created.
    pub created_at: DateTimeWithTimeZone,

    /// When this job completed.
    #[sea_orm(nullable)]
    pub completed_at: Option<DateTimeWithTimeZone>,

    /// When the download expires.
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeWithTimeZone>,
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
