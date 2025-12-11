//! Add authorized fetch settings to `user_profile` and instance tables.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add secure_fetch_only to user_profile
        // When true, requires HTTP signature verification for requests to this user's resources
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .add_column(
                        ColumnDef::new(UserProfile::SecureFetchOnly)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Add require_authorized_fetch to instance table
        // When true, requires HTTP signature for all incoming activities from this instance
        manager
            .alter_table(
                Table::alter()
                    .table(Instance::Table)
                    .add_column(
                        ColumnDef::new(Instance::RequireAuthorizedFetch)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Instance::Table)
                    .drop_column(Instance::RequireAuthorizedFetch)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .drop_column(UserProfile::SecureFetchOnly)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum UserProfile {
    Table,
    SecureFetchOnly,
}

#[derive(Iden)]
enum Instance {
    Table,
    RequireAuthorizedFetch,
}
