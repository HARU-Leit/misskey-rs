//! Drive service for file management.

use crate::services::storage::StorageService;
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{drive_file, drive_folder},
    repositories::{DriveFileRepository, DriveFolderRepository},
};
use sea_orm::Set;

/// Maximum file size (256MB)
pub const MAX_FILE_SIZE: i64 = 256 * 1024 * 1024;

/// Default storage limit per user (1GB)
pub const DEFAULT_STORAGE_LIMIT: i64 = 1024 * 1024 * 1024;

/// Drive service for file management.
#[derive(Clone)]
pub struct DriveService {
    file_repo: DriveFileRepository,
    folder_repo: DriveFolderRepository,
    storage: Option<StorageService>,
    id_gen: IdGenerator,
    base_url: String,
}

/// Input for creating a new file.
pub struct CreateFileInput {
    pub name: String,
    pub content_type: String,
    pub size: i64,
    pub data: Vec<u8>,
    pub folder_id: Option<String>,
    pub comment: Option<String>,
    pub is_sensitive: bool,
}

impl DriveService {
    /// Create a new drive service.
    #[must_use]
    pub const fn new(
        file_repo: DriveFileRepository,
        folder_repo: DriveFolderRepository,
        base_url: String,
    ) -> Self {
        Self {
            file_repo,
            folder_repo,
            storage: None,
            id_gen: IdGenerator::new(),
            base_url,
        }
    }

    /// Create a new drive service with storage backend.
    #[must_use]
    pub fn with_storage(
        file_repo: DriveFileRepository,
        folder_repo: DriveFolderRepository,
        storage: StorageService,
        base_url: String,
    ) -> Self {
        Self {
            file_repo,
            folder_repo,
            storage: Some(storage),
            id_gen: IdGenerator::new(),
            base_url,
        }
    }

    /// Set the storage backend.
    pub fn set_storage(&mut self, storage: StorageService) {
        self.storage = Some(storage);
    }

    /// Upload a new file.
    pub async fn upload_file(
        &self,
        user_id: &str,
        input: CreateFileInput,
    ) -> AppResult<drive_file::Model> {
        // Validate file size
        if input.size > MAX_FILE_SIZE {
            return Err(AppError::BadRequest(format!(
                "File too large. Maximum size is {MAX_FILE_SIZE} bytes"
            )));
        }

        if input.size <= 0 {
            return Err(AppError::BadRequest("File is empty".to_string()));
        }

        // Check storage quota
        let used = self.file_repo.get_storage_used(user_id).await?;
        if used + input.size > DEFAULT_STORAGE_LIMIT {
            return Err(AppError::BadRequest("Storage quota exceeded".to_string()));
        }

        // Validate folder if specified
        if let Some(ref folder_id) = input.folder_id {
            let folder = self.folder_repo.find_by_id(folder_id).await?;
            if let Some(f) = folder {
                if f.user_id != user_id {
                    return Err(AppError::Forbidden(
                        "Folder belongs to another user".to_string(),
                    ));
                }
            } else {
                return Err(AppError::NotFound("Folder not found".to_string()));
            }
        }

        // Calculate MD5 hash
        let md5 = format!("{:x}", md5::compute(&input.data));

        // Check for duplicate file by MD5
        if let Some(existing) = self.file_repo.find_by_md5(&md5).await? {
            // Return existing file if same user owns it
            if existing.user_id == user_id {
                return Ok(existing);
            }
        }

        // Generate file ID and storage key
        let file_id = self.id_gen.generate();
        let storage_key = generate_storage_key(&file_id, &input.name);

        // Save file to storage if backend is configured
        let url = if let Some(ref storage) = self.storage {
            storage.save(&storage_key, &input.data).await?;
            storage.get_url(&storage_key)
        } else {
            // Fallback URL if no storage backend is configured
            format!("{}/files/{}", self.base_url, storage_key)
        };

        // Get image dimensions if applicable
        let (width, height) = if input.content_type.starts_with("image/") {
            get_image_dimensions(&input.data)
        } else {
            (None, None)
        };

        let model = drive_file::ActiveModel {
            id: Set(file_id),
            user_id: Set(user_id.to_string()),
            user_host: Set(None),
            name: Set(input.name),
            content_type: Set(input.content_type),
            size: Set(input.size),
            url: Set(url),
            thumbnail_url: Set(None),
            webpublic_url: Set(None),
            blurhash: Set(None),
            width: Set(width),
            height: Set(height),
            comment: Set(input.comment),
            is_sensitive: Set(input.is_sensitive),
            is_link: Set(false),
            md5: Set(Some(md5)),
            storage_key: Set(Some(storage_key)),
            folder_id: Set(input.folder_id),
            uri: Set(None),
            created_at: Set(chrono::Utc::now().into()),
        };

        self.file_repo.create(model).await
    }

