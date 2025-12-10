//! Custom emoji entity.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Custom emoji entity for instance-level custom emojis.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "emoji")]
pub struct Model {
    /// Emoji ID.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Emoji shortcode (e.g., "blobcat" for :blobcat:).
    #[sea_orm(unique)]
    pub name: String,

    /// Category for organizing emojis (nullable).
    pub category: Option<String>,

    /// Original image URL.
    pub original_url: String,

    /// Static (non-animated) version URL.
    pub static_url: Option<String>,

    /// MIME type of the emoji image.
    pub content_type: String,

    /// Aliases for this emoji (stored as JSON array).
    pub aliases: Json,

    /// Host where this emoji originates (null for local).
    pub host: Option<String>,

    /// Whether this emoji is a license-free emoji.
    pub license: Option<String>,

    /// Whether this emoji is enabled.
    pub is_sensitive: bool,

    /// Whether this emoji is only usable by local users.
    pub local_only: bool,

    /// Width in pixels.
    pub width: Option<i32>,

    /// Height in pixels.
    pub height: Option<i32>,

    /// File size in bytes.
    pub size: Option<i64>,

    /// Created at timestamp.
    pub created_at: DateTime<Utc>,

    /// Updated at timestamp.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Emoji relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
