//! Security key (WebAuthn/Passkey) entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Security key model for WebAuthn/Passkey authentication.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "security_key")]
pub struct Model {
    /// Unique identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// User who owns this key.
    pub user_id: String,

    /// User-provided name for this key.
    pub name: String,

    /// `WebAuthn` credential ID (base64url encoded).
    #[sea_orm(column_type = "Text", unique)]
    pub credential_id: String,

    /// `WebAuthn` public key (CBOR encoded, base64 stored).
    #[sea_orm(column_type = "Text")]
    pub public_key: String,

    /// Signature counter for replay protection.
    pub counter: i64,

    /// Credential type (e.g., "public-key").
    pub credential_type: String,

    /// Transports supported by this authenticator.
    #[sea_orm(column_type = "JsonBinary")]
    pub transports: Json,

    /// AAGUID of the authenticator (if available).
    #[sea_orm(nullable)]
    pub aaguid: Option<String>,

    /// Whether this is a passkey (resident credential).
    #[sea_orm(default_value = false)]
    pub is_passkey: bool,

    /// Last time this key was used.
    #[sea_orm(nullable)]
    pub last_used_at: Option<DateTimeWithTimeZone>,

    /// When this key was registered.
    pub created_at: DateTimeWithTimeZone,
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
