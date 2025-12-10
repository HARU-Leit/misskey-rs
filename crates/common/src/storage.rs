//! Object storage abstraction for file uploads.
//!
//! Supports both local filesystem and S3-compatible object storage.

use std::path::PathBuf;

use crate::{AppError, AppResult};

/// Storage configuration.
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// Local filesystem storage.
    Local {
        /// Base path for stored files.
        base_path: PathBuf,
        /// Base URL for serving files.
        base_url: String,
    },
    /// S3-compatible object storage.
    S3 {
        /// S3 endpoint URL (e.g., "<https://s3.amazonaws.com>" or `MinIO` URL).
        endpoint: String,
        /// S3 bucket name.
        bucket: String,
        /// AWS region.
        region: String,
        /// Access key ID.
        access_key_id: String,
        /// Secret access key.
        secret_access_key: String,
        /// Public URL prefix for serving files.
        public_url: Option<String>,
        /// Path prefix within the bucket.
        prefix: Option<String>,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self::Local {
            base_path: PathBuf::from("./files"),
            base_url: "/files".to_string(),
        }
    }
}

/// Uploaded file metadata.
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Storage key (path or object key).
    pub key: String,
    /// Public URL to access the file.
    pub url: String,
    /// File size in bytes.
    pub size: u64,
    /// MIME content type.
    pub content_type: String,
    /// MD5 hash of the file.
    pub md5: String,
}

/// Storage backend trait.
#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    /// Upload a file.
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> AppResult<UploadedFile>;

    /// Delete a file.
    async fn delete(&self, key: &str) -> AppResult<()>;

    /// Get the public URL for a key.
    fn public_url(&self, key: &str) -> String;

    /// Check if a file exists.
    async fn exists(&self, key: &str) -> AppResult<bool>;
}

/// Local filesystem storage backend.
pub struct LocalStorage {
    base_path: PathBuf,
    base_url: String,
}

impl LocalStorage {
    /// Create a new local storage backend.
    #[must_use] 
    pub const fn new(base_path: PathBuf, base_url: String) -> Self {
        Self { base_path, base_url }
    }
}

#[async_trait::async_trait]
impl StorageBackend for LocalStorage {
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> AppResult<UploadedFile> {
        let path = self.base_path.join(key);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create directory: {e}")))?;
        }

        // Write file
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to write file: {e}")))?;

        // Calculate MD5
        let md5 = format!("{:x}", md5::compute(data));

        Ok(UploadedFile {
            key: key.to_string(),
            url: self.public_url(key),
            size: data.len() as u64,
            content_type: content_type.to_string(),
            md5,
        })
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let path = self.base_path.join(key);
        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to delete file: {e}")))?;
        }
        Ok(())
    }

    fn public_url(&self, key: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), key)
    }

    async fn exists(&self, key: &str) -> AppResult<bool> {
        let path = self.base_path.join(key);
        Ok(path.exists())
    }
}

/// S3-compatible object storage backend.
#[cfg(feature = "s3")]
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    bucket: String,
    public_url: Option<String>,
    prefix: Option<String>,
}

#[cfg(feature = "s3")]
impl S3Storage {
    /// Create a new S3 storage backend.
    pub async fn new(
        endpoint: &str,
        bucket: String,
        region: &str,
        access_key_id: &str,
        secret_access_key: &str,
        public_url: Option<String>,
        prefix: Option<String>,
    ) -> AppResult<Self> {
        use aws_config::Region;
        use aws_sdk_s3::config::Credentials;

        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "misskey-rs",
        );

        let config = aws_sdk_s3::Config::builder()
            .endpoint_url(endpoint)
            .region(Region::new(region.to_string()))
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();

        let client = aws_sdk_s3::Client::from_conf(config);

        Ok(Self {
            client,
            bucket,
            public_url,
            prefix,
        })
    }

    fn full_key(&self, key: &str) -> String {
        match &self.prefix {
            Some(prefix) => format!("{}/{}", prefix.trim_end_matches('/'), key),
            None => key.to_string(),
        }
    }
}

#[cfg(feature = "s3")]
#[async_trait::async_trait]
impl StorageBackend for S3Storage {
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> AppResult<UploadedFile> {
        use aws_sdk_s3::primitives::ByteStream;

        let full_key = self.full_key(key);
        let md5 = format!("{:x}", md5::compute(data));

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(ByteStream::from(data.to_vec()))
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 upload failed: {}", e)))?;

        Ok(UploadedFile {
            key: key.to_string(),
            url: self.public_url(key),
            size: data.len() as u64,
            content_type: content_type.to_string(),
            md5,
        })
    }

    async fn delete(&self, key: &str) -> AppResult<()> {
        let full_key = self.full_key(key);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 delete failed: {}", e)))?;

        Ok(())
    }

    fn public_url(&self, key: &str) -> String {
        let full_key = self.full_key(key);
        match &self.public_url {
            Some(base) => format!("{}/{}", base.trim_end_matches('/'), full_key),
            None => format!("https://{}.s3.amazonaws.com/{}", self.bucket, full_key),
        }
    }

    async fn exists(&self, key: &str) -> AppResult<bool> {
        let full_key = self.full_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") || e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(AppError::Internal(format!("S3 head_object failed: {}", e)))
                }
            }
        }
    }
}

/// Generate a unique storage key for a file.
#[must_use] 
pub fn generate_storage_key(user_id: &str, original_name: &str) -> String {
    use chrono::Utc;

    let now = Utc::now();
    let date_path = now.format("%Y/%m/%d").to_string();
    let timestamp = now.timestamp_millis();

    // Extract extension from original name
    let extension = original_name
        .rfind('.')
        .filter(|&pos| pos > 0 && pos < original_name.len() - 1)
        .map(|pos| &original_name[pos + 1..])
        .filter(|ext| ext.len() <= 10 && !ext.is_empty())
        .unwrap_or("bin");

    format!("{}/{}/{}_{}.{}", date_path, user_id, timestamp, uuid::Uuid::new_v4(), extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_storage_key() {
        let key = generate_storage_key("user123", "photo.jpg");
        assert!(key.contains("user123"));
        assert!(key.ends_with(".jpg"));
        assert!(key.contains('/'));
    }

    #[test]
    fn test_generate_storage_key_no_extension() {
        let key = generate_storage_key("user123", "file");
        assert!(key.ends_with(".bin"));
    }
}
