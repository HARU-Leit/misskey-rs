//! User entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    #[sea_orm(unique)]
    pub username: String,

    pub username_lower: String,

    /// NULL = local user, Some(host) = remote user
    #[sea_orm(nullable)]
    pub host: Option<String>,

    /// Access token (local users only)
    #[sea_orm(unique, nullable)]
    pub token: Option<String>,

    /// Display name
    #[sea_orm(nullable)]
    pub name: Option<String>,

    /// Profile description
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// Avatar URL
    #[sea_orm(nullable)]
    pub avatar_url: Option<String>,

    /// Banner URL
    #[sea_orm(nullable)]
    pub banner_url: Option<String>,

    /// Followers count (denormalized)
    #[sea_orm(default_value = 0)]
    pub followers_count: i32,

    /// Following count (denormalized)
    #[sea_orm(default_value = 0)]
    pub following_count: i32,

    /// Notes count (denormalized)
    #[sea_orm(default_value = 0)]
    pub notes_count: i32,

    /// Is this user a bot?
    #[sea_orm(default_value = false)]
    pub is_bot: bool,

    /// Is this user a cat? (Misskey-specific)
    #[sea_orm(default_value = false)]
    pub is_cat: bool,

    /// Is this account locked (requires follow approval)?
    #[sea_orm(default_value = false)]
    pub is_locked: bool,

    /// Is this account suspended?
    #[sea_orm(default_value = false)]
    pub is_suspended: bool,

    /// Is this account silenced?
    #[sea_orm(default_value = false)]
    pub is_silenced: bool,

    /// Is this user an admin?
    #[sea_orm(default_value = false)]
    pub is_admin: bool,

    /// Is this user a moderator?
    #[sea_orm(default_value = false)]
    pub is_moderator: bool,

    /// `ActivityPub` inbox URL (remote users)
    #[sea_orm(nullable)]
    pub inbox: Option<String>,

    /// `ActivityPub` shared inbox URL (remote users)
    #[sea_orm(nullable)]
    pub shared_inbox: Option<String>,

    /// `ActivityPub` featured collection URL
    #[sea_orm(nullable)]
    pub featured: Option<String>,

    /// `ActivityPub` URI
    #[sea_orm(nullable)]
    pub uri: Option<String>,

    /// Last time this remote user was fetched
    #[sea_orm(nullable)]
    pub last_fetched_at: Option<DateTimeWithTimeZone>,

    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::note::Entity")]
    Notes,

    #[sea_orm(has_one = "super::user_profile::Entity")]
    Profile,

    #[sea_orm(has_one = "super::user_keypair::Entity")]
    Keypair,
}

impl Related<super::note::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Notes.def()
    }
}

impl Related<super::user_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Profile.def()
    }
}

impl Related<super::user_keypair::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Keypair.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
