//! Create webhook table for event notifications.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create webhook table
        manager
            .create_table(
                Table::create()
                    .table(Webhook::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Webhook::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Webhook::UserId).string().not_null())
                    .col(ColumnDef::new(Webhook::Name).string().not_null())
                    .col(ColumnDef::new(Webhook::Url).text().not_null())
                    .col(ColumnDef::new(Webhook::Secret).string().not_null())
                    .col(ColumnDef::new(Webhook::Events).json_binary().not_null())
                    .col(
                        ColumnDef::new(Webhook::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Webhook::LastTriggeredAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Webhook::FailureCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Webhook::LastError).text().null())
                    .col(
                        ColumnDef::new(Webhook::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Webhook::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_webhook_user")
                            .from(Webhook::Table, Webhook::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_user_id")
                    .table(Webhook::Table)
                    .col(Webhook::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on is_active
        manager
            .create_index(
                Index::create()
                    .name("idx_webhook_is_active")
                    .table(Webhook::Table)
                    .col(Webhook::IsActive)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Webhook::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Webhook {
    Table,
    Id,
    UserId,
    Name,
    Url,
    Secret,
    Events,
    IsActive,
    LastTriggeredAt,
    FailureCount,
    LastError,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum User {
    Table,
    Id,
}
