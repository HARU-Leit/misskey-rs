//! Create `drive_file` table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DriveFile::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DriveFile::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DriveFile::UserId).string_len(32).not_null())
                    .col(ColumnDef::new(DriveFile::UserHost).string_len(256))
                    .col(ColumnDef::new(DriveFile::Name).string_len(256).not_null())
                    .col(
                        ColumnDef::new(DriveFile::ContentType)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(ColumnDef::new(DriveFile::Size).big_integer().not_null())
                    .col(ColumnDef::new(DriveFile::Url).string_len(1024).not_null())
                    .col(ColumnDef::new(DriveFile::ThumbnailUrl).string_len(1024))
                    .col(ColumnDef::new(DriveFile::WebpublicUrl).string_len(1024))
                    .col(ColumnDef::new(DriveFile::Blurhash).string_len(128))
                    .col(ColumnDef::new(DriveFile::Width).integer())
                    .col(ColumnDef::new(DriveFile::Height).integer())
                    .col(ColumnDef::new(DriveFile::Comment).text())
                    .col(
                        ColumnDef::new(DriveFile::IsSensitive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(DriveFile::IsLink)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(DriveFile::Md5).string_len(32))
                    .col(ColumnDef::new(DriveFile::StorageKey).string_len(256))
                    .col(ColumnDef::new(DriveFile::FolderId).string_len(32))
                    .col(ColumnDef::new(DriveFile::Uri).string_len(1024))
                    .col(
                        ColumnDef::new(DriveFile::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_drive_file_user")
                            .from(DriveFile::Table, DriveFile::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index: user_id (for listing user's files)
        manager
            .create_index(
                Index::create()
                    .name("idx_drive_file_user_id")
                    .table(DriveFile::Table)
                    .col(DriveFile::UserId)
                    .to_owned(),
            )
            .await?;

        // Index: folder_id (for listing folder contents)
        manager
            .create_index(
                Index::create()
                    .name("idx_drive_file_folder_id")
                    .table(DriveFile::Table)
                    .col(DriveFile::FolderId)
                    .to_owned(),
            )
            .await?;

        // Index: md5 (for duplicate detection)
        manager
            .create_index(
                Index::create()
                    .name("idx_drive_file_md5")
                    .table(DriveFile::Table)
                    .col(DriveFile::Md5)
                    .to_owned(),
            )
            .await?;

        // Index: uri (for ActivityPub lookups)
        manager
            .create_index(
                Index::create()
                    .name("idx_drive_file_uri")
                    .table(DriveFile::Table)
                    .col(DriveFile::Uri)
                    .to_owned(),
            )
            .await?;

        // Index: created_at (for pagination)
        manager
            .create_index(
                Index::create()
                    .name("idx_drive_file_created_at")
                    .table(DriveFile::Table)
                    .col(DriveFile::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DriveFile::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum DriveFile {
    Table,
    Id,
    UserId,
    UserHost,
    Name,
    ContentType,
    Size,
    Url,
    ThumbnailUrl,
    WebpublicUrl,
    Blurhash,
    Width,
    Height,
    Comment,
    IsSensitive,
    IsLink,
    Md5,
    StorageKey,
    FolderId,
    Uri,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
