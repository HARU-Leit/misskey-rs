//! Drive file entity (uploaded files/media).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "drive_file")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Owner user ID
    pub user_id: String,

    /// User's host (denormalized)
    #[sea_orm(nullable)]
    pub user_host: Option<String>,

    /// Original file name
    pub name: String,

    /// MIME type
    pub content_type: String,

    /// File size in bytes
    pub size: i64,

    /// Storage URL (local or object storage)
    pub url: String,

    /// Thumbnail URL
    #[sea_orm(nullable)]
    pub thumbnail_url: Option<String>,

    /// Webpublic URL (optimized version)
    #[sea_orm(nullable)]
    pub webpublic_url: Option<String>,

    /// `BlurHash` for placeholder
    #[sea_orm(nullable)]
    pub blurhash: Option<String>,

    /// Image/video width
    #[sea_orm(nullable)]
    pub width: Option<i32>,

    /// Image/video height
    #[sea_orm(nullable)]
    pub height: Option<i32>,

    /// File comment/alt text
    #[sea_orm(column_type = "Text", nullable)]
    pub comment: Option<String>,

    /// Is this file sensitive (NSFW)?
    #[sea_orm(default_value = false)]
    pub is_sensitive: bool,

    /// Is this a link (not stored locally)?
    #[sea_orm(default_value = false)]
    pub is_link: bool,

    /// MD5 hash of the file
    #[sea_orm(nullable)]
    pub md5: Option<String>,

    /// Storage key for object storage
    #[sea_orm(nullable)]
    pub storage_key: Option<String>,

    /// Folder ID for organization
    #[sea_orm(nullable)]
    pub folder_id: Option<String>,

    /// `ActivityPub` URI (for remote files)
    #[sea_orm(nullable)]
    pub uri: Option<String>,

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

    #[sea_orm(
        belongs_to = "super::drive_folder::Entity",
        from = "Column::FolderId",
        to = "super::drive_folder::Column::Id",
        on_delete = "SetNull"
    )]
    Folder,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::drive_folder::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Folder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
