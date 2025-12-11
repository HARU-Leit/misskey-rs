//! Create emoji table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Emoji::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Emoji::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Emoji::Name).string_len(128).not_null())
                    .col(ColumnDef::new(Emoji::Category).string_len(128))
                    .col(
                        ColumnDef::new(Emoji::OriginalUrl)
                            .string_len(1024)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Emoji::StaticUrl).string_len(1024))
                    .col(ColumnDef::new(Emoji::ContentType).string_len(64).not_null())
                    .col(ColumnDef::new(Emoji::Aliases).json().not_null())
                    .col(ColumnDef::new(Emoji::Host).string_len(256))
                    .col(ColumnDef::new(Emoji::License).string_len(256))
                    .col(
                        ColumnDef::new(Emoji::IsSensitive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Emoji::LocalOnly)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Emoji::Width).integer())
                    .col(ColumnDef::new(Emoji::Height).integer())
                    .col(ColumnDef::new(Emoji::Size).big_integer())
                    .col(
                        ColumnDef::new(Emoji::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Emoji::UpdatedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        // Unique index: (name, host) - NULL host means local emoji
        manager
            .create_index(
                Index::create()
                    .name("idx_emoji_name_host")
                    .table(Emoji::Table)
                    .col(Emoji::Name)
                    .col(Emoji::Host)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: host (for filtering local/remote emojis)
        manager
            .create_index(
                Index::create()
                    .name("idx_emoji_host")
                    .table(Emoji::Table)
                    .col(Emoji::Host)
                    .to_owned(),
            )
            .await?;

        // Index: category (for listing by category)
        manager
            .create_index(
                Index::create()
                    .name("idx_emoji_category")
                    .table(Emoji::Table)
                    .col(Emoji::Category)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Emoji::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Emoji {
    Table,
    Id,
    Name,
    Category,
    OriginalUrl,
    StaticUrl,
    ContentType,
    Aliases,
    Host,
    License,
    IsSensitive,
    LocalOnly,
    Width,
    Height,
    Size,
    CreatedAt,
    UpdatedAt,
}
