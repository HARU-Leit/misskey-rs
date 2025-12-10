//! Create `drive_folder` table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DriveFolder::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DriveFolder::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DriveFolder::UserId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DriveFolder::Name)
                            .string_len(256)
                            .not_null(),
                    )
                    .col(ColumnDef::new(DriveFolder::ParentId).string_len(32).null())
                    .col(
                        ColumnDef::new(DriveFolder::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_drive_folder_user")
                            .from(DriveFolder::Table, DriveFolder::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_drive_folder_parent")
                            .from(DriveFolder::Table, DriveFolder::ParentId)
                            .to(DriveFolder::Table, DriveFolder::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_drive_folder_user_id")
                    .table(DriveFolder::Table)
                    .col(DriveFolder::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_drive_folder_parent_id")
                    .table(DriveFolder::Table)
                    .col(DriveFolder::ParentId)
                    .to_owned(),
            )
            .await?;

        // Add folder_id foreign key to drive_file table
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_drive_file_folder")
                    .from(DriveFile::Table, DriveFile::FolderId)
                    .to(DriveFolder::Table, DriveFolder::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove foreign key from drive_file
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_drive_file_folder")
                    .table(DriveFile::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(DriveFolder::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum DriveFolder {
    Table,
    Id,
    UserId,
    Name,
    ParentId,
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
    FolderId,
}
