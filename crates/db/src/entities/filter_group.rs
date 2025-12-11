//! Filter group entity for organizing word filters into presets.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Filter group entity - a collection of word filters that can be enabled/disabled together.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "filter_group")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who created the filter group.
    pub user_id: String,

    /// Group name.
    pub name: String,

    /// Group description (optional).
    pub description: Option<String>,

    /// Whether this group is active (filters in this group are applied).
    pub is_active: bool,

    /// Display order (for sorting user's groups).
    pub display_order: i32,

    /// When the group was created.
    pub created_at: DateTimeWithTimeZone,

    /// When the group was last updated.
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
    #[sea_orm(has_many = "super::word_filter::Entity")]
    WordFilters,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::word_filter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WordFilters.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
