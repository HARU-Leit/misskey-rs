//! Add smart clip features (condition-based auto-add) and clip note operations.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add smart clip fields to clip table
        manager
            .alter_table(
                Table::alter()
                    .table(Clip::Table)
                    .add_column(
                        ColumnDef::new(Clip::IsSmartClip)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .add_column(
                        ColumnDef::new(Clip::SmartConditions)
                            .json()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Clip::SmartMaxNotes)
                            .integer()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Clip::SmartLastProcessedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for smart clips
        manager
            .create_index(
                Index::create()
                    .name("idx_clip_is_smart_clip")
                    .table(Clip::Table)
                    .col(Clip::IsSmartClip)
                    .col(Clip::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_clip_is_smart_clip")
                    .table(Clip::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Clip::Table)
                    .drop_column(Clip::IsSmartClip)
                    .drop_column(Clip::SmartConditions)
                    .drop_column(Clip::SmartMaxNotes)
                    .drop_column(Clip::SmartLastProcessedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Clip {
    Table,
    UserId,
    IsSmartClip,
    SmartConditions,
    SmartMaxNotes,
    SmartLastProcessedAt,
}
