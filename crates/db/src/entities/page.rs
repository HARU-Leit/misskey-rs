//! Page entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Page visibility levels.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum PageVisibility {
    #[sea_orm(string_value = "public")]
    Public,
    #[sea_orm(string_value = "followers")]
    Followers,
    #[sea_orm(string_value = "specified")]
    Specified,
}

impl Default for PageVisibility {
    fn default() -> Self {
        Self::Public
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "page")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Author user ID
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Page name (URL-safe identifier)
    #[sea_orm(indexed)]
    pub name: String,

    /// Page title
    pub title: String,

    /// Page summary (for SEO)
    #[sea_orm(column_type = "Text", nullable)]
    pub summary: Option<String>,

    /// Page content (JSON structure for Misskey page blocks)
    #[sea_orm(column_type = "JsonBinary")]
    pub content: Json,

    /// Page variables (for interactive pages)
    #[sea_orm(column_type = "JsonBinary")]
    pub variables: Json,

    /// Script (for interactive pages)
    #[sea_orm(column_type = "Text", nullable)]
    pub script: Option<String>,

    /// Visibility level
    pub visibility: PageVisibility,

    /// Users who can see this page (for visibility = specified)
    #[sea_orm(column_type = "JsonBinary")]
    pub visible_user_ids: Json,

    /// Eyecatch image ID
    #[sea_orm(nullable)]
    pub eyecatch_image_id: Option<String>,

    /// Attached file IDs
    #[sea_orm(column_type = "JsonBinary")]
    pub file_ids: Json,

    /// Font for the page
    #[sea_orm(nullable)]
    pub font: Option<String>,

    /// Whether to hide the title in display
    #[sea_orm(default_value = false)]
    pub hide_title_when_pinned: bool,

    /// Align center
    #[sea_orm(default_value = false)]
    pub align_center: bool,

    /// Is this page liked by users
    #[sea_orm(default_value = 0)]
    pub liked_count: i32,

    /// View count
    #[sea_orm(default_value = 0)]
    pub view_count: i32,

    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
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
        belongs_to = "super::drive_file::Entity",
        from = "Column::EyecatchImageId",
        to = "super::drive_file::Column::Id"
    )]
    EyecatchImage,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::drive_file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EyecatchImage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
