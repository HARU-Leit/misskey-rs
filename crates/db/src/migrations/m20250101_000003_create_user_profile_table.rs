//! Create `user_profile` table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserProfile::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserProfile::UserId)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserProfile::Password).string_len(256))
                    .col(ColumnDef::new(UserProfile::Email).string_len(256))
                    .col(
                        ColumnDef::new(UserProfile::EmailVerified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(UserProfile::TwoFactorSecret).string_len(128))
                    .col(
                        ColumnDef::new(UserProfile::TwoFactorEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserProfile::AutoAcceptFollowed)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserProfile::AlwaysMarkNsfw)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(UserProfile::PinnedPageIds)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'[]'::jsonb")),
                    )
                    .col(
                        ColumnDef::new(UserProfile::Fields)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'[]'::jsonb")),
                    )
                    .col(
                        ColumnDef::new(UserProfile::MutedWords)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'[]'::jsonb")),
                    )
                    .col(ColumnDef::new(UserProfile::UserCss).text())
                    .col(ColumnDef::new(UserProfile::Birthday).string_len(16))
                    .col(ColumnDef::new(UserProfile::Location).string_len(256))
                    .col(ColumnDef::new(UserProfile::Lang).string_len(16))
                    .col(
                        ColumnDef::new(UserProfile::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(UserProfile::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_profile_user")
                            .from(UserProfile::Table, UserProfile::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index: email (for login lookups)
        manager
            .create_index(
                Index::create()
                    .name("idx_user_profile_email")
                    .table(UserProfile::Table)
                    .col(UserProfile::Email)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserProfile::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum UserProfile {
    Table,
    UserId,
    Password,
    Email,
    EmailVerified,
    TwoFactorSecret,
    TwoFactorEnabled,
    AutoAcceptFollowed,
    AlwaysMarkNsfw,
    PinnedPageIds,
    Fields,
    MutedWords,
    UserCss,
    Birthday,
    Location,
    Lang,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
