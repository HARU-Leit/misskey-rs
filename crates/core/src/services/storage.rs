//! Storage service for file management.

use async_trait::async_trait;
use misskey_common::AppResult;
use std::path::PathBuf;

/// Storage backend trait for file operations.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Save file data to storage and return the storage key.
    async fn save(&self, key: &str, data: &[u8]) -> AppResult<()>;

    /// Delete a file from storage.
    async fn delete(&self, key: &str) -> AppResult<()>;

    /// Check if a file exists.
    async fn exists(&self, key: &str) -> AppResult<bool>;

    /// Get the public URL for a file.
    fn get_url(&self, key: &str) -> String;
}

/// Local filesystem storage backend.
#[derive(Clone)]
pub struct LocalStorage {
    /// Base directory for storing files.
    base_path: PathBuf,
    /// Base URL for accessing files.
    base_url: String,
}

impl LocalStorage {
    /// Create a new local storage backend.
    pub fn new(base_path: PathBuf, base_url: String) -> Self {
        Self { base_path, base_url }
    }

    /// Get the full path for a storage key.
    fn get_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn save(&self, key: &str, data: &[u8]) -> AppResult<()> {
        let path = self.get_path(key);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| misskey_common::AppError::Internal(format!("Failed to create directory: {e}")))?;
        }

        tokio::fs::write(&path, data)
            .await
            .map_err(|e| misskey_common::AppError::Internal(format!("Failed to write file: {e}")))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let path = self.get_path(key);

        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| misskey_common::AppError::Internal(format!("Failed to delete file: {e}")))?;
        }

        Ok(())
    }

    async fn exists(&self, key: &str) -> AppResult<bool> {
        let path = self.get_path(key);
        Ok(path.exists())
    }

    fn get_url(&self, key: &str) -> String {
        format!("{}/files/{}", self.base_url, key)
    }
}

/// No-op storage backend for testing or when file storage is disabled.
#[derive(Clone, Default)]
pub struct NoOpStorage {
    base_url: String,
}

impl NoOpStorage {
    /// Create a new no-op storage backend.
    #[must_use]
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl StorageBackend for NoOpStorage {
    async fn save(&self, _key: &str, _data: &[u8]) -> AppResult<()> {
        Ok(())
    }

    async fn delete(&self, _key: &str) -> AppResult<()> {
        Ok(())
    }

    async fn exists(&self, _key: &str) -> AppResult<bool> {
        Ok(false)
    }

    fn get_url(&self, key: &str) -> String {
        format!("{}/files/{}", self.base_url, key)
    }
}

/// Type alias for the storage service.
pub type StorageService = std::sync::Arc<dyn StorageBackend>;
