//! Create `scheduled_note` table.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ScheduledNote::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ScheduledNote::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ScheduledNote::UserId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ScheduledNote::Text).text())
                    .col(ColumnDef::new(ScheduledNote::Cw).string_len(512))
                    .col(
                        ColumnDef::new(ScheduledNote::Visibility)
                            .string_len(16)
                            .not_null()
                            .default("public"),
                    )
                    .col(
                        ColumnDef::new(ScheduledNote::VisibleUserIds)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(ScheduledNote::FileIds)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(ScheduledNote::ReplyId).string_len(32))
                    .col(ColumnDef::new(ScheduledNote::RenoteId).string_len(32))
                    .col(ColumnDef::new(ScheduledNote::Poll).json_binary())
                    .col(
                        ColumnDef::new(ScheduledNote::ScheduledAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledNote::Status)
                            .string_len(16)
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(ScheduledNote::PostedNoteId).string_len(32))
                    .col(ColumnDef::new(ScheduledNote::ErrorMessage).text())
                    .col(
                        ColumnDef::new(ScheduledNote::RetryCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ScheduledNote::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(ScheduledNote::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scheduled_note_user")
                            .from(ScheduledNote::Table, ScheduledNote::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scheduled_note_reply")
                            .from(ScheduledNote::Table, ScheduledNote::ReplyId)
                            .to(Note::Table, Note::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scheduled_note_renote")
                            .from(ScheduledNote::Table, ScheduledNote::RenoteId)
                            .to(Note::Table, Note::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on user_id for listing user's scheduled notes
        manager
            .create_index(
                Index::create()
                    .name("idx_scheduled_note_user_id")
                    .table(ScheduledNote::Table)
                    .col(ScheduledNote::UserId)
                    .to_owned(),
            )
            .await?;

        // Index on scheduled_at for finding notes to post
        manager
            .create_index(
                Index::create()
                    .name("idx_scheduled_note_scheduled_at")
                    .table(ScheduledNote::Table)
                    .col(ScheduledNote::ScheduledAt)
                    .to_owned(),
            )
            .await?;

        // Index on status for filtering by status
        manager
            .create_index(
                Index::create()
                    .name("idx_scheduled_note_status")
                    .table(ScheduledNote::Table)
                    .col(ScheduledNote::Status)
                    .to_owned(),
            )
            .await?;

        // Composite index for finding pending notes due for posting
        manager
            .create_index(
                Index::create()
                    .name("idx_scheduled_note_status_scheduled_at")
                    .table(ScheduledNote::Table)
                    .col(ScheduledNote::Status)
                    .col(ScheduledNote::ScheduledAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ScheduledNote::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum ScheduledNote {
    Table,
    Id,
    UserId,
    Text,
    Cw,
    Visibility,
    VisibleUserIds,
    FileIds,
    ReplyId,
    RenoteId,
    Poll,
    ScheduledAt,
    Status,
    PostedNoteId,
    ErrorMessage,
    RetryCount,
    CreatedAt,
    UpdatedAt,
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
