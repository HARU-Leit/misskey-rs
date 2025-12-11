//! URL preview caching with Redis.
//!
//! Provides caching for URL previews to reduce external HTTP requests
//! and improve performance when displaying link cards.
//!
//! # Features
//!
//! - TTL-based cache expiration (default 24 hours for successful lookups)
//! - Negative caching for failed/invalid URLs (default 1 hour)
//! - Automatic serialization/deserialization of `UrlPreview` data
//!
//! # Example
//!
//! ```ignore
//! use misskey_common::url_preview_cache::UrlPreviewCache;
//! use fred::clients::Client as RedisClient;
//! use std::sync::Arc;
//!
//! let cache = UrlPreviewCache::new(Arc::new(redis_client));
//!
//! // Get cached preview or fetch and cache
//! if let Some(preview) = cache.get("https://example.com").await? {
//!     // Use cached preview
//! } else {
//!     // Fetch preview and cache it
//!     let preview = fetch_preview("https://example.com", &config).await;
//!     if let Some(p) = preview {
//!         cache.set(&p).await?;
//!     } else {
//!         cache.set_failed("https://example.com").await?;
//!     }
//! }
//! ```

use crate::url_preview::UrlPreview;
use fred::clients::Client as RedisClient;
use fred::interfaces::KeysInterface;
use fred::types::Expiration;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Default cache TTL: 24 hours for successful previews
const DEFAULT_CACHE_TTL_SECS: i64 = 24 * 60 * 60;

/// Short cache TTL for failed lookups: 1 hour
const FAILED_LOOKUP_TTL_SECS: i64 = 60 * 60;

/// URL preview cache using Redis.
#[derive(Clone)]
pub struct UrlPreviewCache {
    redis: Arc<RedisClient>,
    ttl_secs: i64,
    failed_ttl_secs: i64,
}

impl UrlPreviewCache {
    /// Create a new URL preview cache with default TTL settings.
    #[must_use]
    pub const fn new(redis: Arc<RedisClient>) -> Self {
        Self {
            redis,
            ttl_secs: DEFAULT_CACHE_TTL_SECS,
            failed_ttl_secs: FAILED_LOOKUP_TTL_SECS,
        }
    }

    /// Create a new URL preview cache with custom TTL.
    #[must_use]
    pub const fn with_ttl(redis: Arc<RedisClient>, ttl: Duration, failed_ttl: Duration) -> Self {
        Self {
            redis,
            ttl_secs: ttl.as_secs() as i64,
            failed_ttl_secs: failed_ttl.as_secs() as i64,
        }
    }

    /// Generate cache key for a URL.
    fn cache_key(url: &str) -> String {
        format!("url_preview:{url}")
    }

    /// Generate cache key for a failed lookup (negative cache).
    fn failed_key(url: &str) -> String {
        format!("url_preview_failed:{url}")
    }

    /// Get a cached URL preview.
    ///
    /// Returns `Ok(Some(preview))` if cached, `Ok(None)` if not cached.
    pub async fn get(&self, url: &str) -> Result<Option<UrlPreview>, UrlPreviewCacheError> {
        let key = Self::cache_key(url);

        let result: Option<String> = self
            .redis
            .get(key.clone())
            .await
            .map_err(|e| UrlPreviewCacheError::Redis(e.to_string()))?;

        if let Some(json_str) = result {
            let preview: UrlPreview = serde_json::from_str(&json_str)
                .map_err(|e| UrlPreviewCacheError::Serialization(e.to_string()))?;

            debug!(url = %url, "Cache hit for URL preview");
            Ok(Some(preview))
        } else {
            debug!(url = %url, "Cache miss for URL preview");
            Ok(None)
        }
    }

    /// Check if a lookup previously failed (negative cache).
    ///
    /// Returns `true` if the URL is known to be invalid/unreachable.
    pub async fn is_failed_lookup(&self, url: &str) -> Result<bool, UrlPreviewCacheError> {
        let key = Self::failed_key(url);

        let exists: i64 = self
            .redis
            .exists(key)
            .await
            .map_err(|e| UrlPreviewCacheError::Redis(e.to_string()))?;

        Ok(exists > 0)
    }

