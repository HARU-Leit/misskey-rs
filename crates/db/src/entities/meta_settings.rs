//! Meta settings entity for instance configuration.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Instance-wide settings and configuration.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "meta_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    // Instance info
    /// Instance name
    #[sea_orm(nullable)]
    pub name: Option<String>,

    /// Short name for the instance
    #[sea_orm(nullable)]
    pub short_name: Option<String>,

    /// Instance description
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// Maintainer name
    #[sea_orm(nullable)]
    pub maintainer_name: Option<String>,

    /// Maintainer email
    #[sea_orm(nullable)]
    pub maintainer_email: Option<String>,

    /// Supported languages
    #[sea_orm(column_type = "JsonBinary")]
    pub langs: Json,

    /// Instance icon URL
    #[sea_orm(nullable)]
    pub icon_url: Option<String>,

    /// Instance banner URL
    #[sea_orm(nullable)]
    pub banner_url: Option<String>,

    /// Instance theme color
    #[sea_orm(nullable)]
    pub theme_color: Option<String>,

    // Registration settings
    /// Whether new user registration is disabled
    #[sea_orm(default_value = false)]
    pub disable_registration: bool,

    /// Whether email is required for signup
    #[sea_orm(default_value = false)]
    pub email_required_for_signup: bool,

    /// Whether registration requires admin approval
    #[sea_orm(default_value = false)]
    pub require_registration_approval: bool,

    // Media settings
    /// Whether to force all uploaded media to be marked as NSFW
    #[sea_orm(default_value = false)]
    pub force_nsfw_media: bool,

    // UI defaults
    /// Default setting for blurring NSFW content
    #[sea_orm(default_value = true)]
    pub default_blur_nsfw: bool,

    /// Default setting for hiding ads
    #[sea_orm(default_value = false)]
    pub default_hide_ads: bool,

    // Content limits
    /// Maximum note text length for local users
    #[sea_orm(default_value = 3000)]
    pub max_note_text_length: i32,

    /// Maximum note text length for remote users (federation)
    #[sea_orm(default_value = 10000)]
    pub max_remote_note_text_length: i32,

    /// Maximum page content length (for Page feature)
    #[sea_orm(default_value = 65536)]
    pub max_page_content_length: i32,

    /// Maximum pages per user
    #[sea_orm(default_value = 100)]
    pub max_pages_per_user: i32,

    // Drive limits
    /// Default drive capacity per user (in MB)
    #[sea_orm(default_value = 1024)]
    pub default_drive_capacity_mb: i32,

    /// Maximum file size (in MB)
    #[sea_orm(default_value = 256)]
    pub max_file_size_mb: i32,

    // Bubble timeline settings
    /// Whitelisted instances for bubble timeline (JSON array of hostnames)
    /// e.g., ["mastodon.social", "pixelfed.social"]
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub bubble_instances: Option<Json>,

    // Timestamps
    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Singleton ID for the meta settings
pub const META_SETTINGS_ID: &str = "instance";