    /// Get a file by ID.
    pub async fn get_file(&self, id: &str) -> AppResult<drive_file::Model> {
        self.file_repo.get_by_id(id).await
    }

    /// Get files by IDs.
    pub async fn get_files(&self, ids: &[String]) -> AppResult<Vec<drive_file::Model>> {
        self.file_repo.find_by_ids(ids).await
    }

    /// Get files for a user.
    pub async fn get_user_files(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
        folder_id: Option<&str>,
    ) -> AppResult<Vec<drive_file::Model>> {
        self.file_repo
            .find_by_user(user_id, limit, until_id, folder_id)
            .await
    }

    /// Update file properties.
    pub async fn update_file(
        &self,
        user_id: &str,
        file_id: &str,
        name: Option<String>,
        folder_id: Option<Option<String>>,
        is_sensitive: Option<bool>,
        comment: Option<Option<String>>,
    ) -> AppResult<drive_file::Model> {
        let file = self.file_repo.get_by_id(file_id).await?;

        // Verify ownership
        if file.user_id != user_id {
            return Err(AppError::Forbidden("Not your file".to_string()));
        }

        // Validate new folder if specified
        if let Some(Some(ref new_folder_id)) = folder_id {
            let folder = self.folder_repo.find_by_id(new_folder_id).await?;
            if let Some(f) = folder {
                if f.user_id != user_id {
                    return Err(AppError::Forbidden(
                        "Folder belongs to another user".to_string(),
                    ));
                }
            } else {
                return Err(AppError::NotFound("Folder not found".to_string()));
            }
        }

        let mut model: drive_file::ActiveModel = file.into();

        if let Some(n) = name {
            model.name = Set(n);
        }
        if let Some(fid) = folder_id {
            model.folder_id = Set(fid);
        }
        if let Some(s) = is_sensitive {
            model.is_sensitive = Set(s);
        }
        if let Some(c) = comment {
            model.comment = Set(c);
        }

        self.file_repo.update(model).await
    }

    /// Delete a file.
    pub async fn delete_file(&self, user_id: &str, file_id: &str) -> AppResult<()> {
        let file = self.file_repo.get_by_id(file_id).await?;

        // Verify ownership
        if file.user_id != user_id {
            return Err(AppError::Forbidden("Not your file".to_string()));
        }

        // Delete actual file from storage if storage backend is configured
        if let Some(ref storage) = self.storage
            && let Some(ref storage_key) = file.storage_key
            && let Err(e) = storage.delete(storage_key).await
        {
            tracing::warn!(
                file_id = %file_id,
                storage_key = %storage_key,
                error = %e,
                "Failed to delete file from storage, proceeding with database deletion"
            );
        }

        self.file_repo.delete(file_id).await
    }

    /// Get storage usage for a user.
    pub async fn get_storage_usage(&self, user_id: &str) -> AppResult<StorageUsage> {
        let used = self.file_repo.get_storage_used(user_id).await?;
        Ok(StorageUsage {
            used,
            limit: DEFAULT_STORAGE_LIMIT,
        })
    }

    /// Get unattached files (files not used in notes, pages, etc.)
    pub async fn get_unattached_files(
        &self,
        user_id: &str,
        limit: u64,
    ) -> AppResult<Vec<drive_file::Model>> {
        self.file_repo.find_unattached(user_id, limit).await
    }

