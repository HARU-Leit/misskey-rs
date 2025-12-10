//! Create following table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Following::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Following::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Following::FollowerId).string_len(32).not_null())
                    .col(ColumnDef::new(Following::FolloweeId).string_len(32).not_null())
                    .col(ColumnDef::new(Following::FollowerHost).string_len(256))
                    .col(ColumnDef::new(Following::FolloweeHost).string_len(256))
                    .col(ColumnDef::new(Following::FolloweeInbox).string_len(1024))
                    .col(ColumnDef::new(Following::FolloweeSharedInbox).string_len(1024))
                    .col(
                        ColumnDef::new(Following::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_following_follower")
                            .from(Following::Table, Following::FollowerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_following_followee")
                            .from(Following::Table, Following::FolloweeId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique index: (follower_id, followee_id) - prevent duplicate follows
        manager
            .create_index(
                Index::create()
                    .name("idx_following_follower_followee")
                    .table(Following::Table)
                    .col(Following::FollowerId)
                    .col(Following::FolloweeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index: followee_id (for listing followers)
        manager
            .create_index(
                Index::create()
                    .name("idx_following_followee_id")
                    .table(Following::Table)
                    .col(Following::FolloweeId)
                    .to_owned(),
            )
            .await?;

        // Index: follower_id (for listing following)
        manager
            .create_index(
                Index::create()
                    .name("idx_following_follower_id")
                    .table(Following::Table)
                    .col(Following::FollowerId)
                    .to_owned(),
            )
            .await?;

        // Index: created_at (for pagination)
        manager
            .create_index(
                Index::create()
                    .name("idx_following_created_at")
                    .table(Following::Table)
                    .col(Following::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Following::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Following {
    Table,
    Id,
    FollowerId,
    FolloweeId,
    FollowerHost,
    FolloweeHost,
    FolloweeInbox,
    FolloweeSharedInbox,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
