//! Create `note_edit` table for edit history.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create note_edit table
        manager
            .create_table(
                Table::create()
                    .table(NoteEdit::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NoteEdit::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NoteEdit::NoteId).string_len(32).not_null())
                    .col(ColumnDef::new(NoteEdit::OldText).text())
                    .col(ColumnDef::new(NoteEdit::NewText).text())
                    .col(ColumnDef::new(NoteEdit::OldCw).string_len(512))
                    .col(ColumnDef::new(NoteEdit::NewCw).string_len(512))
                    .col(
                        ColumnDef::new(NoteEdit::OldFileIds)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'[]'")),
                    )
                    .col(
                        ColumnDef::new(NoteEdit::NewFileIds)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'[]'")),
                    )
                    .col(
                        ColumnDef::new(NoteEdit::EditedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_note_edit_note")
                            .from(NoteEdit::Table, NoteEdit::NoteId)
                            .to(Note::Table, Note::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on note_id for quick history lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_note_edit_note_id")
                    .table(NoteEdit::Table)
                    .col(NoteEdit::NoteId)
                    .to_owned(),
            )
            .await?;

        // Create index on edited_at for chronological ordering
        manager
            .create_index(
                Index::create()
                    .name("idx_note_edit_edited_at")
                    .table(NoteEdit::Table)
                    .col(NoteEdit::EditedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NoteEdit::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum NoteEdit {
    Table,
    Id,
    NoteId,
    OldText,
    NewText,
    OldCw,
    NewCw,
    OldFileIds,
    NewFileIds,
    EditedAt,
}

#[derive(Iden)]
enum Note {
    Table,
    Id,
}
