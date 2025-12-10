//! Announcement read tracking entity.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Tracks which users have read which announcements.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "announcement_read")]
pub struct Model {
    /// Unique read record ID.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// ID of the announcement that was read.
    pub announcement_id: String,

    /// ID of the user who read the announcement.
    pub user_id: String,

    /// When the user read the announcement.
    pub created_at: DateTime<Utc>,
}

/// Relationships.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::announcement::Entity",
        from = "Column::AnnouncementId",
        to = "super::announcement::Column::Id"
    )]
    Announcement,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::announcement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Announcement.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
