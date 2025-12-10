//! Instance entity for federation.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Instance/server in the federation network.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "instance")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The hostname of this instance (unique identifier).
    #[sea_orm(unique)]
    pub host: String,

    /// Number of users from this instance we know about.
    #[sea_orm(default_value = 0)]
    pub users_count: i32,

    /// Number of notes from this instance we've received.
    #[sea_orm(default_value = 0)]
    pub notes_count: i32,

    /// Number of users from this instance following local users.
    #[sea_orm(default_value = 0)]
    pub following_count: i32,

    /// Number of local users following users from this instance.
    #[sea_orm(default_value = 0)]
    pub followers_count: i32,

    /// Software name (e.g., "misskey", "mastodon", "pleroma").
    #[sea_orm(nullable)]
    pub software_name: Option<String>,

    /// Software version.
    #[sea_orm(nullable)]
    pub software_version: Option<String>,

    /// Instance name.
    #[sea_orm(nullable)]
    pub name: Option<String>,

    /// Instance description.
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// Admin contact email.
    #[sea_orm(nullable)]
    pub maintainer_email: Option<String>,

    /// Admin contact name.
    #[sea_orm(nullable)]
    pub maintainer_name: Option<String>,

    /// Instance icon URL.
    #[sea_orm(nullable)]
    pub icon_url: Option<String>,

    /// Instance favicon URL.
    #[sea_orm(nullable)]
    pub favicon_url: Option<String>,

    /// Instance theme color.
    #[sea_orm(nullable)]
    pub theme_color: Option<String>,

    /// Whether this instance is blocked (no federation).
    #[sea_orm(default_value = false)]
    pub is_blocked: bool,

    /// Whether this instance is silenced (posts not in global timeline).
    #[sea_orm(default_value = false)]
    pub is_silenced: bool,

    /// Whether this instance is suspended (temporary block).
    #[sea_orm(default_value = false)]
    pub is_suspended: bool,

    /// Moderator notes about this instance.
    #[sea_orm(column_type = "Text", nullable)]
    pub moderation_note: Option<String>,

    /// Last time we successfully communicated with this instance.
    #[sea_orm(nullable)]
    pub last_communicated_at: Option<DateTimeWithTimeZone>,

    /// Last time we fetched instance info.
    #[sea_orm(nullable)]
    pub info_updated_at: Option<DateTimeWithTimeZone>,

    /// Whether nodeinfo was successfully fetched.
    #[sea_orm(default_value = false)]
    pub is_nodeinfo_fetched: bool,

    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
