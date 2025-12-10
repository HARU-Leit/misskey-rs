//! Scheduled note entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Visibility of a scheduled note.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ScheduledVisibility {
    #[sea_orm(string_value = "public")]
    Public,
    #[sea_orm(string_value = "home")]
    Home,
    #[sea_orm(string_value = "followers")]
    Followers,
    #[sea_orm(string_value = "specified")]
    Specified,
}

/// Scheduled note status.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum ScheduledStatus {
    /// Waiting to be posted.
    #[sea_orm(string_value = "pending")]
    Pending,
    /// Currently being posted.
    #[sea_orm(string_value = "processing")]
    Processing,
    /// Successfully posted.
    #[sea_orm(string_value = "posted")]
    Posted,
    /// Failed to post.
    #[sea_orm(string_value = "failed")]
    Failed,
    /// Cancelled by user.
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}

/// A note scheduled for future posting.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduled_note")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created this scheduled note.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// The text content of the note.
    #[sea_orm(column_type = "Text", nullable)]
    pub text: Option<String>,

    /// Content warning.
    #[sea_orm(nullable)]
    pub cw: Option<String>,

    /// Visibility of the note.
    pub visibility: ScheduledVisibility,

    /// Users who can see the note (for specified visibility).
    #[sea_orm(column_type = "JsonBinary")]
    pub visible_user_ids: Json,

    /// Attached file IDs.
    #[sea_orm(column_type = "JsonBinary")]
    pub file_ids: Json,

    /// ID of the note this is replying to.
    #[sea_orm(nullable)]
    pub reply_id: Option<String>,

    /// ID of the note this is renoting.
    #[sea_orm(nullable)]
    pub renote_id: Option<String>,

    /// Poll data (JSON).
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub poll: Option<Json>,

    /// Scheduled time for posting.
    #[sea_orm(indexed)]
    pub scheduled_at: DateTimeWithTimeZone,

    /// Current status.
    pub status: ScheduledStatus,

    /// ID of the posted note (after successful posting).
    #[sea_orm(nullable)]
    pub posted_note_id: Option<String>,

    /// Error message if posting failed.
    #[sea_orm(nullable)]
    pub error_message: Option<String>,

    /// Number of retry attempts.
    #[sea_orm(default_value = 0)]
    pub retry_count: i32,

    /// When this scheduled note was created.
    pub created_at: DateTimeWithTimeZone,

    /// When this scheduled note was last updated.
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
        belongs_to = "super::note::Entity",
        from = "Column::ReplyId",
        to = "super::note::Column::Id",
        on_delete = "SetNull"
    )]
    Reply,
    #[sea_orm(
        belongs_to = "super::note::Entity",
        from = "Column::RenoteId",
        to = "super::note::Column::Id",
        on_delete = "SetNull"
    )]
    Renote,
    #[sea_orm(
        belongs_to = "super::note::Entity",
        from = "Column::PostedNoteId",
        to = "super::note::Column::Id",
        on_delete = "SetNull"
    )]
    PostedNote,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
