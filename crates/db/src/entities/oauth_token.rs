//! OAuth token entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// OAuth token type.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum TokenType {
    #[sea_orm(string_value = "authorization_code")]
    AuthorizationCode,
    #[sea_orm(string_value = "access_token")]
    AccessToken,
    #[sea_orm(string_value = "refresh_token")]
    RefreshToken,
}

/// OAuth token model.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oauth_token")]
pub struct Model {
    /// Unique identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The token value (hashed for security).
    #[sea_orm(column_type = "Text", unique)]
    pub token_hash: String,

    /// Token type.
    pub token_type: TokenType,

    /// Associated OAuth application.
    pub app_id: String,

    /// User who authorized this token.
    pub user_id: String,

    /// Granted scopes (JSON array).
    #[sea_orm(column_type = "JsonBinary")]
    pub scopes: Json,

    /// Code challenge for PKCE (only for authorization codes).
    #[sea_orm(nullable)]
    pub code_challenge: Option<String>,

    /// Code challenge method for PKCE.
    #[sea_orm(nullable)]
    pub code_challenge_method: Option<String>,

    /// Redirect URI used for this authorization.
    #[sea_orm(nullable)]
    pub redirect_uri: Option<String>,

    /// When this token expires.
    pub expires_at: DateTimeWithTimeZone,

    /// Is this token revoked?
    #[sea_orm(default_value = false)]
    pub is_revoked: bool,

    /// When this token was created.
    pub created_at: DateTimeWithTimeZone,

    /// When this token was last used.
    #[sea_orm(nullable)]
    pub last_used_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::oauth_app::Entity",
        from = "Column::AppId",
        to = "super::oauth_app::Column::Id",
        on_delete = "Cascade"
    )]
    App,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::oauth_app::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::App.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
