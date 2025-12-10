//! Follow request entity (pending follow requests for locked accounts).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "follow_request")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The user who sent the follow request
    pub follower_id: String,

    /// The user who received the follow request
    pub followee_id: String,

    /// Follower's host (denormalized)
    #[sea_orm(nullable)]
    pub follower_host: Option<String>,

    /// Followee's host (denormalized)
    #[sea_orm(nullable)]
    pub followee_host: Option<String>,

    /// Inbox URL for the follower (for Accept/Reject activities)
    #[sea_orm(nullable)]
    pub follower_inbox: Option<String>,

    /// Shared inbox URL for the follower
    #[sea_orm(nullable)]
    pub follower_shared_inbox: Option<String>,

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
