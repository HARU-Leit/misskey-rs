//! Gallery post entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Gallery post entity - a visual showcase post.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gallery_post")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Author user ID.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Post title.
    pub title: String,

    /// Post description (optional).
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// File IDs (images/videos) for this gallery post.
    #[sea_orm(column_type = "JsonBinary")]
    pub file_ids: Json,

    /// Whether this post contains sensitive content.
    #[sea_orm(default_value = false)]
    pub is_sensitive: bool,

    /// Tags for categorization.
    #[sea_orm(column_type = "JsonBinary")]
    pub tags: Json,

    /// Number of likes.
    #[sea_orm(default_value = 0)]
    pub liked_count: i32,

    /// When the post was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the post was last updated.
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
    #[sea_orm(has_many = "super::gallery_like::Entity")]
    Likes,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::gallery_like::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Likes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
