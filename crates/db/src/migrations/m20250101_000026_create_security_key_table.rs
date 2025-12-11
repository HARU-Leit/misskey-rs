//! Create security key table for WebAuthn/Passkey authentication.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create security_key table
        manager
            .create_table(
                Table::create()
                    .table(SecurityKey::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SecurityKey::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SecurityKey::UserId).string().not_null())
                    .col(ColumnDef::new(SecurityKey::Name).string().not_null())
                    .col(
                        ColumnDef::new(SecurityKey::CredentialId)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(SecurityKey::PublicKey).text().not_null())
                    .col(
                        ColumnDef::new(SecurityKey::Counter)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(SecurityKey::CredentialType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecurityKey::Transports)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SecurityKey::Aaguid).string().null())
                    .col(
                        ColumnDef::new(SecurityKey::IsPasskey)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(SecurityKey::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(SecurityKey::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_security_key_user")
                            .from(SecurityKey::Table, SecurityKey::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for efficient lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_security_key_user_id")
                    .table(SecurityKey::Table)
                    .col(SecurityKey::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on credential_id for authentication
        manager
            .create_index(
                Index::create()
                    .name("idx_security_key_credential_id")
                    .table(SecurityKey::Table)
                    .col(SecurityKey::CredentialId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SecurityKey::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum SecurityKey {
    Table,
    Id,
    UserId,
    Name,
    CredentialId,
    PublicKey,
    Counter,
    CredentialType,
    Transports,
    Aaguid,
    IsPasskey,
    LastUsedAt,
    CreatedAt,
}

#[derive(Iden)]
pub enum User {
    Table,
    Id,
}
