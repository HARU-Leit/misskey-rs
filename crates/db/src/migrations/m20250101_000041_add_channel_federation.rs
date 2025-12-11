//! Add federation fields to channel table for `ActivityPub` Group actor support.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add URI field for ActivityPub identity
        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .add_column(
                        ColumnDef::new(Channel::Uri)
                            .string_len(512)
                            .unique_key()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add public key PEM for HTTP signature verification
        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .add_column(ColumnDef::new(Channel::PublicKeyPem).text().null())
                    .to_owned(),
            )
            .await?;

        // Add private key PEM for signing outgoing activities
        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .add_column(ColumnDef::new(Channel::PrivateKeyPem).text().null())
                    .to_owned(),
            )
            .await?;

        // Add inbox URL for receiving activities
        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .add_column(ColumnDef::new(Channel::Inbox).string_len(512).null())
                    .to_owned(),
            )
            .await?;

        // Add shared inbox URL for efficient delivery
        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .add_column(ColumnDef::new(Channel::SharedInbox).string_len(512).null())
                    .to_owned(),
            )
            .await?;

        // Add host field for remote channels (null for local)
        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .add_column(ColumnDef::new(Channel::Host).string_len(128).null())
                    .to_owned(),
            )
            .await?;

        // Index on URI for lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_uri")
                    .table(Channel::Table)
                    .col(Channel::Uri)
                    .to_owned(),
            )
            .await?;

        // Index on host for filtering local/remote channels
        manager
            .create_index(
                Index::create()
                    .name("idx_channel_host")
                    .table(Channel::Table)
                    .col(Channel::Host)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_channel_host")
                    .table(Channel::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_channel_uri")
                    .table(Channel::Table)
                    .to_owned(),
            )
            .await?;

        // Drop columns
        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .drop_column(Channel::Host)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .drop_column(Channel::SharedInbox)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .drop_column(Channel::Inbox)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .drop_column(Channel::PrivateKeyPem)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .drop_column(Channel::PublicKeyPem)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channel::Table)
                    .drop_column(Channel::Uri)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Channel {
    Table,
    Uri,
    PublicKeyPem,
    PrivateKeyPem,
    Inbox,
    SharedInbox,
    Host,
}
