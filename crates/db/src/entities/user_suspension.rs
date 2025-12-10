//! User suspension entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// User suspension model - tracks when users are suspended by admins.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_suspension")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// The suspended user.
    pub user_id: String,
    /// The admin who created the suspension.
    pub moderator_id: String,
    /// Reason for the suspension.
    pub reason: String,
    /// When the suspension was created.
    pub created_at: DateTimeWithTimeZone,
    /// When the suspension expires (None = permanent).
    pub expires_at: Option<DateTimeWithTimeZone>,
    /// When the suspension was lifted (if lifted early).
    pub lifted_at: Option<DateTimeWithTimeZone>,
    /// Admin who lifted the suspension.
    pub lifted_by: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
