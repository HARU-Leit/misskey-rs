//! Create antenna and antenna_note tables.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create antenna table
        manager
            .create_table(
                Table::create()
                    .table(Antenna::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Antenna::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Antenna::UserId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Antenna::Name)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Antenna::Src)
                            .string_len(16)
                            .not_null()
                            .default("all"),
                    )
                    .col(ColumnDef::new(Antenna::UserListId).string_len(32))
                    .col(
                        ColumnDef::new(Antenna::Keywords)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Antenna::ExcludeKeywords)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Antenna::Users)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Antenna::Instances)
                            .json_binary()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(Antenna::CaseSensitive)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Antenna::WithReplies)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Antenna::WithFile)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Antenna::Notify)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Antenna::LocalOnly)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Antenna::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Antenna::DisplayOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Antenna::NotesCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Antenna::LastUsedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Antenna::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Antenna::UpdatedAt).timestamp_with_time_zone())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_antenna_user")
                            .from(Antenna::Table, Antenna::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_antenna_user_list")
                            .from(Antenna::Table, Antenna::UserListId)
                            .to(UserList::Table, UserList::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_antenna_user_id")
                    .table(Antenna::Table)
                    .col(Antenna::UserId)
                    .to_owned(),
            )
            .await?;

        // Index on is_active for matching queries
        manager
            .create_index(
                Index::create()
                    .name("idx_antenna_is_active")
                    .table(Antenna::Table)
                    .col(Antenna::IsActive)
                    .to_owned(),
            )
            .await?;

        // Create antenna_note table
        manager
            .create_table(
                Table::create()
                    .table(AntennaNotes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AntennaNotes::Id)
                            .string_len(32)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AntennaNotes::AntennaId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AntennaNotes::NoteId)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AntennaNotes::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(AntennaNotes::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_antenna_note_antenna")
                            .from(AntennaNotes::Table, AntennaNotes::AntennaId)
                            .to(Antenna::Table, Antenna::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_antenna_note_note")
                            .from(AntennaNotes::Table, AntennaNotes::NoteId)
                            .to(Note::Table, Note::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index on antenna_id for fetching notes
        manager
            .create_index(
                Index::create()
                    .name("idx_antenna_note_antenna_id")
                    .table(AntennaNotes::Table)
                    .col(AntennaNotes::AntennaId)
                    .to_owned(),
            )
            .await?;

        // Index on note_id for checking duplicates
        manager
            .create_index(
                Index::create()
                    .name("idx_antenna_note_note_id")
                    .table(AntennaNotes::Table)
                    .col(AntennaNotes::NoteId)
                    .to_owned(),
            )
            .await?;

        // Unique constraint on (antenna_id, note_id)
        manager
            .create_index(
                Index::create()
                    .name("idx_antenna_note_unique")
                    .table(AntennaNotes::Table)
                    .col(AntennaNotes::AntennaId)
                    .col(AntennaNotes::NoteId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index on created_at for pagination
        manager
            .create_index(
                Index::create()
                    .name("idx_antenna_note_created_at")
                    .table(AntennaNotes::Table)
                    .col(AntennaNotes::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AntennaNotes::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Antenna::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Antenna {
    Table,
    Id,
    UserId,
    Name,
    Src,
    UserListId,
    Keywords,
    ExcludeKeywords,
    Users,
    Instances,
    CaseSensitive,
    WithReplies,
    WithFile,
    Notify,
    LocalOnly,
    IsActive,
    DisplayOrder,
    NotesCount,
    LastUsedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum AntennaNotes {
    Table,
    Id,
    AntennaId,
    NoteId,
    IsRead,
    CreatedAt,
}

#[derive(Iden)]
enum User {
    Table,
    Id,
}

#[derive(Iden)]
enum UserList {
    Table,
    Id,
}

#[derive(Iden)]
enum Note {
    Table,
    Id,
}
