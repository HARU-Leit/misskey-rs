//! Antenna entity for filtered note streams.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Source type for antenna matching.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum AntennaSource {
    /// Match from home timeline.
    #[sea_orm(string_value = "home")]
    Home,
    /// Match from all public notes.
    #[sea_orm(string_value = "all")]
    All,
    /// Match from specific users.
    #[sea_orm(string_value = "users")]
    Users,
    /// Match from a user list.
    #[sea_orm(string_value = "list")]
    List,
    /// Match from specific instances.
    #[sea_orm(string_value = "instances")]
    Instances,
}

/// Antenna entity - a filtered stream of notes matching specific criteria.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "antenna")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created the antenna.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// Antenna name.
    pub name: String,

    /// Source type for matching.
    pub src: AntennaSource,

    /// User list ID (when src is "list").
    #[sea_orm(nullable)]
    pub user_list_id: Option<String>,

    /// Keywords to match (JSON array of arrays for AND/OR logic).
    /// Example: [["foo", "bar"], ["baz"]] = (foo AND bar) OR baz
    #[sea_orm(column_type = "JsonBinary")]
    pub keywords: Json,

    /// Keywords to exclude (same format as keywords).
    #[sea_orm(column_type = "JsonBinary")]
    pub exclude_keywords: Json,

    /// User IDs to match (when src is "users").
    #[sea_orm(column_type = "JsonBinary")]
    pub users: Json,

    /// Instance hosts to match (when src is "instances").
    #[sea_orm(column_type = "JsonBinary")]
    pub instances: Json,

    /// Whether to use case-sensitive matching.
    #[sea_orm(default_value = false)]
    pub case_sensitive: bool,

    /// Whether to include replies.
    #[sea_orm(default_value = false)]
    pub with_replies: bool,

    /// Whether to only include notes with files.
    #[sea_orm(default_value = false)]
    pub with_file: bool,

    /// Whether to notify when notes match.
    #[sea_orm(default_value = false)]
    pub notify: bool,

    /// Whether to only include local notes.
    #[sea_orm(default_value = false)]
    pub local_only: bool,

    /// Whether this antenna is active.
    #[sea_orm(default_value = true)]
    pub is_active: bool,

    /// Display order (for sorting user's antennas).
    pub display_order: i32,

    /// Number of notes matched (for statistics).
    #[sea_orm(default_value = 0)]
    pub notes_count: i64,

    /// Last time a note matched.
    #[sea_orm(nullable)]
    pub last_used_at: Option<DateTimeWithTimeZone>,

    /// When the antenna was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the antenna was last updated.
    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,
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
        belongs_to = "super::user_list::Entity",
        from = "Column::UserListId",
        to = "super::user_list::Column::Id",
        on_delete = "SetNull"
    )]
    UserList,
    #[sea_orm(has_many = "super::antenna_note::Entity")]
    AntennaNotes,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::user_list::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserList.def()
    }
}

impl Related<super::antenna_note::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AntennaNotes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
