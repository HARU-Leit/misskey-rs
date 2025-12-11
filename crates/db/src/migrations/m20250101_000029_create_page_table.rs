//! Create page and page_like tables.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create page table
        manager
            .create_table(
                Table::create()
                    .table(Page::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Page::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Page::UserId).string().not_null())
                    .col(ColumnDef::new(Page::Name).string_len(256).not_null())
                    .col(ColumnDef::new(Page::Title).string_len(256).not_null())
                    .col(ColumnDef::new(Page::Summary).text().null())
                    .col(ColumnDef::new(Page::Content).json_binary().not_null())
                    .col(ColumnDef::new(Page::Variables).json_binary().not_null())
                    .col(ColumnDef::new(Page::Script).text().null())
                    .col(
                        ColumnDef::new(Page::Visibility)
                            .string_len(16)
                            .not_null()
                            .default("public"),
                    )
                    .col(
                        ColumnDef::new(Page::VisibleUserIds)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Page::EyecatchImageId).string().null())
                    .col(ColumnDef::new(Page::FileIds).json_binary().not_null())
                    .col(ColumnDef::new(Page::Font).string_len(32).null())
                    .col(
                        ColumnDef::new(Page::HideTitleWhenPinned)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Page::AlignCenter)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Page::LikedCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Page::ViewCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Page::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Page::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_page_user")
                            .from(Page::Table, Page::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_page_eyecatch")
                            .from(Page::Table, Page::EyecatchImageId)
                            .to(DriveFile::Table, DriveFile::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for page table
        manager
            .create_index(
                Index::create()
                    .name("idx_page_user_id")
                    .table(Page::Table)
                    .col(Page::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_page_name")
                    .table(Page::Table)
                    .col(Page::Name)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on user_id + name
        manager
            .create_index(
                Index::create()
                    .name("idx_page_user_name_unique")
                    .table(Page::Table)
                    .col(Page::UserId)
                    .col(Page::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_page_created_at")
                    .table(Page::Table)
                    .col(Page::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Create page_like table
        manager
            .create_table(
                Table::create()
                    .table(PageLike::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PageLike::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PageLike::PageId).string().not_null())
                    .col(ColumnDef::new(PageLike::UserId).string().not_null())
                    .col(
                        ColumnDef::new(PageLike::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_page_like_page")
                            .from(PageLike::Table, PageLike::PageId)
                            .to(Page::Table, Page::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_page_like_user")
                            .from(PageLike::Table, PageLike::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for page_like table
        manager
            .create_index(
                Index::create()
                    .name("idx_page_like_page_id")
                    .table(PageLike::Table)
                    .col(PageLike::PageId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_page_like_user_id")
                    .table(PageLike::Table)
                    .col(PageLike::UserId)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on page_id + user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_page_like_unique")
                    .table(PageLike::Table)
                    .col(PageLike::PageId)
                    .col(PageLike::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PageLike::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Page::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Page {
    Table,
    Id,
    UserId,
    Name,
    Title,
    Summary,
    Content,
    Variables,
    Script,
    Visibility,
    VisibleUserIds,
    EyecatchImageId,
    FileIds,
    Font,
    HideTitleWhenPinned,
    AlignCenter,
    LikedCount,
    ViewCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum PageLike {
    Table,
    Id,
    PageId,
    UserId,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}

#[derive(Iden)]
enum DriveFile {
    Table,
    Id,
}
