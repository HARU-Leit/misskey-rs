//! Drive endpoints for file management.

use axum::{
    Json, Router,
    extract::{Multipart, State},
    routing::post,
};
use misskey_common::AppResult;
use misskey_core::{CreateFileInput, CreateFolderInput};
use misskey_db::entities::{
    drive_file::Model as DriveFileModel, drive_folder::Model as DriveFolderModel,
};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Drive file response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFileResponse {
    pub id: String,
    pub created_at: String,
    pub name: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub size: i64,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub is_sensitive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
}

impl From<DriveFileModel> for DriveFileResponse {
    fn from(f: DriveFileModel) -> Self {
        Self {
            id: f.id,
            created_at: f.created_at.to_rfc3339(),
            name: f.name,
            content_type: f.content_type,
            size: f.size,
            url: f.url,
            thumbnail_url: f.thumbnail_url,
            blurhash: f.blurhash,
            folder_id: f.folder_id,
            comment: f.comment,
            is_sensitive: f.is_sensitive,
            width: f.width,
            height: f.height,
        }
    }
}

/// Upload a file via multipart form.
async fn upload_file(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<ApiResponse<DriveFileResponse>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut folder_id: Option<String> = None;
    let mut comment: Option<String> = None;
    let mut is_sensitive = false;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| misskey_common::AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(std::string::ToString::to_string);
                content_type = field.content_type().map(std::string::ToString::to_string);
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| misskey_common::AppError::BadRequest(e.to_string()))?
                        .to_vec(),
                );
            }
            "name" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| misskey_common::AppError::BadRequest(e.to_string()))?;
                if !text.is_empty() {
                    file_name = Some(text);
                }
            }
            "folderId" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| misskey_common::AppError::BadRequest(e.to_string()))?;
                if !text.is_empty() && text != "null" {
                    folder_id = Some(text);
                }
            }
            "comment" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| misskey_common::AppError::BadRequest(e.to_string()))?;
                if !text.is_empty() {
                    comment = Some(text);
                }
            }
            "isSensitive" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| misskey_common::AppError::BadRequest(e.to_string()))?;
                is_sensitive = text == "true" || text == "1";
            }
            _ => {}
        }
    }

    let data = file_data
        .ok_or_else(|| misskey_common::AppError::BadRequest("No file provided".to_string()))?;

    let name = file_name.unwrap_or_else(|| "unnamed".to_string());
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());
    let size = data.len() as i64;

    let input = CreateFileInput {
        name,
        content_type,
        size,
        data,
        folder_id,
        comment,
        is_sensitive,
    };

    let file = state.drive_service.upload_file(&user.id, input).await?;
    Ok(ApiResponse::ok(file.into()))
}

/// List files request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFilesRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
    pub folder_id: Option<String>,
}

const fn default_limit() -> u64 {
    10
}

/// List user's files.
async fn list_files(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListFilesRequest>,
) -> AppResult<ApiResponse<Vec<DriveFileResponse>>> {
    let limit = req.limit.min(100);
    let files = state
        .drive_service
        .get_user_files(
            &user.id,
            limit,
            req.until_id.as_deref(),
            req.folder_id.as_deref(),
        )
        .await?;
    Ok(ApiResponse::ok(files.into_iter().map(Into::into).collect()))
}

/// Show file request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowFileRequest {
    pub file_id: String,
}

/// Get file details.
async fn show_file(
    State(state): State<AppState>,
    Json(req): Json<ShowFileRequest>,
) -> AppResult<ApiResponse<DriveFileResponse>> {
    let file = state.drive_service.get_file(&req.file_id).await?;
    Ok(ApiResponse::ok(file.into()))
}

/// Update file request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFileRequest {
    pub file_id: String,
    pub name: Option<String>,
    pub folder_id: Option<Option<String>>,
    pub is_sensitive: Option<bool>,
    pub comment: Option<Option<String>>,
}

/// Update file properties.
async fn update_file(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateFileRequest>,
) -> AppResult<ApiResponse<DriveFileResponse>> {
    let file = state
        .drive_service
        .update_file(
            &user.id,
            &req.file_id,
            req.name,
            req.folder_id,
            req.is_sensitive,
            req.comment,
        )
        .await?;
    Ok(ApiResponse::ok(file.into()))
}

/// Delete file request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFileRequest {
    pub file_id: String,
}