    /// Count unattached files for a user.
    pub async fn count_unattached_files(&self, user_id: &str) -> AppResult<i64> {
        self.file_repo.count_unattached(user_id).await
    }

    /// Search files by name and/or comment (description).
    pub async fn search_files(
        &self,
        user_id: &str,
        query: &str,
        content_type: Option<&str>,
        folder_id: Option<Option<&str>>,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<drive_file::Model>> {
        self.file_repo
            .search(user_id, query, content_type, folder_id, limit, until_id)
            .await
    }

    /// Delete unattached files for a user.
    /// Returns the number of files deleted and total bytes freed.
    pub async fn cleanup_unattached_files(
        &self,
        user_id: &str,
        limit: u64,
    ) -> AppResult<CleanupResult> {
        let files = self.file_repo.find_unattached(user_id, limit).await?;

        if files.is_empty() {
            return Ok(CleanupResult {
                deleted_count: 0,
                freed_bytes: 0,
                file_ids: vec![],
            });
        }

        let file_ids: Vec<String> = files.iter().map(|f| f.id.clone()).collect();
        let total_size: i64 = files.iter().map(|f| f.size).sum();

        // Delete files from storage
        if let Some(ref storage) = self.storage {
            for file in &files {
                if let Some(ref storage_key) = file.storage_key
                    && let Err(e) = storage.delete(storage_key).await
                {
                    tracing::warn!(
                        file_id = %file.id,
                        storage_key = %storage_key,
                        error = %e,
                        "Failed to delete file from storage during cleanup"
                    );
                }
            }
        }

        // Delete from database
        let deleted = self.file_repo.delete_many(&file_ids).await?;

        Ok(CleanupResult {
            deleted_count: deleted,
            freed_bytes: total_size,
            file_ids,
        })
    }
}

/// Storage usage information.
pub struct StorageUsage {
    pub used: i64,
    pub limit: i64,
}

/// Result of cleaning up unattached files.
pub struct CleanupResult {
    pub deleted_count: u64,
    pub freed_bytes: i64,
    pub file_ids: Vec<String>,
}

/// Input for creating a folder.
pub struct CreateFolderInput {
    pub name: String,
    pub parent_id: Option<String>,
}

/// Folder service methods
impl DriveService {
    /// Create a new folder.
    pub async fn create_folder(
        &self,
        user_id: &str,
        input: CreateFolderInput,
    ) -> AppResult<drive_folder::Model> {
        // Validate folder name
        let name = input.name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("Folder name is required".to_string()));
        }
        if name.len() > 200 {
            return Err(AppError::BadRequest("Folder name too long".to_string()));
        }

        // Validate parent folder if specified
        if let Some(ref parent_id) = input.parent_id {
            let parent = self.folder_repo.find_by_id(parent_id).await?;
            if let Some(p) = parent {
                if p.user_id != user_id {
                    return Err(AppError::Forbidden(
                        "Parent folder belongs to another user".to_string(),
                    ));
                }
            } else {
                return Err(AppError::NotFound("Parent folder not found".to_string()));
            }
        }

        // Check for duplicate folder name in same parent
        if let Some(_existing) = self
            .folder_repo
            .find_by_name(user_id, name, input.parent_id.as_deref())
            .await?
        {
            return Err(AppError::BadRequest(
                "A folder with this name already exists".to_string(),
            ));
        }

        let folder_id = self.id_gen.generate();
        let model = drive_folder::ActiveModel {
            id: Set(folder_id),
            user_id: Set(user_id.to_string()),
            name: Set(name.to_string()),
            parent_id: Set(input.parent_id),
            created_at: Set(chrono::Utc::now().into()),
        };

