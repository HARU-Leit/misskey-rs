//! User keypair entity (RSA keys for `ActivityPub` signing).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// User keypair for `ActivityPub` HTTP Signatures.
/// Each local user has exactly one keypair.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_keypair")]
pub struct Model {
    /// Same as user.id (1:1 relationship)
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,

    /// RSA public key (PEM format)
    #[sea_orm(column_type = "Text")]
    pub public_key: String,

    /// RSA private key (PEM format, encrypted at rest recommended)
    #[sea_orm(column_type = "Text")]
    pub private_key: String,

    /// Key ID (typically the user's `ActivityPub` ID + #main-key)
    pub key_id: String,

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
