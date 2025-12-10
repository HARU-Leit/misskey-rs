//! Create push_subscription table for Web Push notifications.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PushSubscription::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PushSubscription::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PushSubscription::UserId).string().not_null())
                    .col(ColumnDef::new(PushSubscription::Endpoint).text().not_null())
                    .col(ColumnDef::new(PushSubscription::Auth).string().not_null())
                    .col(ColumnDef::new(PushSubscription::P256dh).string().not_null())
                    .col(
                        ColumnDef::new(PushSubscription::Types)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushSubscription::Active)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(PushSubscription::UserAgent).string().null())
                    .col(ColumnDef::new(PushSubscription::DeviceName).string().null())
                    .col(
                        ColumnDef::new(PushSubscription::QuietHoursStart)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PushSubscription::QuietHoursEnd)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PushSubscription::LastPushedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PushSubscription::FailCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(PushSubscription::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PushSubscription::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_push_subscription_user")
                            .from(PushSubscription::Table, PushSubscription::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on user_id for listing user's subscriptions
        manager
            .create_index(
                Index::create()
                    .name("idx_push_subscription_user_id")
                    .table(PushSubscription::Table)
                    .col(PushSubscription::UserId)
                    .to_owned(),
            )
            .await?;

        // Unique index on endpoint to prevent duplicate subscriptions
        manager
            .create_index(
                Index::create()
                    .name("idx_push_subscription_endpoint")
                    .table(PushSubscription::Table)
                    .col(PushSubscription::Endpoint)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on active for filtering active subscriptions
        manager
            .create_index(
                Index::create()
                    .name("idx_push_subscription_active")
                    .table(PushSubscription::Table)
                    .col(PushSubscription::Active)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PushSubscription::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum PushSubscription {
    Table,
    Id,
    UserId,
    Endpoint,
    Auth,
    P256dh,
    Types,
    Active,
    UserAgent,
    DeviceName,
    QuietHoursStart,
    QuietHoursEnd,
    LastPushedAt,
    FailCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
