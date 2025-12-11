//! Import job entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Status of an import job.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ImportStatus {
    /// Job is queued for processing.
    #[sea_orm(string_value = "queued")]
    Queued,
    /// Job is validating data.
    #[sea_orm(string_value = "validating")]
    Validating,
    /// Job is currently being processed.
    #[sea_orm(string_value = "processing")]
    Processing,
    /// Job completed successfully.
    #[sea_orm(string_value = "completed")]
    Completed,
    /// Job completed with some errors.
    #[sea_orm(string_value = "partial")]
    PartiallyCompleted,
    /// Job failed.
    #[sea_orm(string_value = "failed")]
    Failed,
}

/// Data type being imported.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ImportDataType {
    /// Following list.
    #[sea_orm(string_value = "following")]
    Following,
    /// Muting list.
    #[sea_orm(string_value = "muting")]
    Muting,
    /// Blocking list.
    #[sea_orm(string_value = "blocking")]
    Blocking,
    /// User lists.
    #[sea_orm(string_value = "user_lists")]
    UserLists,
}

/// An import job.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "import_job")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created this import job.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Data type being imported.
    pub data_type: ImportDataType,

    /// Current status.
    pub status: ImportStatus,

    /// Progress (0-100).
    #[sea_orm(default_value = 0)]
    pub progress: i32,

    /// Total items to import.
    #[sea_orm(default_value = 0)]
    pub total_items: i32,

    /// Successfully imported items.
    #[sea_orm(default_value = 0)]
    pub imported_items: i32,

    /// Skipped items (duplicates, etc.).
    #[sea_orm(default_value = 0)]
    pub skipped_items: i32,

    /// Failed items.
    #[sea_orm(default_value = 0)]
    pub failed_items: i32,

    /// Error message if failed.
    #[sea_orm(nullable)]
    pub error_message: Option<String>,

    /// Detailed errors for individual items (JSON array).
    #[sea_orm(column_type = "JsonBinary")]
    pub item_errors: Json,

    /// When this job was created.
    pub created_at: DateTimeWithTimeZone,

    /// When this job completed.
    #[sea_orm(nullable)]
    pub completed_at: Option<DateTimeWithTimeZone>,
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
