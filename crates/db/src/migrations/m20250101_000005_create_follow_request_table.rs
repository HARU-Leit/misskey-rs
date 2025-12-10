//! Create `follow_request` table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FollowRequest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FollowRequest::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(FollowRequest::FollowerId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FollowRequest::FolloweeId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(ColumnDef::new(FollowRequest::FollowerHost).string_len(256))
                    .col(ColumnDef::new(FollowRequest::FolloweeHost).string_len(256))
                    .col(ColumnDef::new(FollowRequest::FollowerInbox).string_len(1024))
                    .col(ColumnDef::new(FollowRequest::FollowerSharedInbox).string_len(1024))
                    .col(
                        ColumnDef::new(FollowRequest::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_follow_request_follower")
                            .from(FollowRequest::Table, FollowRequest::FollowerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_follow_request_followee")
                            .from(FollowRequest::Table, FollowRequest::FolloweeId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: (follower_id, followee_id) - prevent duplicate requests
        manager
            .create_index(
                Index::create()
                    .name("idx_follow_request_follower_followee")
                    .table(FollowRequest::Table)
                    .col(FollowRequest::FollowerId)
                    .col(FollowRequest::FolloweeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: followee_id (for listing pending requests)
        manager
            .create_index(
                Index::create()
                    .name("idx_follow_request_followee_id")
                    .table(FollowRequest::Table)
                    .col(FollowRequest::FolloweeId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FollowRequest::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum FollowRequest {
    Table,
    Id,
    FollowerId,
    FolloweeId,
    FollowerHost,
    FolloweeHost,
    FollowerInbox,
    FollowerSharedInbox,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
