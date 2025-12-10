//! Muting entity (mute relationships between users).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "muting")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The user who is muting
    pub muter_id: String,

    /// The user being muted
    pub mutee_id: String,

    /// When the mute expires (NULL = permanent)
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeWithTimeZone>,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::MuterId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Muter,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::MuteeId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    Mutee,
}

impl ActiveModelBehavior for ActiveModel {}
