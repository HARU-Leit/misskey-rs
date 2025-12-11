//! Channel entity for topic-based note grouping.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Channel entity - a topic-based container for notes.
/// Supports `ActivityPub` federation as a Group actor.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "channel")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created the channel.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Channel name.
    pub name: String,

    /// Channel description (optional).
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// Banner image ID (optional).
    #[sea_orm(nullable)]
    pub banner_id: Option<String>,

    /// Channel color (hex code, optional).
    #[sea_orm(nullable)]
    pub color: Option<String>,

    /// Whether this channel is archived.
    #[sea_orm(default_value = false)]
    pub is_archived: bool,

    /// Whether notes in this channel are searchable.
    #[sea_orm(default_value = true)]
    pub is_searchable: bool,

    /// Whether anyone can post to this channel.
    #[sea_orm(default_value = true)]
    pub allow_anyone_to_post: bool,

    /// Number of notes in this channel (denormalized).
    #[sea_orm(default_value = 0)]
    pub notes_count: i64,

    /// Number of users following this channel.
    #[sea_orm(default_value = 0)]
    pub users_count: i64,

    /// Last time a note was posted.
    #[sea_orm(nullable)]
    pub last_noted_at: Option<DateTimeWithTimeZone>,

    /// When the channel was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the channel was last updated.
    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,

    // === Federation fields (ActivityPub Group actor) ===
    /// `ActivityPub` URI for this channel (unique identifier for federation).
    /// Null for legacy local channels without federation enabled.
    #[sea_orm(nullable, unique, indexed)]
    pub uri: Option<String>,

    /// Public key PEM for HTTP signature verification.
    #[sea_orm(column_type = "Text", nullable)]
    pub public_key_pem: Option<String>,

    /// Private key PEM for signing outgoing activities.
    /// Only present for local channels.
    #[sea_orm(column_type = "Text", nullable)]
    pub private_key_pem: Option<String>,

    /// Inbox URL for receiving `ActivityPub` activities.
    #[sea_orm(nullable)]
    pub inbox: Option<String>,

    /// Shared inbox URL for efficient activity delivery.
    #[sea_orm(nullable)]
    pub shared_inbox: Option<String>,

    /// Host of the remote instance (null for local channels).
    #[sea_orm(nullable, indexed)]
    pub host: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::drive_file::Entity",
        from = "Column::BannerId",
        to = "super::drive_file::Column::Id",
        on_delete = "SetNull"
    )]
    Banner,
    #[sea_orm(has_many = "super::channel_following::Entity")]
    Followings,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::drive_file::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Banner.def()
    }
}

impl Related<super::channel_following::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Followings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
