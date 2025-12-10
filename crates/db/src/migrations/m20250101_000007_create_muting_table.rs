//! Create muting table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Muting::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Muting::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Muting::MuterId).string_len(32).not_null())
                    .col(ColumnDef::new(Muting::MuteeId).string_len(32).not_null())
                    .col(ColumnDef::new(Muting::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Muting::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_muting_muter")
                            .from(Muting::Table, Muting::MuterId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_muting_mutee")
                            .from(Muting::Table, Muting::MuteeId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: (muter_id, mutee_id) - prevent duplicate mutes
        manager
            .create_index(
                Index::create()
                    .name("idx_muting_muter_mutee")
                    .table(Muting::Table)
                    .col(Muting::MuterId)
                    .col(Muting::MuteeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: muter_id (for listing muted users)
        manager
            .create_index(
                Index::create()
                    .name("idx_muting_muter_id")
                    .table(Muting::Table)
                    .col(Muting::MuterId)
                    .to_owned(),
            )
            .await?;

        // Index: expires_at (for cleanup job)
        manager
            .create_index(
                Index::create()
                    .name("idx_muting_expires_at")
                    .table(Muting::Table)
                    .col(Muting::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Muting::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Muting {
    Table,
    Id,
    MuterId,
    MuteeId,
    ExpiresAt,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
