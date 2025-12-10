//! Create announcement and `announcement_read` tables migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create announcement table
        manager
            .create_table(
                Table::create()
                    .table(Announcement::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Announcement::Id).string_len(32).not_null().primary_key())
                    .col(ColumnDef::new(Announcement::Title).string_len(256).not_null())
                    .col(ColumnDef::new(Announcement::Text).text().not_null())
                    .col(ColumnDef::new(Announcement::ImageUrl).string_len(1024))
                    .col(ColumnDef::new(Announcement::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(Announcement::NeedsConfirmationToRead).boolean().not_null().default(false))
                    .col(ColumnDef::new(Announcement::DisplayOrder).integer().not_null().default(0))
                    .col(ColumnDef::new(Announcement::Icon).string_len(64))
                    .col(ColumnDef::new(Announcement::ForegroundColor).string_len(16))
                    .col(ColumnDef::new(Announcement::BackgroundColor).string_len(16))
                    .col(ColumnDef::new(Announcement::StartsAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Announcement::EndsAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Announcement::ReadsCount).integer().not_null().default(0))
                    .col(
                        ColumnDef::new(Announcement::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Announcement::UpdatedAt).timestamp_with_time_zone())
                    .to_owned(),
            )
            .await?;

        // Index: is_active (for filtering active announcements)
        manager
            .create_index(
                Index::create()
                    .name("idx_announcement_is_active")
                    .table(Announcement::Table)
                    .col(Announcement::IsActive)
                    .to_owned(),
            )
            .await?;

        // Index: display_order
        manager
            .create_index(
                Index::create()
                    .name("idx_announcement_display_order")
                    .table(Announcement::Table)
                    .col(Announcement::DisplayOrder)
                    .to_owned(),
            )
            .await?;

        // Index: created_at
        manager
            .create_index(
                Index::create()
                    .name("idx_announcement_created_at")
                    .table(Announcement::Table)
                    .col(Announcement::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Create announcement_read table
        manager
            .create_table(
                Table::create()
                    .table(AnnouncementRead::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AnnouncementRead::Id).string_len(32).not_null().primary_key())
                    .col(ColumnDef::new(AnnouncementRead::AnnouncementId).string_len(32).not_null())
                    .col(ColumnDef::new(AnnouncementRead::UserId).string_len(32).not_null())
                    .col(
                        ColumnDef::new(AnnouncementRead::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_announcement_read_announcement")
                            .from(AnnouncementRead::Table, AnnouncementRead::AnnouncementId)
                            .to(Announcement::Table, Announcement::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: (announcement_id, user_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_announcement_read_unique")
                    .table(AnnouncementRead::Table)
                    .col(AnnouncementRead::AnnouncementId)
                    .col(AnnouncementRead::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_announcement_read_user_id")
                    .table(AnnouncementRead::Table)
                    .col(AnnouncementRead::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AnnouncementRead::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Announcement::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Announcement {
    Table,
    Id,
    Title,
    Text,
    ImageUrl,
    IsActive,
    NeedsConfirmationToRead,
    DisplayOrder,
    Icon,
    ForegroundColor,
    BackgroundColor,
    StartsAt,
    EndsAt,
    ReadsCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum AnnouncementRead {
    Table,
    Id,
    AnnouncementId,
    UserId,
    CreatedAt,
}
