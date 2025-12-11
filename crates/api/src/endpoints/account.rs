//! Account management endpoints (migration, deletion, export, import).

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::{
    AccountService, CreateExportInput, CreateImportInput, DeleteAccountInput, DeletionRecord,
    DeletionStatusResponse, ExportDataType, ExportJob, ExportedFollow, ExportedNote,
    ExportedProfile, ImportJob, MigrateAccountInput, MigrationRecord, MigrationStatusResponse,
};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

// =====================
// Account Migration
// =====================

/// Request to set account aliases.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAliasesRequest {
    /// List of also-known-as URIs
    pub aliases: Vec<String>,
}

/// Initiate account migration.
async fn migrate(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<MigrateAccountInput>,
) -> AppResult<ApiResponse<MigrationRecord>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let migration = account_service.migrate_account(&user.id, input).await?;
    Ok(ApiResponse::ok(migration))
}

/// Set account aliases (alsoKnownAs).
async fn set_aliases(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<SetAliasesRequest>,
) -> AppResult<ApiResponse<Vec<String>>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    account_service.set_aliases(&user.id, req.aliases).await?;
    let aliases = account_service.get_aliases(&user.id).await?;
    Ok(ApiResponse::ok(aliases))
}

/// Get migration status.
async fn migration_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<MigrationStatusResponse>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let aliases = account_service.get_aliases(&user.id).await?;

    let response = MigrationStatusResponse {
        has_pending_migration: false, // TODO: Check for pending migrations
        migration: None,
        aliases,
        moved_to: None, // TODO: Get from user profile
    };

    Ok(ApiResponse::ok(response))
}

/// Request to cancel migration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelMigrationRequest {
    /// Migration ID to cancel
    pub migration_id: String,
}

/// Cancel a pending migration.
async fn cancel_migration(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CancelMigrationRequest>,
) -> AppResult<ApiResponse<()>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    account_service
        .cancel_migration(&user.id, &req.migration_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

// =====================
// Account Deletion
// =====================

/// Schedule account for deletion.
async fn schedule_deletion(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<DeleteAccountInput>,
) -> AppResult<ApiResponse<DeletionRecord>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let deletion = account_service.schedule_deletion(&user.id, input).await?;
    Ok(ApiResponse::ok(deletion))
}

/// Get deletion status.
async fn deletion_status(
    AuthUser(user): AuthUser,
    State(_state): State<AppState>,
) -> AppResult<ApiResponse<DeletionStatusResponse>> {
    // TODO: Fetch from database
    let response = DeletionStatusResponse {
        is_scheduled: false,
        deletion: None,
    };

    Ok(ApiResponse::ok(response))
}

/// Cancel scheduled deletion.
async fn cancel_deletion(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<()>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    account_service.cancel_deletion(&user.id).await?;
    Ok(ApiResponse::ok(()))
}

// =====================
// Account Export
// =====================

/// Create an export job.
async fn create_export(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateExportInput>,
) -> AppResult<ApiResponse<ExportJob>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let job = account_service.create_export(&user.id, input).await?;
    Ok(ApiResponse::ok(job))
}

/// Export profile data immediately.
async fn export_profile(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<ExportedProfile>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let profile = account_service.export_profile(&user.id).await?;
    Ok(ApiResponse::ok(profile))
}

/// Export following list immediately.
async fn export_following(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<ExportedFollow>>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let following = account_service.export_following(&user.id).await?;
    Ok(ApiResponse::ok(following))
}

/// Export followers list immediately.
async fn export_followers(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<ExportedFollow>>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let followers = account_service.export_followers(&user.id).await?;
    Ok(ApiResponse::ok(followers))
}

/// Request to export notes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportNotesRequest {
    /// Maximum number of notes to export (default: 10000)
    #[serde(default = "default_export_limit")]
    pub limit: u32,
    /// Export format (json or csv)
    #[serde(default)]
    pub format: Option<String>,
}

const fn default_export_limit() -> u32 {
    10000
}

/// Response for note export (either JSON array or CSV string).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ExportNotesResponse {
    /// JSON format response
    Json(Vec<ExportedNote>),
    /// CSV format response
    Csv { csv: String, count: usize },
}

/// Export user's notes immediately (JSON or CSV).
async fn export_notes(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ExportNotesRequest>,
) -> AppResult<ApiResponse<ExportNotesResponse>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let notes = account_service.export_notes(&user.id, req.limit).await?;

    let response = match req.format.as_deref() {
        Some("csv") => {
            let count = notes.len();
            let csv = AccountService::export_notes_as_csv(&notes);
            ExportNotesResponse::Csv { csv, count }
        }
        _ => ExportNotesResponse::Json(notes),
    };

    Ok(ApiResponse::ok(response))
}

