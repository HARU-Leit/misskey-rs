//! Gallery like entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Gallery like - a record of a user liking a gallery post.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gallery_like")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who liked the post.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Gallery post that was liked.
    #[sea_orm(indexed)]
    pub post_id: String,

    /// When the like was created.
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
        belongs_to = "super::gallery_post::Entity",
        from = "Column::PostId",
        to = "super::gallery_post::Column::Id",
        on_delete = "Cascade"
    )]
    Post,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::gallery_post::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Post.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
