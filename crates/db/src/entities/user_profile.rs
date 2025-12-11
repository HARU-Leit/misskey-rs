//! User profile entity (stores password and additional settings).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_profile")]
pub struct Model {
    /// Same as user.id (1:1 relationship)
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,

    /// Password hash (Argon2, local users only)
    #[sea_orm(nullable)]
    pub password: Option<String>,

    /// Email address
    #[sea_orm(nullable)]
    pub email: Option<String>,

    /// Is email verified?
    #[sea_orm(default_value = false)]
    pub email_verified: bool,

    /// Two-factor authentication secret
    #[sea_orm(nullable)]
    pub two_factor_secret: Option<String>,

    /// Is two-factor authentication enabled?
    #[sea_orm(default_value = false)]
    pub two_factor_enabled: bool,

    /// Pending two-factor secret (during setup, before confirmation)
    #[sea_orm(nullable)]
    pub two_factor_pending: Option<String>,

    /// Two-factor backup codes (hashed, JSON array)
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub two_factor_backup_codes: Option<Json>,

    /// Auto-accept follow requests?
    #[sea_orm(default_value = false)]
    pub auto_accept_followed: bool,

    /// Always mark notes as sensitive?
    #[sea_orm(default_value = false)]
    pub always_mark_nsfw: bool,

    /// Pinned page IDs
    #[sea_orm(column_type = "JsonBinary")]
    pub pinned_page_ids: Json,

    /// Pinned note IDs (displayed on profile)
    #[sea_orm(column_type = "JsonBinary")]
    pub pinned_note_ids: Json,

    /// Profile fields (key-value pairs)
    #[sea_orm(column_type = "JsonBinary")]
    pub fields: Json,

    /// Muted words (for timeline filtering)
    #[sea_orm(column_type = "JsonBinary")]
    pub muted_words: Json,

    /// User-defined CSS for their profile
    #[sea_orm(column_type = "Text", nullable)]
    pub user_css: Option<String>,

    /// Birthday (YYYY-MM-DD format)
    #[sea_orm(nullable)]
    pub birthday: Option<String>,

    /// Location
    #[sea_orm(nullable)]
    pub location: Option<String>,

    /// Language preference
    #[sea_orm(nullable)]
    pub lang: Option<String>,

    /// Pronouns (e.g., "they/them", "she/her", "he/him")
    #[sea_orm(nullable)]
    pub pronouns: Option<String>,

    /// Also known as (`ActivityPub` aliases for account migration)
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub also_known_as: Option<Json>,

    /// URI of account this user has moved to
    #[sea_orm(nullable)]
    pub moved_to_uri: Option<String>,

    /// Hide notes from bot accounts in timeline
    #[sea_orm(default_value = false)]
    pub hide_bots: bool,

    /// Default reaction emoji (e.g., "👍", ":like:", custom emoji shortcode)
    #[sea_orm(nullable)]
    pub default_reaction: Option<String>,

    /// Only allow DMs from followers (when true, non-followers cannot send DMs)
    #[sea_orm(default_value = false)]
    pub receive_dm_from_followers_only: bool,

    /// Require HTTP signature verification for requests to this user's resources.
    /// When true, unsigned/invalid signature requests to this user's profile, notes, etc.
    /// will be rejected with 401 Unauthorized.
    #[sea_orm(default_value = false)]
    pub secure_fetch_only: bool,

    pub created_at: DateTimeWithTimeZone,

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
