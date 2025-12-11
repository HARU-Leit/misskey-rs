//! Create instance table for federation management.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create instance table
        manager
            .create_table(
                Table::create()
                    .table(Instance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Instance::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Instance::Host)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Instance::UsersCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Instance::NotesCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Instance::FollowingCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Instance::FollowersCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Instance::SoftwareName).string().null())
                    .col(ColumnDef::new(Instance::SoftwareVersion).string().null())
                    .col(ColumnDef::new(Instance::Name).string().null())
                    .col(ColumnDef::new(Instance::Description).text().null())
                    .col(ColumnDef::new(Instance::MaintainerEmail).string().null())
                    .col(ColumnDef::new(Instance::MaintainerName).string().null())
                    .col(ColumnDef::new(Instance::IconUrl).string().null())
                    .col(ColumnDef::new(Instance::FaviconUrl).string().null())
                    .col(ColumnDef::new(Instance::ThemeColor).string().null())
                    .col(
                        ColumnDef::new(Instance::IsBlocked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Instance::IsSilenced)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Instance::IsSuspended)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Instance::ModerationNote).text().null())
                    .col(
                        ColumnDef::new(Instance::LastCommunicatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Instance::InfoUpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Instance::IsNodeinfoFetched)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Instance::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Instance::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on is_blocked for efficient filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_instance_is_blocked")
                    .table(Instance::Table)
                    .col(Instance::IsBlocked)
                    .to_owned(),
            )
            .await?;

        // Create index on is_silenced for efficient filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_instance_is_silenced")
                    .table(Instance::Table)
                    .col(Instance::IsSilenced)
                    .to_owned(),
            )
            .await?;

        // Create index on host for lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_instance_host")
                    .table(Instance::Table)
                    .col(Instance::Host)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Instance::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Instance {
    Table,
    Id,
    Host,
    UsersCount,
    NotesCount,
    FollowingCount,
    FollowersCount,
    SoftwareName,
    SoftwareVersion,
    Name,
    Description,
    MaintainerEmail,
    MaintainerName,
    IconUrl,
    FaviconUrl,
    ThemeColor,
    IsBlocked,
    IsSilenced,
    IsSuspended,
    ModerationNote,
    LastCommunicatedAt,
    InfoUpdatedAt,
    IsNodeinfoFetched,
    CreatedAt,
    UpdatedAt,
}
