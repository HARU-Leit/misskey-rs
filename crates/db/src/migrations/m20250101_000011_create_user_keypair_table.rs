//! Create `user_keypair` table migration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserKeypair::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserKeypair::UserId)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserKeypair::PublicKey).text().not_null())
                    .col(ColumnDef::new(UserKeypair::PrivateKey).text().not_null())
                    .col(
                        ColumnDef::new(UserKeypair::KeyId)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserKeypair::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_keypair_user")
                            .from(UserKeypair::Table, UserKeypair::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on key_id for lookup during signature verification
        manager
            .create_index(
                Index::create()
                    .name("idx_user_keypair_key_id")
                    .table(UserKeypair::Table)
                    .col(UserKeypair::KeyId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserKeypair::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum UserKeypair {
    Table,
    UserId,
    PublicKey,
    PrivateKey,
    KeyId,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
