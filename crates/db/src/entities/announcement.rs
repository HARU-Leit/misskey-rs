//! Announcement entity.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Announcement model for instance-wide notices.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "announcement")]
pub struct Model {
    /// Unique announcement ID.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Title of the announcement.
    pub title: String,

    /// Content/body of the announcement (MFM supported).
    #[sea_orm(column_type = "Text")]
    pub text: String,

    /// Image URL for the announcement (optional).
    #[sea_orm(nullable)]
    pub image_url: Option<String>,

    /// Whether the announcement is currently active/visible.
    pub is_active: bool,

    /// Whether users must acknowledge/read the announcement.
    pub needs_confirmation_to_read: bool,

    /// Display order (lower = higher priority).
    pub display_order: i32,

    /// Icon to display with the announcement.
    #[sea_orm(nullable)]
    pub icon: Option<String>,

    /// Foreground color for the announcement.
    #[sea_orm(nullable)]
    pub foreground_color: Option<String>,

    /// Background color for the announcement.
    #[sea_orm(nullable)]
    pub background_color: Option<String>,

    /// When to start showing the announcement (optional).
    #[sea_orm(nullable)]
    pub starts_at: Option<DateTime<Utc>>,

    /// When to stop showing the announcement (optional).
    #[sea_orm(nullable)]
    pub ends_at: Option<DateTime<Utc>>,

    /// How many users have read this announcement.
    pub reads_count: i32,

    /// When the announcement was created.
    pub created_at: DateTime<Utc>,

    /// When the announcement was last updated.
    #[sea_orm(nullable)]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Relationships.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::announcement_read::Entity")]
    AnnouncementReads,
}

impl Related<super::announcement_read::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AnnouncementReads.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
