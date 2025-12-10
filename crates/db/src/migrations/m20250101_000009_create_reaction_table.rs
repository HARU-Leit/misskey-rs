//! Create reaction table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Reaction::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Reaction::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Reaction::UserId).string_len(32).not_null())
                    .col(ColumnDef::new(Reaction::NoteId).string_len(32).not_null())
                    .col(ColumnDef::new(Reaction::Reaction).string_len(256).not_null())
                    .col(
                        ColumnDef::new(Reaction::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reaction_user")
                            .from(Reaction::Table, Reaction::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reaction_note")
                            .from(Reaction::Table, Reaction::NoteId)
                            .to(Note::Table, Note::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: (user_id, note_id) - one reaction per user per note
        manager
            .create_index(
                Index::create()
                    .name("idx_reaction_user_note")
                    .table(Reaction::Table)
                    .col(Reaction::UserId)
                    .col(Reaction::NoteId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: note_id (for listing reactions on a note)
        manager
            .create_index(
                Index::create()
                    .name("idx_reaction_note_id")
                    .table(Reaction::Table)
                    .col(Reaction::NoteId)
                    .to_owned(),
            )
            .await?;

        // Index: user_id (for listing user's reactions)
        manager
            .create_index(
                Index::create()
                    .name("idx_reaction_user_id")
                    .table(Reaction::Table)
                    .col(Reaction::UserId)
                    .to_owned(),
            )
            .await?;

        // Index: created_at (for pagination)
        manager
            .create_index(
                Index::create()
                    .name("idx_reaction_created_at")
                    .table(Reaction::Table)
                    .col(Reaction::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Reaction::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Reaction {
    Table,
    Id,
    UserId,
    NoteId,
    Reaction,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}

#[derive(Iden)]
enum Note {
    Table,
    Id,
}
