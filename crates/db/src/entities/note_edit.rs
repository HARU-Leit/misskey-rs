//! Note edit history entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Record of a note edit.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "note_edit")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Note ID that was edited
    #[sea_orm(indexed)]
    pub note_id: String,

    /// Previous text content (before this edit)
    #[sea_orm(column_type = "Text", nullable)]
    pub old_text: Option<String>,

    /// New text content (after this edit)
    #[sea_orm(column_type = "Text", nullable)]
    pub new_text: Option<String>,

    /// Previous CW (before this edit)
    #[sea_orm(nullable)]
    pub old_cw: Option<String>,

    /// New CW (after this edit)
    #[sea_orm(nullable)]
    pub new_cw: Option<String>,

    /// Previous file IDs (before this edit)
    #[sea_orm(column_type = "JsonBinary")]
    pub old_file_ids: Json,

    /// New file IDs (after this edit)
    #[sea_orm(column_type = "JsonBinary")]
    pub new_file_ids: Json,

    /// When the edit was made
    pub edited_at: DateTimeWithTimeZone,
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
