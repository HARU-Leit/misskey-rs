//! Create OAuth tables for application authentication.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create oauth_app table
        manager
            .create_table(
                Table::create()
                    .table(OAuthApp::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OAuthApp::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OAuthApp::ClientId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(OAuthApp::ClientSecret).string().not_null())
                    .col(ColumnDef::new(OAuthApp::Name).string().not_null())
                    .col(ColumnDef::new(OAuthApp::Description).text().null())
                    .col(ColumnDef::new(OAuthApp::IconUrl).string().null())
                    .col(ColumnDef::new(OAuthApp::WebsiteUrl).string().null())
                    .col(
                        ColumnDef::new(OAuthApp::RedirectUris)
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(OAuthApp::Scopes).json_binary().not_null())
                    .col(ColumnDef::new(OAuthApp::UserId).string().not_null())
                    .col(
                        ColumnDef::new(OAuthApp::IsTrusted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(OAuthApp::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(OAuthApp::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(OAuthApp::UpdatedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_app_user")
                            .from(OAuthApp::Table, OAuthApp::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on client_id
        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_app_client_id")
                    .table(OAuthApp::Table)
                    .col(OAuthApp::ClientId)
                    .to_owned(),
            )
            .await?;

        // Create index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_app_user_id")
                    .table(OAuthApp::Table)
                    .col(OAuthApp::UserId)
                    .to_owned(),
            )
            .await?;

        // Create oauth_token table
        manager
            .create_table(
                Table::create()
                    .table(OAuthToken::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OAuthToken::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(OAuthToken::TokenHash)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(OAuthToken::TokenType).string().not_null())
                    .col(ColumnDef::new(OAuthToken::AppId).string().not_null())
                    .col(ColumnDef::new(OAuthToken::UserId).string().not_null())
                    .col(ColumnDef::new(OAuthToken::Scopes).json_binary().not_null())
                    .col(ColumnDef::new(OAuthToken::CodeChallenge).string().null())
                    .col(
                        ColumnDef::new(OAuthToken::CodeChallengeMethod)
                            .string()
                            .null(),
                    )
                    .col(ColumnDef::new(OAuthToken::RedirectUri).string().null())
                    .col(
                        ColumnDef::new(OAuthToken::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OAuthToken::IsRevoked)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(OAuthToken::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(OAuthToken::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_token_app")
                            .from(OAuthToken::Table, OAuthToken::AppId)
                            .to(OAuthApp::Table, OAuthApp::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_token_user")
                            .from(OAuthToken::Table, OAuthToken::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on token_hash for lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_token_hash")
                    .table(OAuthToken::Table)
                    .col(OAuthToken::TokenHash)
                    .to_owned(),
            )
            .await?;

        // Create index on app_id
        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_token_app_id")
                    .table(OAuthToken::Table)
                    .col(OAuthToken::AppId)
                    .to_owned(),
            )
            .await?;

        // Create index on user_id
        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_token_user_id")
                    .table(OAuthToken::Table)
                    .col(OAuthToken::UserId)
                    .to_owned(),
            )
            .await?;

        // Create index on expires_at for cleanup
        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_token_expires_at")
                    .table(OAuthToken::Table)
                    .col(OAuthToken::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OAuthToken::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(OAuthApp::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum OAuthApp {
    Table,
    Id,
    ClientId,
    ClientSecret,
    Name,
    Description,
    IconUrl,
    WebsiteUrl,
    RedirectUris,
    Scopes,
    UserId,
    IsTrusted,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum OAuthToken {
    Table,
    Id,
    TokenHash,
    TokenType,
    AppId,
    UserId,
    Scopes,
    CodeChallenge,
    CodeChallengeMethod,
    RedirectUri,
    ExpiresAt,
    IsRevoked,
    CreatedAt,
    LastUsedAt,
}

#[derive(Iden)]
pub enum User {
    Table,
    Id,
}
