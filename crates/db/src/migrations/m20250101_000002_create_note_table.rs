//! Create note table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Note::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Note::Id).string_len(32).not_null().primary_key())
                    .col(ColumnDef::new(Note::UserId).string_len(32).not_null())
                    .col(ColumnDef::new(Note::UserHost).string_len(256))
                    .col(ColumnDef::new(Note::Text).text())
                    .col(ColumnDef::new(Note::Cw).string_len(512))
                    .col(ColumnDef::new(Note::Visibility).string_len(16).not_null().default("public"))
                    .col(ColumnDef::new(Note::ReplyId).string_len(32))
                    .col(ColumnDef::new(Note::RenoteId).string_len(32))
                    .col(ColumnDef::new(Note::ThreadId).string_len(32))
                    .col(ColumnDef::new(Note::Mentions).json_binary().not_null().default("[]"))
                    .col(ColumnDef::new(Note::VisibleUserIds).json_binary().not_null().default("[]"))
                    .col(ColumnDef::new(Note::FileIds).json_binary().not_null().default("[]"))
                    .col(ColumnDef::new(Note::Tags).json_binary().not_null().default("[]"))
                    .col(ColumnDef::new(Note::Reactions).json_binary().not_null().default("{}"))
                    .col(ColumnDef::new(Note::RepliesCount).integer().not_null().default(0))
                    .col(ColumnDef::new(Note::RenoteCount).integer().not_null().default(0))
                    .col(ColumnDef::new(Note::ReactionCount).integer().not_null().default(0))
                    .col(ColumnDef::new(Note::IsLocal).boolean().not_null().default(true))
                    .col(ColumnDef::new(Note::Uri).string_len(1024))
                    .col(ColumnDef::new(Note::Url).string_len(1024))
                    .col(
                        ColumnDef::new(Note::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Note::UpdatedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        // Composite index: (user_id, id) for user timeline
        manager
            .create_index(
                Index::create()
                    .name("idx_note_user_id_id")
                    .table(Note::Table)
                    .col(Note::UserId)
                    .col(Note::Id)
                    .to_owned(),
            )
            .await?;

        // Index: reply_id
        manager
            .create_index(
                Index::create()
                    .name("idx_note_reply_id")
                    .table(Note::Table)
                    .col(Note::ReplyId)
                    .to_owned(),
            )
            .await?;

        // Index: renote_id
        manager
            .create_index(
                Index::create()
                    .name("idx_note_renote_id")
                    .table(Note::Table)
                    .col(Note::RenoteId)
                    .to_owned(),
            )
            .await?;

        // Index: thread_id
        manager
            .create_index(
                Index::create()
                    .name("idx_note_thread_id")
                    .table(Note::Table)
                    .col(Note::ThreadId)
                    .to_owned(),
            )
            .await?;

        // Index: visibility + created_at (for public timeline)
        manager
            .create_index(
                Index::create()
                    .name("idx_note_visibility_created_at")
                    .table(Note::Table)
                    .col(Note::Visibility)
                    .col(Note::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Index: user_host (for local/remote filtering)
        manager
            .create_index(
                Index::create()
                    .name("idx_note_user_host")
                    .table(Note::Table)
                    .col(Note::UserHost)
                    .to_owned(),
            )
            .await?;

        // Foreign key: user_id -> user.id
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_note_user_id")
                    .from(Note::Table, Note::UserId)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Note::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Note {
    Table,
    Id,
    UserId,
    UserHost,
    Text,
    Cw,
    Visibility,
    ReplyId,
    RenoteId,
    ThreadId,
    Mentions,
    VisibleUserIds,
    FileIds,
    Tags,
    Reactions,
    RepliesCount,
    RenoteCount,
    ReactionCount,
    IsLocal,
    Uri,
    Url,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
