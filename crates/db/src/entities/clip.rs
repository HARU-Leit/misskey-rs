//! Clip entity for saving notes to collections.

use sea_orm::entity::prelude::*;

/// Clip entity - a collection of notes created by a user.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "clip")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created the clip.
    pub user_id: String,

    /// Clip name.
    pub name: String,

    /// Clip description (optional).
    pub description: Option<String>,

    /// Whether this clip is public.
    pub is_public: bool,

    /// Number of notes in this clip (denormalized for performance).
    pub notes_count: i32,

    /// Display order (for sorting user's clips).
    pub display_order: i32,

    /// When the clip was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the clip was last updated.
    pub updated_at: Option<DateTimeWithTimeZone>,

    /// Whether this is a smart clip (auto-adds notes based on conditions).
    pub is_smart_clip: bool,

    /// Smart clip conditions (JSON).
    /// Example: {"keywords": ["rust"], "users": ["userId"], "hashtags": ["programming"]}
    pub smart_conditions: Option<Json>,

    /// Maximum notes for smart clip (oldest removed when exceeded).
    pub smart_max_notes: Option<i32>,

    /// When the smart clip last processed new notes.
    pub smart_last_processed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(has_many = "super::clip_note::Entity")]
    ClipNotes,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::clip_note::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ClipNotes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
