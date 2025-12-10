//! OAuth application entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// OAuth application model.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oauth_app")]
pub struct Model {
    /// Unique identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// Client ID for OAuth.
    #[sea_orm(unique)]
    pub client_id: String,

    /// Client secret (hashed).
    pub client_secret: String,

    /// Application name.
    pub name: String,

    /// Application description.
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    /// Application icon URL.
    #[sea_orm(nullable)]
    pub icon_url: Option<String>,

    /// Application website URL.
    #[sea_orm(nullable)]
    pub website_url: Option<String>,

    /// Allowed redirect URIs (JSON array).
    #[sea_orm(column_type = "JsonBinary")]
    pub redirect_uris: Json,

    /// Scopes the application is allowed to request (JSON array).
    #[sea_orm(column_type = "JsonBinary")]
    pub scopes: Json,

    /// User who created this application.
    pub user_id: String,

    /// Is this a trusted first-party application?
    #[sea_orm(default_value = false)]
    pub is_trusted: bool,

    /// Is this application active?
    #[sea_orm(default_value = true)]
    pub is_active: bool,

    /// When this application was created.
    pub created_at: DateTimeWithTimeZone,

    /// When this application was last updated.
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
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
