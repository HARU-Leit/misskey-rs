//! Create notification table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Notification::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Notification::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Notification::NotifieeId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Notification::NotifierId).string_len(32))
                    .col(
                        ColumnDef::new(Notification::NotificationType)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Notification::NoteId).string_len(32))
                    .col(ColumnDef::new(Notification::FollowRequestId).string_len(32))
                    .col(ColumnDef::new(Notification::Reaction).string_len(256))
                    .col(ColumnDef::new(Notification::CustomData).json_binary())
                    .col(
                        ColumnDef::new(Notification::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Notification::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_notifiee")
                            .from(Notification::Table, Notification::NotifieeId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_notifier")
                            .from(Notification::Table, Notification::NotifierId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_note")
                            .from(Notification::Table, Notification::NoteId)
                            .to(Note::Table, Note::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index: notifiee_id (for listing user's notifications)
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_notifiee_id")
                    .table(Notification::Table)
                    .col(Notification::NotifieeId)
                    .to_owned(),
            )
            .await?;

        // Index: (notifiee_id, is_read) (for unread count)
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_notifiee_is_read")
                    .table(Notification::Table)
                    .col(Notification::NotifieeId)
                    .col(Notification::IsRead)
                    .to_owned(),
            )
            .await?;

        // Index: created_at (for pagination)
        manager
            .create_index(
                Index::create()
                    .name("idx_notification_created_at")
                    .table(Notification::Table)
                    .col(Notification::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Notification::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Notification {
    Table,
    Id,
    NotifieeId,
    NotifierId,
    NotificationType,
    NoteId,
    FollowRequestId,
    Reaction,
    CustomData,
    IsRead,
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
