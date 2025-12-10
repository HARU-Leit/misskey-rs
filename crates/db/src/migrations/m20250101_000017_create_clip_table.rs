//! Create clip and clip_note tables migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create clip table
        manager
            .create_table(
                Table::create()
                    .table(Clip::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Clip::Id).string_len(32).not_null().primary_key())
                    .col(ColumnDef::new(Clip::UserId).string_len(32).not_null())
                    .col(ColumnDef::new(Clip::Name).string_len(128).not_null())
                    .col(ColumnDef::new(Clip::Description).text())
                    .col(ColumnDef::new(Clip::IsPublic).boolean().not_null().default(false))
                    .col(ColumnDef::new(Clip::NotesCount).integer().not_null().default(0))
                    .col(ColumnDef::new(Clip::DisplayOrder).integer().not_null().default(0))
                    .col(
                        ColumnDef::new(Clip::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Clip::UpdatedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        // Index: user_id (for listing user's clips)
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_user_id")
                    .table(Clip::Table)
                    .col(Clip::UserId)
                    .to_owned(),
            )
            .await?;

        // Index: is_public (for listing public clips)
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_is_public")
                    .table(Clip::Table)
                    .col(Clip::IsPublic)
                    .to_owned(),
            )
            .await?;

        // Index: display_order
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_display_order")
                    .table(Clip::Table)
                    .col(Clip::DisplayOrder)
                    .to_owned(),
            )
            .await?;

        // Create clip_note table
        manager
            .create_table(
                Table::create()
                    .table(ClipNote::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ClipNote::Id).string_len(32).not_null().primary_key())
                    .col(ColumnDef::new(ClipNote::ClipId).string_len(32).not_null())
                    .col(ColumnDef::new(ClipNote::NoteId).string_len(32).not_null())
                    .col(ColumnDef::new(ClipNote::DisplayOrder).integer().not_null().default(0))
                    .col(ColumnDef::new(ClipNote::Comment).text())
                    .col(
                        ColumnDef::new(ClipNote::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_clip_note_clip")
                            .from(ClipNote::Table, ClipNote::ClipId)
                            .to(Clip::Table, Clip::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: (clip_id, note_id) - a note can only be in a clip once
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_note_unique")
                    .table(ClipNote::Table)
                    .col(ClipNote::ClipId)
                    .col(ClipNote::NoteId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: clip_id (for listing notes in a clip)
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_note_clip_id")
                    .table(ClipNote::Table)
                    .col(ClipNote::ClipId)
                    .to_owned(),
            )
            .await?;

        // Index: note_id (for finding which clips contain a note)
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_note_note_id")
                    .table(ClipNote::Table)
                    .col(ClipNote::NoteId)
                    .to_owned(),
            )
            .await?;

        // Index: display_order (for ordered listing)
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_note_display_order")
                    .table(ClipNote::Table)
                    .col(ClipNote::DisplayOrder)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ClipNote::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Clip::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Clip {
    Table,
    Id,
    UserId,
    Name,
    Description,
    IsPublic,
    NotesCount,
    DisplayOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ClipNote {
    Table,
    Id,
    ClipId,
    NoteId,
    DisplayOrder,
    Comment,
    CreatedAt,
}
