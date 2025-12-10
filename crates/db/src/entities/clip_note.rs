//! Clip note entity - notes added to clips.

use sea_orm::entity::prelude::*;

/// Clip note entity - a note that belongs to a clip.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "clip_note")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Clip this note belongs to.
    pub clip_id: String,

    /// Note that was clipped.
    pub note_id: String,

    /// Display order within the clip (for manual ordering).
    pub display_order: i32,

    /// Optional comment about why this note was clipped.
    pub comment: Option<String>,

    /// When the note was added to the clip.
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::clip::Entity",
        from = "Column::ClipId",
        to = "super::clip::Column::Id"
    )]
    Clip,
    #[sea_orm(
        belongs_to = "super::note::Entity",
        from = "Column::NoteId",
        to = "super::note::Column::Id"
    )]
    Note,
}

impl Related<super::clip::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Clip.def()
    }
}

impl Related<super::note::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Note.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
