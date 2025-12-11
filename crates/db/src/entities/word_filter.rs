//! Word filter entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Action to take when a word filter matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum FilterAction {
    /// Hide the content completely.
    #[sea_orm(string_value = "hide")]
    Hide,
    /// Show a warning before revealing.
    #[sea_orm(string_value = "warn")]
    Warn,
    /// Automatically add a content warning.
    #[sea_orm(string_value = "cw")]
    ContentWarning,
}

/// Context where the filter applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum FilterContext {
    /// Apply to home timeline.
    #[sea_orm(string_value = "home")]
    Home,
    /// Apply to notifications.
    #[sea_orm(string_value = "notifications")]
    Notifications,
    /// Apply to public timelines.
    #[sea_orm(string_value = "public")]
    Public,
    /// Apply to search results.
    #[sea_orm(string_value = "search")]
    Search,
    /// Apply everywhere.
    #[sea_orm(string_value = "all")]
    All,
}

/// Word filter for hiding or warning about content.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "word_filter")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created this filter.
    #[sea_orm(indexed)]
    pub user_id: String,

    /// The word or phrase to filter.
    pub phrase: String,

    /// Whether to use regex matching.
    #[sea_orm(default_value = false)]
    pub is_regex: bool,

    /// Whether matching is case-sensitive.
    #[sea_orm(default_value = false)]
    pub case_sensitive: bool,

    /// Whether to match whole words only.
    #[sea_orm(default_value = true)]
    pub whole_word: bool,

    /// Action to take when filter matches.
    pub action: FilterAction,

    /// Context where the filter applies.
    pub context: FilterContext,

    /// Optional expiration date (for temporary filters).
    #[sea_orm(nullable)]
    pub expires_at: Option<DateTimeWithTimeZone>,

    /// Number of times this filter has matched.
    #[sea_orm(default_value = 0)]
    pub match_count: i64,

    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(nullable)]
    pub updated_at: Option<DateTimeWithTimeZone>,

    /// Filter group this filter belongs to (optional).
    #[sea_orm(nullable)]
    pub group_id: Option<String>,
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
        belongs_to = "super::filter_group::Entity",
        from = "Column::GroupId",
        to = "super::filter_group::Column::Id"
    )]
    FilterGroup,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::filter_group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FilterGroup.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
