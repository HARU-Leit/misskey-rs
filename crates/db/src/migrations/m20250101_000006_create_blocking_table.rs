//! Create blocking table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Blocking::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Blocking::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Blocking::BlockerId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Blocking::BlockeeId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Blocking::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blocking_blocker")
                            .from(Blocking::Table, Blocking::BlockerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_blocking_blockee")
                            .from(Blocking::Table, Blocking::BlockeeId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: (blocker_id, blockee_id) - prevent duplicate blocks
        manager
            .create_index(
                Index::create()
                    .name("idx_blocking_blocker_blockee")
                    .table(Blocking::Table)
                    .col(Blocking::BlockerId)
                    .col(Blocking::BlockeeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: blocker_id (for listing blocked users)
        manager
            .create_index(
                Index::create()
                    .name("idx_blocking_blocker_id")
                    .table(Blocking::Table)
                    .col(Blocking::BlockerId)
                    .to_owned(),
            )
            .await?;

        // Index: blockee_id (for checking if blocked)
        manager
            .create_index(
                Index::create()
                    .name("idx_blocking_blockee_id")
                    .table(Blocking::Table)
                    .col(Blocking::BlockeeId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Blocking::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Blocking {
    Table,
    Id,
    BlockerId,
    BlockeeId,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
