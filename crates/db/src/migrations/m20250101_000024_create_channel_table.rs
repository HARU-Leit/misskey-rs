//! Create channel and `channel_following` tables.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create channel table
        manager
            .create_table(
                Table::create()
                    .table(Channel::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Channel::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Channel::UserId).string_len(32).not_null())
                    .col(ColumnDef::new(Channel::Name).string_len(128).not_null())
                    .col(ColumnDef::new(Channel::Description).text())
                    .col(ColumnDef::new(Channel::BannerId).string_len(32))
                    .col(ColumnDef::new(Channel::Color).string_len(16))
                    .col(
                        ColumnDef::new(Channel::IsArchived)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Channel::IsSearchable)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Channel::AllowAnyoneToPost)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Channel::NotesCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Channel::UsersCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Channel::LastNotedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Channel::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Channel::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_channel_user")
                            .from(Channel::Table, Channel::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_channel_banner")
                            .from(Channel::Table, Channel::BannerId)
                            .to(DriveFile::Table, DriveFile::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_user_id")
                    .table(Channel::Table)
                    .col(Channel::UserId)
                    .to_owned(),
            )
            .await?;

        // Index on is_archived for filtering active channels
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_is_archived")
                    .table(Channel::Table)
                    .col(Channel::IsArchived)
                    .to_owned(),
            )
            .await?;

        // Index on notes_count for sorting by popularity
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_notes_count")
                    .table(Channel::Table)
                    .col(Channel::NotesCount)
                    .to_owned(),
            )
            .await?;

        // Create channel_following table
        manager
            .create_table(
                Table::create()
                    .table(ChannelFollowing::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChannelFollowing::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ChannelFollowing::UserId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChannelFollowing::ChannelId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ChannelFollowing::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(ChannelFollowing::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_channel_following_user")
                            .from(ChannelFollowing::Table, ChannelFollowing::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_channel_following_channel")
                            .from(ChannelFollowing::Table, ChannelFollowing::ChannelId)
                            .to(Channel::Table, Channel::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_following_user_id")
                    .table(ChannelFollowing::Table)
                    .col(ChannelFollowing::UserId)
                    .to_owned(),
            )
            .await?;

        // Index on channel_id
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_following_channel_id")
                    .table(ChannelFollowing::Table)
                    .col(ChannelFollowing::ChannelId)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on (user_id, channel_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_following_unique")
                    .table(ChannelFollowing::Table)
                    .col(ChannelFollowing::UserId)
                    .col(ChannelFollowing::ChannelId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChannelFollowing::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Channel::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Channel {
    Table,
    Id,
    UserId,
    Name,
    Description,
    BannerId,
    Color,
    IsArchived,
    IsSearchable,
    AllowAnyoneToPost,
    NotesCount,
    UsersCount,
    LastNotedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ChannelFollowing {
    Table,
    Id,
    UserId,
    ChannelId,
    IsRead,
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
