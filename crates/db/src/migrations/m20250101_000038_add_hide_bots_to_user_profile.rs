//! Migration to add `hide_bots` setting to `user_profile`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add hide_bots field to user_profile
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .add_column(
                        ColumnDef::new(UserProfile::HideBots)
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
                    .table(UserProfile::Table)
                    .drop_column(UserProfile::HideBots)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum UserProfile {
    Table,
    HideBots,
}
