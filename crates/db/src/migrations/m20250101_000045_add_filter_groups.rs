//! Add filter groups feature for organizing word filters into presets.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create filter_group table
        manager
            .create_table(
                Table::create()
                    .table(FilterGroup::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FilterGroup::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FilterGroup::UserId).string().not_null())
                    .col(ColumnDef::new(FilterGroup::Name).string().not_null())
                    .col(ColumnDef::new(FilterGroup::Description).text().null())
                    .col(
                        ColumnDef::new(FilterGroup::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(FilterGroup::DisplayOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(FilterGroup::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FilterGroup::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(FilterGroup::Table, FilterGroup::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for filter_group user lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_filter_group_user_id")
                    .table(FilterGroup::Table)
                    .col(FilterGroup::UserId)
                    .to_owned(),
            )
            .await?;

        // Add group_id column to word_filter table
        manager
            .alter_table(
                Table::alter()
                    .table(WordFilter::Table)
                    .add_column(ColumnDef::new(WordFilter::GroupId).string().null())
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_word_filter_group_id")
                            .from_tbl(WordFilter::Table)
                            .from_col(WordFilter::GroupId)
                            .to_tbl(FilterGroup::Table)
                            .to_col(FilterGroup::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for word_filter group lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_word_filter_group_id")
                    .table(WordFilter::Table)
                    .col(WordFilter::GroupId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_word_filter_group_id")
                    .table(WordFilter::Table)
                    .to_owned(),
            )
            .await?;

        // Drop foreign key and column from word_filter
        manager
            .alter_table(
                Table::alter()
                    .table(WordFilter::Table)
                    .drop_foreign_key(Alias::new("fk_word_filter_group_id"))
                    .drop_column(WordFilter::GroupId)
                    .to_owned(),
            )
            .await?;

        // Drop filter_group table
        manager
            .drop_table(Table::drop().table(FilterGroup::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum FilterGroup {
    Table,
    Id,
    UserId,
    Name,
    Description,
    IsActive,
    DisplayOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum WordFilter {
    Table,
    GroupId,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}
