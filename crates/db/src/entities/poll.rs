//! Poll entity for note polls/votes.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "poll")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub note_id: String,

    /// Poll choices (JSON array of strings)
    #[sea_orm(column_type = "Json")]
    pub choices: JsonValue,

    /// Vote counts per choice (JSON array of integers)
    #[sea_orm(column_type = "Json")]
    pub votes: JsonValue,

    /// Whether multiple choices are allowed
    pub multiple: bool,

    /// When the poll expires (null for no expiration)
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeWithTimeZone>,

    /// Total number of unique voters
    pub voters_count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::note::Entity",
        from = "Column::NoteId",
        to = "super::note::Column::Id",
        on_delete = "Cascade"
    )]
    Note,
}

impl Related<super::note::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Note.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
