//! Note entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Note visibility levels.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum Visibility {
    #[sea_orm(string_value = "public")]
    Public,
    #[sea_orm(string_value = "home")]
    Home,
    #[sea_orm(string_value = "followers")]
    Followers,
    #[sea_orm(string_value = "specified")]
    Specified,
}

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "note")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Author user ID
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Author's host (denormalized for query efficiency)
    #[sea_orm(nullable)]
    pub user_host: Option<String>,

    /// Note text content
    #[sea_orm(column_type = "Text", nullable)]
    pub text: Option<String>,

    /// Content warning
    #[sea_orm(nullable)]
    pub cw: Option<String>,

    /// Visibility level
    pub visibility: Visibility,

    /// Reply target note ID
    #[sea_orm(nullable, indexed)]
    pub reply_id: Option<String>,

    /// Renote target note ID
    #[sea_orm(nullable, indexed)]
    pub renote_id: Option<String>,

    /// Thread root ID
    #[sea_orm(nullable, indexed)]
    pub thread_id: Option<String>,

    /// Mentioned user IDs
    #[sea_orm(column_type = "JsonBinary")]
    pub mentions: Json,

    /// Users who can see this note (for visibility = specified)
    #[sea_orm(column_type = "JsonBinary")]
    pub visible_user_ids: Json,

    /// Attached file IDs
    #[sea_orm(column_type = "JsonBinary")]
    pub file_ids: Json,

    /// Hashtags
    #[sea_orm(column_type = "JsonBinary")]
    pub tags: Json,

    /// Reactions (emoji -> count)
    #[sea_orm(column_type = "JsonBinary")]
    pub reactions: Json,

    /// Reply count (denormalized)
    #[sea_orm(default_value = 0)]
    pub replies_count: i32,

    /// Renote count (denormalized)
    #[sea_orm(default_value = 0)]
    pub renote_count: i32,

    /// Reaction count (denormalized)
    #[sea_orm(default_value = 0)]
    pub reaction_count: i32,

    /// Is this note local?
    #[sea_orm(default_value = true)]
    pub is_local: bool,

    /// `ActivityPub` URI
    #[sea_orm(nullable)]
    pub uri: Option<String>,

    /// `ActivityPub` URL (human-readable)
    #[sea_orm(nullable)]
    pub url: Option<String>,

    /// Channel ID (if posted to a channel)
    #[sea_orm(nullable, indexed)]
    pub channel_id: Option<String>,

    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,

    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ReplyId",
        to = "Column::Id"
    )]
    Reply,

    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::RenoteId",
        to = "Column::Id"
    )]
    Renote,

    #[sea_orm(
        belongs_to = "super::channel::Entity",
        from = "Column::ChannelId",
        to = "super::channel::Column::Id"
    )]
    Channel,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