/// Export blocking list immediately.
async fn export_blocking(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<ExportedFollow>>> {
    // Get all blocked users (up to 10000)
    let blockings = state
        .blocking_service
        .get_blocking(&user.id, 10000, None)
        .await?;

    let mut result = Vec::new();
    for blocking in blockings {
        if let Ok(blockee) = state.user_service.get(&blocking.blockee_id).await {
            let acct = if let Some(host) = &blockee.host {
                format!("{}@{}", blockee.username, host)
            } else {
                blockee.username.clone()
            };

            result.push(ExportedFollow {
                acct,
                uri: blockee.uri,
            });
        }
    }

    Ok(ApiResponse::ok(result))
}

/// Export muting list immediately.
async fn export_muting(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<ExportedFollow>>> {
    // Get all muted users (up to 10000)
    let mutings = state
        .muting_service
        .get_muting(&user.id, 10000, None)
        .await?;

    let mut result = Vec::new();
    for muting in mutings {
        if let Ok(mutee) = state.user_service.get(&muting.mutee_id).await {
            let acct = if let Some(host) = &mutee.host {
                format!("{}@{}", mutee.username, host)
            } else {
                mutee.username.clone()
            };

            result.push(ExportedFollow {
                acct,
                uri: mutee.uri,
            });
        }
    }

    Ok(ApiResponse::ok(result))
}

/// Request to get export job status.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExportStatusRequest {
    /// Job ID
    pub job_id: String,
}

/// Get export job status.
async fn export_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetExportStatusRequest>,
) -> AppResult<ApiResponse<ExportJob>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let job = account_service
        .get_export_status(&user.id, &req.job_id)
        .await?;
    Ok(ApiResponse::ok(job))
}

// =====================
// Account Import
// =====================

/// Create an import job.
async fn create_import(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateImportInput>,
) -> AppResult<ApiResponse<ImportJob>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let job = account_service.create_import(&user.id, input).await?;
    Ok(ApiResponse::ok(job))
}

/// Request to import following list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFollowingRequest {
    /// CSV data (one acct per line)
    pub data: String,
}

/// Import following list from CSV.
async fn import_following(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ImportFollowingRequest>,
) -> AppResult<ApiResponse<ImportJob>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let job = account_service
        .import_following(&user.id, &req.data)
        .await?;
    Ok(ApiResponse::ok(job))
}

/// Request to import blocking/muting list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportListRequest {
    /// CSV data (Mastodon format or simple list)
    pub data: String,
}

/// Import result summary.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// Total items processed
    pub total: u32,
    /// Successfully imported
    pub imported: u32,
    /// Skipped (already exists)
    pub skipped: u32,
    /// Failed to import
    pub failed: u32,
    /// Error details
    pub errors: Vec<ImportErrorDetail>,
}

/// Import error detail.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportErrorDetail {
    /// Line number or index
    pub index: u32,
    /// Account identifier
    pub account: String,
    /// Error message
    pub error: String,
}

/// Import blocking list from CSV (Mastodon format supported).
async fn import_blocking(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ImportListRequest>,
) -> AppResult<ApiResponse<ImportResult>> {
    let accounts = AccountService::parse_mastodon_csv(&req.data);

    let mut result = ImportResult {
        total: accounts.len() as u32,
        imported: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for (index, acct) in accounts.iter().enumerate() {
        let (username, host) = AccountService::parse_acct(acct);
        let host_ref = host.as_deref();

        // Find user
        if let Ok(target) = state
            .user_service
            .get_by_username(&username, host_ref)
            .await
        {
            // Try to block
            match state.blocking_service.block(&user.id, &target.id).await {
                Ok(_) => result.imported += 1,
                Err(misskey_common::AppError::Conflict(_)) => result.skipped += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(ImportErrorDetail {
                        index: index as u32,
                        account: acct.clone(),
                        error: e.to_string(),
                    });
                }
            }
        } else {
            result.failed += 1;
            result.errors.push(ImportErrorDetail {
                index: index as u32,
                account: acct.clone(),
                error: "User not found".to_string(),
            });
        }
    }

    Ok(ApiResponse::ok(result))
}

/// Import muting list from CSV (Mastodon format supported).
async fn import_muting(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ImportListRequest>,
) -> AppResult<ApiResponse<ImportResult>> {
    let accounts = AccountService::parse_mastodon_csv(&req.data);

    let mut result = ImportResult {
        total: accounts.len() as u32,
        imported: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    for (index, acct) in accounts.iter().enumerate() {
        let (username, host) = AccountService::parse_acct(acct);
        let host_ref = host.as_deref();

        // Find user
        if let Ok(target) = state
            .user_service
            .get_by_username(&username, host_ref)
            .await
        {
            // Try to mute (permanent)
            match state.muting_service.mute(&user.id, &target.id, None).await {
                Ok(_) => result.imported += 1,
                Err(misskey_common::AppError::Conflict(_)) => result.skipped += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(ImportErrorDetail {
                        index: index as u32,
                        account: acct.clone(),
                        error: e.to_string(),
                    });
                }
            }
        } else {
            result.failed += 1;
            result.errors.push(ImportErrorDetail {
                index: index as u32,
                account: acct.clone(),
                error: "User not found".to_string(),
            });
        }
    }

    Ok(ApiResponse::ok(result))
}

