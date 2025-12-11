//! Create group, `group_member`, and `group_invite` tables.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create group table
        manager
            .create_table(
                Table::create()
                    .table(Group::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Group::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Group::OwnerId).string_len(32).not_null())
                    .col(ColumnDef::new(Group::Name).string_len(128).not_null())
                    .col(ColumnDef::new(Group::Description).text())
                    .col(ColumnDef::new(Group::BannerId).string_len(32))
                    .col(ColumnDef::new(Group::AvatarId).string_len(32))
                    .col(
                        ColumnDef::new(Group::JoinPolicy)
                            .string_len(20)
                            .not_null()
                            .default("invite_only"),
                    )
                    .col(
                        ColumnDef::new(Group::IsArchived)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Group::IsSearchable)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Group::MembersOnlyPost)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Group::MembersCount)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Group::NotesCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Group::Rules).text())
                    .col(ColumnDef::new(Group::Metadata).json_binary())
                    .col(
                        ColumnDef::new(Group::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Group::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_owner")
                            .from(Group::Table, Group::OwnerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_banner")
                            .from(Group::Table, Group::BannerId)
                            .to(DriveFile::Table, DriveFile::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_avatar")
                            .from(Group::Table, Group::AvatarId)
                            .to(DriveFile::Table, DriveFile::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for group table
        manager
            .create_index(
                Index::create()
                    .name("idx_group_owner_id")
                    .table(Group::Table)
                    .col(Group::OwnerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_group_is_archived")
                    .table(Group::Table)
                    .col(Group::IsArchived)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_group_members_count")
                    .table(Group::Table)
                    .col(Group::MembersCount)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_group_created_at")
                    .table(Group::Table)
                    .col(Group::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Create group_member table
        manager
            .create_table(
                Table::create()
                    .table(GroupMember::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GroupMember::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GroupMember::UserId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GroupMember::GroupId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GroupMember::Role)
                            .string_len(20)
                            .not_null()
                            .default("member"),
                    )
                    .col(
                        ColumnDef::new(GroupMember::IsMuted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(GroupMember::IsBanned)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(GroupMember::Nickname).string_len(64))
                    .col(
                        ColumnDef::new(GroupMember::JoinedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(GroupMember::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_member_user")
                            .from(GroupMember::Table, GroupMember::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_member_group")
                            .from(GroupMember::Table, GroupMember::GroupId)
                            .to(Group::Table, Group::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for group_member table
        manager
            .create_index(
                Index::create()
                    .name("idx_group_member_user_id")
                    .table(GroupMember::Table)
                    .col(GroupMember::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_group_member_group_id")
                    .table(GroupMember::Table)
                    .col(GroupMember::GroupId)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on (user_id, group_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_group_member_unique")
                    .table(GroupMember::Table)
                    .col(GroupMember::UserId)
                    .col(GroupMember::GroupId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create group_invite table
        manager
            .create_table(
                Table::create()
                    .table(GroupInvite::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GroupInvite::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GroupInvite::GroupId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GroupInvite::UserId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(GroupInvite::InviterId).string_len(32))
                    .col(
                        ColumnDef::new(GroupInvite::InviteType)
                            .string_len(20)
                            .not_null()
                            .default("invite"),
                    )
                    .col(
                        ColumnDef::new(GroupInvite::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(GroupInvite::Message).text())
                    .col(ColumnDef::new(GroupInvite::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(GroupInvite::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(GroupInvite::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_invite_group")
                            .from(GroupInvite::Table, GroupInvite::GroupId)
                            .to(Group::Table, Group::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_invite_user")
                            .from(GroupInvite::Table, GroupInvite::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_group_invite_inviter")
                            .from(GroupInvite::Table, GroupInvite::InviterId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for group_invite table
        manager
            .create_index(
                Index::create()
                    .name("idx_group_invite_group_id")
                    .table(GroupInvite::Table)
                    .col(GroupInvite::GroupId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_group_invite_user_id")
                    .table(GroupInvite::Table)
                    .col(GroupInvite::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_group_invite_status")
                    .table(GroupInvite::Table)
                    .col(GroupInvite::Status)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on pending (user_id, group_id, status='pending')
        // Using partial index for pending invites
        manager
            .create_index(
                Index::create()
                    .name("idx_group_invite_pending_unique")
                    .table(GroupInvite::Table)
                    .col(GroupInvite::UserId)
                    .col(GroupInvite::GroupId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GroupInvite::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(GroupMember::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Group::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Group {
    Table,
    Id,
    OwnerId,
    Name,
    Description,
    BannerId,
    AvatarId,
    JoinPolicy,
    IsArchived,
    IsSearchable,
    MembersOnlyPost,
    MembersCount,
    NotesCount,
    Rules,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum GroupMember {
    Table,
    Id,
    UserId,
    GroupId,
    Role,
    IsMuted,
    IsBanned,
    Nickname,
    JoinedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum GroupInvite {
    Table,
    Id,
    GroupId,
    UserId,
    InviterId,
    InviteType,
    Status,
    Message,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
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
