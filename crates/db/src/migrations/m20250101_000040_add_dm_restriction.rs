//! Migration to add DM restriction setting to user_profile.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add receive_dm_from_followers_only field to user_profile
        // When true, only followers can send DMs to this user
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .add_column(
                        ColumnDef::new(UserProfile::ReceiveDmFromFollowersOnly)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .drop_column(UserProfile::ReceiveDmFromFollowersOnly)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum UserProfile {
    Table,
    ReceiveDmFromFollowersOnly,
}