/// Request to get import job status.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetImportStatusRequest {
    /// Job ID
    pub job_id: String,
}

/// Get import job status.
async fn import_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetImportStatusRequest>,
) -> AppResult<ApiResponse<ImportJob>> {
    let account_service = state.account_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Account service not configured".to_string())
    })?;

    let job = account_service
        .get_import_status(&user.id, &req.job_id)
        .await?;
    Ok(ApiResponse::ok(job))
}

// =====================
// Available Data Types
// =====================

/// Response for available export data types.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableDataTypesResponse {
    /// Available data types for export/import
    pub data_types: Vec<DataTypeInfo>,
}

/// Info about a data type.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataTypeInfo {
    /// Data type ID
    pub id: ExportDataType,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Can be exported
    pub exportable: bool,
    /// Can be imported
    pub importable: bool,
}

/// Get available data types for export/import.
async fn available_data_types(
    AuthUser(_user): AuthUser,
) -> AppResult<ApiResponse<AvailableDataTypesResponse>> {
    let data_types = vec![
        DataTypeInfo {
            id: ExportDataType::Profile,
            name: "Profile".to_string(),
            description: "User profile information".to_string(),
            exportable: true,
            importable: false,
        },
        DataTypeInfo {
            id: ExportDataType::Notes,
            name: "Notes".to_string(),
            description: "Your posts/notes".to_string(),
            exportable: true,
            importable: false,
        },
        DataTypeInfo {
            id: ExportDataType::Following,
            name: "Following".to_string(),
            description: "Accounts you follow".to_string(),
            exportable: true,
            importable: true,
        },
        DataTypeInfo {
            id: ExportDataType::Followers,
            name: "Followers".to_string(),
            description: "Accounts following you".to_string(),
            exportable: true,
            importable: false,
        },
        DataTypeInfo {
            id: ExportDataType::Muting,
            name: "Muting".to_string(),
            description: "Muted accounts".to_string(),
            exportable: true,
            importable: true,
        },
        DataTypeInfo {
            id: ExportDataType::Blocking,
            name: "Blocking".to_string(),
            description: "Blocked accounts".to_string(),
            exportable: true,
            importable: true,
        },
        DataTypeInfo {
            id: ExportDataType::DriveFiles,
            name: "Drive Files".to_string(),
            description: "Files in your drive".to_string(),
            exportable: true,
            importable: false,
        },
        DataTypeInfo {
            id: ExportDataType::Favorites,
            name: "Favorites".to_string(),
            description: "Bookmarked notes".to_string(),
            exportable: true,
            importable: false,
        },
        DataTypeInfo {
            id: ExportDataType::UserLists,
            name: "User Lists".to_string(),
            description: "Custom user lists".to_string(),
            exportable: true,
            importable: true,
        },
        DataTypeInfo {
            id: ExportDataType::Antennas,
            name: "Antennas".to_string(),
            description: "Custom antennas".to_string(),
            exportable: true,
            importable: true,
        },
        DataTypeInfo {
            id: ExportDataType::Clips,
            name: "Clips".to_string(),
            description: "Note collections".to_string(),
            exportable: true,
            importable: false,
        },
    ];

    Ok(ApiResponse::ok(AvailableDataTypesResponse { data_types }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Migration
        .route("/migrate", post(migrate))
        .route("/aliases", post(set_aliases))
        .route("/migration/status", post(migration_status))
        .route("/migration/cancel", post(cancel_migration))
        // Deletion
        .route("/delete", post(schedule_deletion))
        .route("/deletion/status", post(deletion_status))
        .route("/deletion/cancel", post(cancel_deletion))
        // Export
        .route("/export", post(create_export))
        .route("/export/profile", post(export_profile))
        .route("/export/following", post(export_following))
        .route("/export/followers", post(export_followers))
        .route("/export/notes", post(export_notes))
        .route("/export/blocking", post(export_blocking))
        .route("/export/muting", post(export_muting))
        .route("/export/status", post(export_status))
        // Import
        .route("/import", post(create_import))
        .route("/import/following", post(import_following))
        .route("/import/blocking", post(import_blocking))
        .route("/import/muting", post(import_muting))
        .route("/import/status", post(import_status))
        // Info
        .route("/data-types", post(available_data_types))
}
