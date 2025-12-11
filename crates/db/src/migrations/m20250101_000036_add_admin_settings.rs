//! Add admin settings:
//! - `max_remote_note_text_length` for separate limits on remote notes
//! - `require_registration_approval` for manual account approval
//! - `force_nsfw_media` to auto-mark all uploads as NSFW

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add max_remote_note_text_length column
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .add_column(
                        ColumnDef::new(MetaSettings::MaxRemoteNoteTextLength)
                            .integer()
                            .not_null()
                            .default(10000),
                    )
                    .to_owned(),
            )
            .await?;

        // Add require_registration_approval column
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .add_column(
                        ColumnDef::new(MetaSettings::RequireRegistrationApproval)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Add force_nsfw_media column
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .add_column(
                        ColumnDef::new(MetaSettings::ForceNsfwMedia)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // Create registration_approval table for pending approvals
        manager
            .create_table(
                Table::create()
                    .table(RegistrationApproval::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RegistrationApproval::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RegistrationApproval::UserId)
                            .string_len(32)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(RegistrationApproval::Reason).text().null())
                    .col(
                        ColumnDef::new(RegistrationApproval::Status)
                            .string_len(16)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(RegistrationApproval::ReviewedBy)
                            .string_len(32)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RegistrationApproval::ReviewNote)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(RegistrationApproval::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RegistrationApproval::ReviewedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on status for filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_registration_approval_status")
                    .table(RegistrationApproval::Table)
                    .col(RegistrationApproval::Status)
                    .to_owned(),
            )
            .await?;

        // Add foreign key to user table
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_registration_approval_user")
                    .from(RegistrationApproval::Table, RegistrationApproval::UserId)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop registration_approval table
        manager
            .drop_table(Table::drop().table(RegistrationApproval::Table).to_owned())
            .await?;

        // Remove force_nsfw_media column
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .drop_column(MetaSettings::ForceNsfwMedia)
                    .to_owned(),
            )
            .await?;

        // Remove require_registration_approval column
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .drop_column(MetaSettings::RequireRegistrationApproval)
                    .to_owned(),
            )
            .await?;

        // Remove max_remote_note_text_length column
        manager
            .alter_table(
                Table::alter()
                    .table(MetaSettings::Table)
                    .drop_column(MetaSettings::MaxRemoteNoteTextLength)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum MetaSettings {
    Table,
    MaxRemoteNoteTextLength,
    RequireRegistrationApproval,
    ForceNsfwMedia,
}

#[derive(Iden)]
enum RegistrationApproval {
    Table,
    Id,
    UserId,
    Reason,
    Status,
    ReviewedBy,
    ReviewNote,
    CreatedAt,
    ReviewedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}
