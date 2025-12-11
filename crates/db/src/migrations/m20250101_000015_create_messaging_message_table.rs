//! Create `messaging_message` table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create messaging_message table
        manager
            .create_table(
                Table::create()
                    .table(MessagingMessage::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MessagingMessage::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MessagingMessage::UserId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(MessagingMessage::RecipientId).string_len(32))
                    .col(ColumnDef::new(MessagingMessage::GroupId).string_len(32))
                    .col(ColumnDef::new(MessagingMessage::Text).text())
                    .col(ColumnDef::new(MessagingMessage::FileId).string_len(32))
                    .col(
                        ColumnDef::new(MessagingMessage::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(MessagingMessage::Uri).string_len(512))
                    .col(
                        ColumnDef::new(MessagingMessage::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messaging_message_user")
                            .from(MessagingMessage::Table, MessagingMessage::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index: user_id (sender)
        manager
            .create_index(
                Index::create()
                    .name("idx_messaging_message_user_id")
                    .table(MessagingMessage::Table)
                    .col(MessagingMessage::UserId)
                    .to_owned(),
            )
            .await?;

        // Index: recipient_id
        manager
            .create_index(
                Index::create()
                    .name("idx_messaging_message_recipient_id")
                    .table(MessagingMessage::Table)
                    .col(MessagingMessage::RecipientId)
                    .to_owned(),
            )
            .await?;

        // Index: group_id
        manager
            .create_index(
                Index::create()
                    .name("idx_messaging_message_group_id")
                    .table(MessagingMessage::Table)
                    .col(MessagingMessage::GroupId)
                    .to_owned(),
            )
            .await?;

        // Index: created_at for sorting
        manager
            .create_index(
                Index::create()
                    .name("idx_messaging_message_created_at")
                    .table(MessagingMessage::Table)
                    .col(MessagingMessage::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Composite index for conversation lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_messaging_message_conversation")
                    .table(MessagingMessage::Table)
                    .col(MessagingMessage::UserId)
                    .col(MessagingMessage::RecipientId)
                    .col(MessagingMessage::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MessagingMessage::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum MessagingMessage {
    Table,
    Id,
    UserId,
    RecipientId,
    GroupId,
    Text,
    FileId,
    IsRead,
    Uri,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
