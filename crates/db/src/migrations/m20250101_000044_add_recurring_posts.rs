//! Add recurring posts feature for automatic repeated posting.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create recurring_post table
        manager
            .create_table(
                Table::create()
                    .table(RecurringPost::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecurringPost::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RecurringPost::UserId).string().not_null())
                    .col(ColumnDef::new(RecurringPost::Text).text().null())
                    .col(ColumnDef::new(RecurringPost::Cw).string().null())
                    .col(
                        ColumnDef::new(RecurringPost::Visibility)
                            .string()
                            .not_null()
                            .default("Public"),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::LocalOnly)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::FileIds)
                            .json()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(RecurringPost::Interval).string().not_null())
                    .col(
                        ColumnDef::new(RecurringPost::DayOfWeek)
                            .small_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::DayOfMonth)
                            .small_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::HourUtc)
                            .small_integer()
                            .not_null()
                            .default(12),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::MinuteUtc)
                            .small_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::Timezone)
                            .string()
                            .not_null()
                            .default("UTC"),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::LastPostedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::NextPostAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::PostCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(RecurringPost::MaxPosts).integer().null())
                    .col(
                        ColumnDef::new(RecurringPost::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecurringPost::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RecurringPost::Table, RecurringPost::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_recurring_post_user_id")
                    .table(RecurringPost::Table)
                    .col(RecurringPost::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_recurring_post_next_post_at")
                    .table(RecurringPost::Table)
                    .col(RecurringPost::NextPostAt)
                    .col(RecurringPost::IsActive)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecurringPost::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum RecurringPost {
    Table,
    Id,
    UserId,
    Text,
    Cw,
    Visibility,
    LocalOnly,
    FileIds,
    Interval,
    DayOfWeek,
    DayOfMonth,
    HourUtc,
    MinuteUtc,
    Timezone,
    IsActive,
    LastPostedAt,
    NextPostAt,
    PostCount,
    MaxPosts,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
