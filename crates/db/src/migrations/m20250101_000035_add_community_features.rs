//! Add community-requested features:
//! - pronouns field to user_profile
//! - meta_settings table for instance configuration
//! - Extensions for drive cleanup

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add pronouns field to user_profile
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .add_column(ColumnDef::new(UserProfile::Pronouns).string_len(128).null())
                    .to_owned(),
            )
            .await?;

        // Create meta_settings table for instance configuration
        manager
            .create_table(
                Table::create()
                    .table(MetaSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MetaSettings::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    // Instance info
                    .col(ColumnDef::new(MetaSettings::Name).string_len(256).null())
                    .col(ColumnDef::new(MetaSettings::ShortName).string_len(64).null())
                    .col(ColumnDef::new(MetaSettings::Description).text().null())
                    .col(ColumnDef::new(MetaSettings::MaintainerName).string_len(256).null())
                    .col(ColumnDef::new(MetaSettings::MaintainerEmail).string_len(256).null())
                    .col(ColumnDef::new(MetaSettings::Langs).json_binary().not_null())
                    .col(ColumnDef::new(MetaSettings::IconUrl).string_len(512).null())
                    .col(ColumnDef::new(MetaSettings::BannerUrl).string_len(512).null())
                    .col(ColumnDef::new(MetaSettings::ThemeColor).string_len(32).null())
                    // Registration settings
                    .col(
                        ColumnDef::new(MetaSettings::DisableRegistration)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(MetaSettings::EmailRequiredForSignup)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    // UI defaults
                    .col(
                        ColumnDef::new(MetaSettings::DefaultBlurNsfw)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(MetaSettings::DefaultHideAds)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    // Content limits
                    .col(
                        ColumnDef::new(MetaSettings::MaxNoteTextLength)
                            .integer()
                            .not_null()
                            .default(3000),
                    )
                    .col(
                        ColumnDef::new(MetaSettings::MaxPageContentLength)
                            .integer()
                            .not_null()
                            .default(65536),
                    )
                    .col(
                        ColumnDef::new(MetaSettings::MaxPagesPerUser)
                            .integer()
                            .not_null()
                            .default(100),
                    )
                    // Drive limits
                    .col(
                        ColumnDef::new(MetaSettings::DefaultDriveCapacityMb)
                            .integer()
                            .not_null()
                            .default(1024),
                    )
                    .col(
                        ColumnDef::new(MetaSettings::MaxFileSizeMb)
                            .integer()
                            .not_null()
                            .default(256),
                    )
                    // Timestamps
                    .col(
                        ColumnDef::new(MetaSettings::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MetaSettings::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop meta_settings table
        manager
            .drop_table(Table::drop().table(MetaSettings::Table).to_owned())
            .await?;

        // Remove pronouns column from user_profile
        manager
            .alter_table(
                Table::alter()
                    .table(UserProfile::Table)
                    .drop_column(UserProfile::Pronouns)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum UserProfile {
    Table,
    Pronouns,
}

#[derive(Iden)]
enum MetaSettings {
    Table,
    Id,
    Name,
    ShortName,
    Description,
    MaintainerName,
    MaintainerEmail,
    Langs,
    IconUrl,
    BannerUrl,
    ThemeColor,
    DisableRegistration,
    EmailRequiredForSignup,
    DefaultBlurNsfw,
    DefaultHideAds,
    MaxNoteTextLength,
    MaxPageContentLength,
    MaxPagesPerUser,
    DefaultDriveCapacityMb,
    MaxFileSizeMb,
    CreatedAt,
    UpdatedAt,
}
