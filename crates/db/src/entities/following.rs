//! Following entity (follow relationships between users).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "following")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The user who is following
    pub follower_id: String,

    /// The user being followed
    pub followee_id: String,

    /// Follower's host (denormalized for query efficiency)
    #[sea_orm(nullable)]
    pub follower_host: Option<String>,

    /// Followee's host (denormalized for query efficiency)
    #[sea_orm(nullable)]
    pub followee_host: Option<String>,

    /// Inbox URL for the followee (for delivering activities)
    #[sea_orm(nullable)]
    pub followee_inbox: Option<String>,

    /// Shared inbox URL for the followee
    #[sea_orm(nullable)]
    pub followee_shared_inbox: Option<String>,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::FollowerId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Follower,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::FolloweeId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Followee,
}

impl ActiveModelBehavior for ActiveModel {}
