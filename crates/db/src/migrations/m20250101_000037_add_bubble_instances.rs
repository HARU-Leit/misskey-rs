//! Migration to add bubble_instances column to meta_settings table.
//!
//! This adds support for the "bubble timeline" feature, which shows posts
//! from local users and whitelisted remote instances.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add bubble_instances column (JSON array of hostnames)
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .add_column(
                        ColumnDef::new(MetaSettings::BubbleInstances)
                            .json_binary()
                            .null()
                            .default(Value::String(Some(Box::new("[]".to_string())))),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .drop_column(MetaSettings::BubbleInstances)
                    .to_owned(),
            )
            .await
    }
}

/// Meta settings table for the migration.
#[derive(Iden)]
enum MetaSettings {
    Table,
    BubbleInstances,
}
