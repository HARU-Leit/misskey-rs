//! Create gallery_post and gallery_like tables.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create gallery_post table
        manager
            .create_table(
                Table::create()
                    .table(GalleryPost::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GalleryPost::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GalleryPost::UserId).string().not_null())
                    .col(
                        ColumnDef::new(GalleryPost::Title)
                            .string_len(256)
                            .not_null(),
                    )
                    .col(ColumnDef::new(GalleryPost::Description).text().null())
                    .col(
                        ColumnDef::new(GalleryPost::FileIds)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GalleryPost::IsSensitive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(GalleryPost::Tags).json_binary().not_null())
                    .col(
                        ColumnDef::new(GalleryPost::LikedCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(GalleryPost::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GalleryPost::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_gallery_post_user")
                            .from(GalleryPost::Table, GalleryPost::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for gallery_post table
        manager
            .create_index(
                Index::create()
                    .name("idx_gallery_post_user_id")
                    .table(GalleryPost::Table)
                    .col(GalleryPost::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gallery_post_created_at")
                    .table(GalleryPost::Table)
                    .col(GalleryPost::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gallery_post_liked_count")
                    .table(GalleryPost::Table)
                    .col(GalleryPost::LikedCount)
                    .to_owned(),
            )
            .await?;

        // Create gallery_like table
        manager
            .create_table(
                Table::create()
                    .table(GalleryLike::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GalleryLike::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GalleryLike::PostId).string().not_null())
                    .col(ColumnDef::new(GalleryLike::UserId).string().not_null())
                    .col(
                        ColumnDef::new(GalleryLike::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_gallery_like_post")
                            .from(GalleryLike::Table, GalleryLike::PostId)
                            .to(GalleryPost::Table, GalleryPost::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_gallery_like_user")
                            .from(GalleryLike::Table, GalleryLike::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for gallery_like table
        manager
            .create_index(
                Index::create()
                    .name("idx_gallery_like_post_id")
                    .table(GalleryLike::Table)
                    .col(GalleryLike::PostId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_gallery_like_user_id")
                    .table(GalleryLike::Table)
                    .col(GalleryLike::UserId)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on post_id + user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_gallery_like_unique")
                    .table(GalleryLike::Table)
                    .col(GalleryLike::PostId)
                    .col(GalleryLike::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GalleryLike::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(GalleryPost::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum GalleryPost {
    Table,
    Id,
    UserId,
    Title,
    Description,
    FileIds,
    IsSensitive,
    Tags,
    LikedCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum GalleryLike {
    Table,
    Id,
    PostId,
    UserId,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
