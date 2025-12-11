//! Create word_filter table.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WordFilter::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WordFilter::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WordFilter::UserId).string_len(32).not_null())
                    .col(
                        ColumnDef::new(WordFilter::Phrase)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WordFilter::IsRegex)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(WordFilter::CaseSensitive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(WordFilter::WholeWord)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(WordFilter::Action)
                            .string_len(16)
                            .not_null()
                            .default("hide"),
                    )
                    .col(
                        ColumnDef::new(WordFilter::Context)
                            .string_len(16)
                            .not_null()
                            .default("all"),
                    )
                    .col(ColumnDef::new(WordFilter::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(WordFilter::MatchCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(WordFilter::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(WordFilter::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_word_filter_user")
                            .from(WordFilter::Table, WordFilter::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on user_id for quick filter lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_word_filter_user_id")
                    .table(WordFilter::Table)
                    .col(WordFilter::UserId)
                    .to_owned(),
            )
            .await?;

        // Index on expires_at for cleanup queries
        manager
            .create_index(
                Index::create()
                    .name("idx_word_filter_expires_at")
                    .table(WordFilter::Table)
                    .col(WordFilter::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WordFilter::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum WordFilter {
    Table,
    Id,
    UserId,
    Phrase,
    IsRegex,
    CaseSensitive,
    WholeWord,
    Action,
    Context,
    ExpiresAt,
    MatchCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
