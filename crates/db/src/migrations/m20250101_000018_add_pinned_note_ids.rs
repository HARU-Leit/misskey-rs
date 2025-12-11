//! Add `pinned_note_ids` column to `user_profile` table.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .add_column(
                        ColumnDef::new(UserProfile::PinnedNoteIds)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'[]'")),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .drop_column(UserProfile::PinnedNoteIds)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum UserProfile {
    Table,
    PinnedNoteIds,
}
