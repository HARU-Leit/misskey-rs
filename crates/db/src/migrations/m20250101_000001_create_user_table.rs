//! Create user table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).string_len(32).not_null().primary_key())
                    .col(ColumnDef::new(User::Username).string_len(128).not_null())
                    .col(ColumnDef::new(User::UsernameLower).string_len(128).not_null())
                    .col(ColumnDef::new(User::Host).string_len(256))
                    .col(ColumnDef::new(User::Token).string_len(64))
                    .col(ColumnDef::new(User::Name).string_len(256))
                    .col(ColumnDef::new(User::Description).text())
                    .col(ColumnDef::new(User::AvatarUrl).string_len(1024))
                    .col(ColumnDef::new(User::BannerUrl).string_len(1024))
                    .col(ColumnDef::new(User::FollowersCount).integer().not_null().default(0))
                    .col(ColumnDef::new(User::FollowingCount).integer().not_null().default(0))
                    .col(ColumnDef::new(User::NotesCount).integer().not_null().default(0))
                    .col(ColumnDef::new(User::IsBot).boolean().not_null().default(false))
                    .col(ColumnDef::new(User::IsCat).boolean().not_null().default(false))
                    .col(ColumnDef::new(User::IsLocked).boolean().not_null().default(false))
                    .col(ColumnDef::new(User::IsSuspended).boolean().not_null().default(false))
                    .col(ColumnDef::new(User::IsSilenced).boolean().not_null().default(false))
                    .col(ColumnDef::new(User::Inbox).string_len(1024))
                    .col(ColumnDef::new(User::SharedInbox).string_len(1024))
                    .col(ColumnDef::new(User::Featured).string_len(1024))
                    .col(ColumnDef::new(User::Uri).string_len(1024))
                    .col(ColumnDef::new(User::LastFetchedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(User::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(User::UpdatedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        // Unique index: (username_lower, host) - NULL host means local user
        manager
            .create_index(
                Index::create()
                    .name("idx_user_username_lower_host")
                    .table(User::Table)
                    .col(User::UsernameLower)
                    .col(User::Host)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Unique index: token
        manager
            .create_index(
                Index::create()
                    .name("idx_user_token")
                    .table(User::Table)
                    .col(User::Token)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: host (for filtering local/remote users)
        manager
            .create_index(
                Index::create()
                    .name("idx_user_host")
                    .table(User::Table)
                    .col(User::Host)
                    .to_owned(),
            )
            .await?;

        // Index: created_at
        manager
            .create_index(
                Index::create()
                    .name("idx_user_created_at")
                    .table(User::Table)
                    .col(User::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum User {
    Table,
    Id,
    Username,
    UsernameLower,
    Host,
    Token,
    Name,
    Description,
    AvatarUrl,
    BannerUrl,
    FollowersCount,
    FollowingCount,
    NotesCount,
    IsBot,
    IsCat,
    IsLocked,
    IsSuspended,
    IsSilenced,
    Inbox,
    SharedInbox,
    Featured,
    Uri,
    LastFetchedAt,
    CreatedAt,
    UpdatedAt,
}