    /// Store a URL preview in cache.
    pub async fn set(&self, preview: &UrlPreview) -> Result<(), UrlPreviewCacheError> {
        let key = Self::cache_key(&preview.url);
        let json_str = serde_json::to_string(preview)
            .map_err(|e| UrlPreviewCacheError::Serialization(e.to_string()))?;

        self.redis
            .set::<(), _, _>(
                key,
                json_str,
                Some(Expiration::EX(self.ttl_secs)),
                None,
                false,
            )
            .await
            .map_err(|e| UrlPreviewCacheError::Redis(e.to_string()))?;

        info!(url = %preview.url, "Cached URL preview");

        // Clear any previous failed lookup marker
        let _ = self.clear_failed(&preview.url).await;

        Ok(())
    }

    /// Mark a lookup as failed (negative cache).
    ///
    /// This prevents repeated attempts to fetch an invalid/unreachable URL.
    pub async fn set_failed(&self, url: &str) -> Result<(), UrlPreviewCacheError> {
        let key = Self::failed_key(url);

        self.redis
            .set::<(), _, _>(
                key,
                "1",
                Some(Expiration::EX(self.failed_ttl_secs)),
                None,
                false,
            )
            .await
            .map_err(|e| UrlPreviewCacheError::Redis(e.to_string()))?;

        warn!(url = %url, "Marked URL preview lookup as failed");

        Ok(())
    }

    /// Invalidate a cached URL preview.
    pub async fn invalidate(&self, url: &str) -> Result<(), UrlPreviewCacheError> {
        let key = Self::cache_key(url);

        self.redis
            .del::<(), _>(key)
            .await
            .map_err(|e| UrlPreviewCacheError::Redis(e.to_string()))?;

        info!(url = %url, "Invalidated cached URL preview");

        Ok(())
    }

    /// Clear the failed lookup cache for a URL.
    pub async fn clear_failed(&self, url: &str) -> Result<(), UrlPreviewCacheError> {
        let key = Self::failed_key(url);

        self.redis
            .del::<(), _>(key)
            .await
            .map_err(|e| UrlPreviewCacheError::Redis(e.to_string()))?;

        Ok(())
    }

    /// Get a URL preview, using cache or fetching if not cached.
    ///
    /// This is a convenience method that:
    /// 1. Checks the cache for an existing preview
    /// 2. Checks if the URL is in the failed lookup cache
    /// 3. If not cached, fetches the preview and caches the result
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to get a preview for
    /// * `config` - Configuration for the URL preview fetcher
    pub async fn get_or_fetch(
        &self,
        url: &str,
        config: &crate::url_preview::UrlPreviewConfig,
    ) -> Result<Option<UrlPreview>, UrlPreviewCacheError> {
        // Check cache first
        if let Some(preview) = self.get(url).await? {
            return Ok(Some(preview));
        }

        // Check if previously failed
        if self.is_failed_lookup(url).await? {
            debug!(url = %url, "URL is in failed lookup cache, skipping fetch");
            return Ok(None);
        }

        // Fetch and cache
        let preview = crate::url_preview::fetch_preview(url, config).await;

        if let Some(p) = preview {
            self.set(&p).await?;
            Ok(Some(p))
        } else {
            self.set_failed(url).await?;
            Ok(None)
        }
    }
}

/// URL preview cache error type.
#[derive(Debug, thiserror::Error)]
pub enum UrlPreviewCacheError {
    /// Redis operation failed.
    #[error("Redis error: {0}")]
    Redis(String),

    /// JSON serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key = UrlPreviewCache::cache_key("https://example.com/page");
        assert_eq!(key, "url_preview:https://example.com/page");
    }

    #[test]
    fn test_failed_key_generation() {
        let key = UrlPreviewCache::failed_key("https://example.com/invalid");
        assert_eq!(key, "url_preview_failed:https://example.com/invalid");
    }
}
