//! Antenna note entity - tracks notes matched by antennas.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Association between an antenna and a matched note.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "antenna_note")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The antenna that matched.
    #[sea_orm(indexed)]
    pub antenna_id: String,

    /// The note that was matched.
    #[sea_orm(indexed)]
    pub note_id: String,

    /// Whether the user has read this note in the antenna.
    #[sea_orm(default_value = false)]
    pub is_read: bool,

    /// When this match was recorded.
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::antenna::Entity",
        from = "Column::AntennaId",
        to = "super::antenna::Column::Id",
        on_delete = "Cascade"
    )]
    Antenna,
    #[sea_orm(
        belongs_to = "super::note::Entity",
        from = "Column::NoteId",
        to = "super::note::Column::Id",
        on_delete = "Cascade"
    )]
    Note,
}

impl Related<super::antenna::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Antenna.def()
    }
}

impl Related<super::note::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Note.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
