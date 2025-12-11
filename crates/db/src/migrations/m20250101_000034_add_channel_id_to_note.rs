//! Migration to add `channel_id` field to note table.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add channel_id field to note table
        manager
            .alter_table(
                Table::alter()
                    .table(Note::Table)
                    .add_column(ColumnDef::new(Note::ChannelId).string_len(32).null())
                    .to_owned(),
            )
            .await?;

        // Create index for channel_id for efficient channel timeline queries
        manager
            .create_index(
                Index::create()
                    .name("idx_note_channel_id")
                    .table(Note::Table)
                    .col(Note::ChannelId)
                    .to_owned(),
            )
            .await?;

        // Create foreign key constraint
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_note_channel")
                    .from(Note::Table, Note::ChannelId)
                    .to(Channel::Table, Channel::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop foreign key
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_note_channel")
                    .table(Note::Table)
                    .to_owned(),
            )
            .await?;

        // Drop index
        manager
            .drop_index(Index::drop().name("idx_note_channel_id").to_owned())
            .await?;

        // Drop column
        manager
            .alter_table(
                Table::alter()
                    .table(Note::Table)
                    .drop_column(Note::ChannelId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Note {
    Table,
    ChannelId,
}

#[derive(Iden)]
enum Channel {
    Table,
    Id,
}
