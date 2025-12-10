//! Blocking entity (block relationships between users).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "blocking")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The user who is blocking
    pub blocker_id: String,

    /// The user being blocked
    pub blockee_id: String,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::BlockerId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Blocker,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::BlockeeId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Blockee,
}

impl ActiveModelBehavior for ActiveModel {}
