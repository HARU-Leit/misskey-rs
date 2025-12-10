//! Hashtag entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Hashtag for indexing and trending.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hashtag")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The hashtag name (lowercase, without #)
    #[sea_orm(unique)]
    pub name: String,

    /// Number of notes using this hashtag
    #[sea_orm(default_value = 0)]
    pub notes_count: i32,

    /// Number of users who have used this hashtag
    #[sea_orm(default_value = 0)]
    pub users_count: i32,

    /// Number of local notes using this hashtag
    #[sea_orm(default_value = 0)]
    pub local_notes_count: i32,

    /// Number of remote notes using this hashtag
    #[sea_orm(default_value = 0)]
    pub remote_notes_count: i32,

    /// Is this hashtag trending?
    #[sea_orm(default_value = false)]
    pub is_trending: bool,

    /// When was this hashtag last used
    #[sea_orm(nullable)]
    pub last_used_at: Option<DateTimeWithTimeZone>,

    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