/// Delete a file.
async fn delete_file(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteFileRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .drive_service
        .delete_file(&user.id, &req.file_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Storage usage response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsageResponse {
    pub used: i64,
    pub limit: i64,
}

/// Get storage usage.
async fn storage_usage(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<StorageUsageResponse>> {
    let usage = state.drive_service.get_storage_usage(&user.id).await?;
    Ok(ApiResponse::ok(StorageUsageResponse {
        used: usage.used,
        limit: usage.limit,
    }))
}

// =====================
// Folder types
// =====================

/// Drive folder response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFolderResponse {
    pub id: String,
    pub created_at: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

impl From<DriveFolderModel> for DriveFolderResponse {
    fn from(f: DriveFolderModel) -> Self {
        Self {
            id: f.id,
            created_at: f.created_at.to_rfc3339(),
            name: f.name,
            parent_id: f.parent_id,
        }
    }
}

/// Create folder request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<String>,
}

/// Create a new folder.
async fn create_folder(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateFolderRequest>,
) -> AppResult<ApiResponse<DriveFolderResponse>> {
    let input = CreateFolderInput {
        name: req.name,
        parent_id: req.parent_id,
    };
    let folder = state.drive_service.create_folder(&user.id, input).await?;
    Ok(ApiResponse::ok(folder.into()))
}

/// List folders request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFoldersRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub parent_id: Option<String>,
}

/// List user's folders.
async fn list_folders(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListFoldersRequest>,
) -> AppResult<ApiResponse<Vec<DriveFolderResponse>>> {
    let limit = req.limit.min(100);
    let folders = state
        .drive_service
        .get_user_folders(&user.id, req.parent_id.as_deref(), limit)
        .await?;
    Ok(ApiResponse::ok(
        folders.into_iter().map(Into::into).collect(),
    ))
}

/// Show folder request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowFolderRequest {
    pub folder_id: String,
}

/// Get folder details.
async fn show_folder(
    State(state): State<AppState>,
    Json(req): Json<ShowFolderRequest>,
) -> AppResult<ApiResponse<DriveFolderResponse>> {
    let folder = state.drive_service.get_folder(&req.folder_id).await?;
    Ok(ApiResponse::ok(folder.into()))
}

/// Update folder request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFolderRequest {
    pub folder_id: String,
    pub name: Option<String>,
    pub parent_id: Option<Option<String>>,
}

/// Update folder properties.
async fn update_folder(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateFolderRequest>,
) -> AppResult<ApiResponse<DriveFolderResponse>> {
    let folder = state
        .drive_service
        .update_folder(&user.id, &req.folder_id, req.name, req.parent_id)
        .await?;
    Ok(ApiResponse::ok(folder.into()))
}

/// Delete folder request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFolderRequest {
    pub folder_id: String,
}

/// Delete a folder.
async fn delete_folder(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteFolderRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .drive_service
        .delete_folder(&user.id, &req.folder_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

// =====================
// Cleanup (unattached files)
// =====================

/// Cleanup preview request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreviewRequest {
    #[serde(default = "default_cleanup_limit")]
    pub limit: u64,
}

const fn default_cleanup_limit() -> u64 {
    100
}

/// Cleanup preview response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreviewResponse {
    pub count: i64,
    pub files: Vec<DriveFileResponse>,
    pub total_size: i64,
}

/// Preview unattached files that would be deleted.
async fn cleanup_preview(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CleanupPreviewRequest>,
) -> AppResult<ApiResponse<CleanupPreviewResponse>> {
    let limit = req.limit.min(100);
    let count = state.drive_service.count_unattached_files(&user.id).await?;
    let files = state
        .drive_service
        .get_unattached_files(&user.id, limit)
        .await?;
    let total_size: i64 = files.iter().map(|f| f.size).sum();

    Ok(ApiResponse::ok(CleanupPreviewResponse {
        count,
        files: files.into_iter().map(Into::into).collect(),
        total_size,
    }))
}

/// Cleanup execute request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupExecuteRequest {
    #[serde(default = "default_cleanup_limit")]
    pub limit: u64,
}

/// Cleanup result response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResultResponse {
    pub deleted_count: u64,
    pub freed_bytes: i64,
    pub file_ids: Vec<String>,
}

/// Execute cleanup of unattached files.
async fn cleanup_execute(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CleanupExecuteRequest>,
) -> AppResult<ApiResponse<CleanupResultResponse>> {
    let limit = req.limit.min(100);
    let result = state
        .drive_service
        .cleanup_unattached_files(&user.id, limit)
        .await?;

    Ok(ApiResponse::ok(CleanupResultResponse {
        deleted_count: result.deleted_count,
        freed_bytes: result.freed_bytes,
        file_ids: result.file_ids,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // File routes
        .route("/files/create", post(upload_file))
        .route("/files", post(list_files))
        .route("/files/show", post(show_file))
        .route("/files/update", post(update_file))
        .route("/files/delete", post(delete_file))
        // Folder routes
        .route("/folders/create", post(create_folder))
        .route("/folders", post(list_folders))
        .route("/folders/show", post(show_folder))
        .route("/folders/update", post(update_folder))
        .route("/folders/delete", post(delete_folder))
        // Cleanup routes
        .route("/files/cleanup/preview", post(cleanup_preview))
        .route("/files/cleanup/execute", post(cleanup_execute))
        // Storage info
        .route("/", post(storage_usage))
}
