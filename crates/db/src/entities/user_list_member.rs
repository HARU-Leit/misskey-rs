//! User list member entity.

use sea_orm::entity::prelude::*;

/// User list member entity.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user_list_member")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,

    /// The list this member belongs to.
    pub list_id: String,

    /// The user who is a member.
    pub user_id: String,

    /// When the member was added.
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user_list::Entity",
        from = "Column::ListId",
        to = "super::user_list::Column::Id"
    )]
    UserList,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user_list::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserList.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