        self.folder_repo.create(model).await
    }

    /// Get a folder by ID.
    pub async fn get_folder(&self, id: &str) -> AppResult<drive_folder::Model> {
        self.folder_repo.get_by_id(id).await
    }

    /// Get folders for a user.
    pub async fn get_user_folders(
        &self,
        user_id: &str,
        parent_id: Option<&str>,
        limit: u64,
    ) -> AppResult<Vec<drive_folder::Model>> {
        self.folder_repo
            .find_by_user(user_id, parent_id, limit)
            .await
    }

    /// Update a folder.
    pub async fn update_folder(
        &self,
        user_id: &str,
        folder_id: &str,
        name: Option<String>,
        parent_id: Option<Option<String>>,
    ) -> AppResult<drive_folder::Model> {
        let folder = self.folder_repo.get_by_id(folder_id).await?;

        // Verify ownership
        if folder.user_id != user_id {
            return Err(AppError::Forbidden("Not your folder".to_string()));
        }

        // Validate new parent if specified
        if let Some(Some(ref new_parent_id)) = parent_id {
            // Cannot move folder into itself
            if new_parent_id == folder_id {
                return Err(AppError::BadRequest(
                    "Cannot move folder into itself".to_string(),
                ));
            }

            let parent = self.folder_repo.find_by_id(new_parent_id).await?;
            if let Some(p) = parent {
                if p.user_id != user_id {
                    return Err(AppError::Forbidden(
                        "Parent folder belongs to another user".to_string(),
                    ));
                }
                // TODO: Check for circular references
            } else {
                return Err(AppError::NotFound("Parent folder not found".to_string()));
            }
        }

        let mut model: drive_folder::ActiveModel = folder.into();

        if let Some(n) = name {
            let n = n.trim();
            if n.is_empty() {
                return Err(AppError::BadRequest("Folder name is required".to_string()));
            }
            model.name = Set(n.to_string());
        }
        if let Some(pid) = parent_id {
            model.parent_id = Set(pid);
        }

        self.folder_repo.update(model).await
    }

    /// Delete a folder.
    pub async fn delete_folder(&self, user_id: &str, folder_id: &str) -> AppResult<()> {
        let folder = self.folder_repo.get_by_id(folder_id).await?;

        // Verify ownership
        if folder.user_id != user_id {
            return Err(AppError::Forbidden("Not your folder".to_string()));
        }

        // Note: Files and subfolders will be orphaned (parent_id set to null by DB cascade)
        self.folder_repo.delete(folder_id).await
    }
}

/// Generate a storage key for a file.
fn generate_storage_key(file_id: &str, original_name: &str) -> String {
    let extension = original_name
        .rsplit('.')
        .next()
        .filter(|ext| ext.len() <= 10 && ext.chars().all(char::is_alphanumeric))
        .unwrap_or("bin");

    format!("{file_id}.{extension}")
}

/// Get image dimensions from data.
fn get_image_dimensions(data: &[u8]) -> (Option<i32>, Option<i32>) {
    // Simple PNG dimension extraction
    if data.len() >= 24 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return (Some(width as i32), Some(height as i32));
    }

    // Simple JPEG dimension extraction (basic SOF0 marker)
    if data.len() > 2 && data[0] == 0xFF && data[1] == 0xD8 {
        let mut i = 2;
        while i + 9 < data.len() {
            if data[i] == 0xFF {
                let marker = data[i + 1];
                // SOF0, SOF1, SOF2 markers
                if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 {
                    let height = u16::from_be_bytes([data[i + 5], data[i + 6]]);
                    let width = u16::from_be_bytes([data[i + 7], data[i + 8]]);
                    return (Some(i32::from(width)), Some(i32::from(height)));
                }
                if marker == 0xD9 {
                    break; // End of image
                }
                let length = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                i += 2 + length;
            } else {
                i += 1;
            }
        }
    }

    (None, None)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_storage_key() {
        let key = generate_storage_key("abc123", "image.png");
        assert_eq!(key, "abc123.png");

        let key = generate_storage_key("abc123", "document.pdf");
        assert_eq!(key, "abc123.pdf");

        let key = generate_storage_key("abc123", "noextension");
        assert_eq!(key, "abc123.bin");
    }

    #[test]
    fn test_get_image_dimensions_png() {
        // Minimal PNG header with 100x50 dimensions
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0, 0, 0, 13]); // IHDR length
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&100u32.to_be_bytes()); // width
        data.extend_from_slice(&50u32.to_be_bytes()); // height

        let (width, height) = get_image_dimensions(&data);
        assert_eq!(width, Some(100));
        assert_eq!(height, Some(50));
    }
}
