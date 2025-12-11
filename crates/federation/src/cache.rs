//! Remote actor caching with Redis.
//!
//! Provides caching for remote `ActivityPub` actors to reduce network requests
//! and improve performance. Implements a 24-hour TTL cache with automatic
//! invalidation on Update activities.

#![allow(missing_docs)]

use fred::clients::Client as RedisClient;
use fred::interfaces::KeysInterface;
use fred::types::Expiration;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Default cache TTL: 24 hours
const DEFAULT_CACHE_TTL_SECS: i64 = 24 * 60 * 60;

/// Short cache TTL for failed lookups: 5 minutes
const FAILED_LOOKUP_TTL_SECS: i64 = 5 * 60;

/// Cached remote actor data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRemoteActor {
    /// Actor's `ActivityPub` ID (URL)
    pub id: String,
    /// Actor type (Person, Service, etc.)
    pub actor_type: String,
    /// Preferred username
    pub preferred_username: String,
    /// Display name
    pub name: Option<String>,
    /// Actor summary/bio
    pub summary: Option<String>,
    /// Inbox URL
    pub inbox: String,
    /// Shared inbox URL (optional)
    pub shared_inbox: Option<String>,
    /// Outbox URL (optional)
    pub outbox: Option<String>,
    /// Followers collection URL (optional)
    pub followers: Option<String>,
    /// Following collection URL (optional)
    pub following: Option<String>,
    /// Public key ID
    pub public_key_id: String,
    /// Public key PEM
    pub public_key_pem: String,
    /// Avatar icon URL (optional)
    pub icon: Option<String>,
    /// Header/banner image URL (optional)
    pub image: Option<String>,
    /// Host domain
    pub host: String,
    /// When this cache entry was created
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

impl CachedRemoteActor {
    /// Create a cached actor from raw `ActivityPub` JSON.
    pub fn from_json(json: &serde_json::Value, host: &str) -> Option<Self> {
        let id = json.get("id")?.as_str()?.to_string();
        let actor_type = json.get("type")?.as_str()?.to_string();
        let preferred_username = json.get("preferredUsername")?.as_str()?.to_string();
        let name = json.get("name").and_then(|v| v.as_str()).map(String::from);
        let summary = json
            .get("summary")
            .and_then(|v| v.as_str())
            .map(String::from);
        let inbox = json.get("inbox")?.as_str()?.to_string();

        // Handle endpoints object for shared inbox
        let shared_inbox = json
            .get("endpoints")
            .and_then(|e| e.get("sharedInbox"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                json.get("sharedInbox")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            });

        let outbox = json
            .get("outbox")
            .and_then(|v| v.as_str())
            .map(String::from);
        let followers = json
            .get("followers")
            .and_then(|v| v.as_str())
            .map(String::from);
        let following = json
            .get("following")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Public key
        let public_key = json.get("publicKey")?;
        let public_key_id = public_key.get("id")?.as_str()?.to_string();
        let public_key_pem = public_key.get("publicKeyPem")?.as_str()?.to_string();

        // Icon (avatar)
        let icon = json
            .get("icon")
            .and_then(|i| {
                if i.is_object() {
                    i.get("url").and_then(|v| v.as_str())
                } else {
                    i.as_str()
                }
            })
            .map(String::from);

        // Image (header/banner)
        let image = json
            .get("image")
            .and_then(|i| {
                if i.is_object() {
                    i.get("url").and_then(|v| v.as_str())
                } else {
                    i.as_str()
                }
            })
            .map(String::from);

        Some(Self {
            id,
            actor_type,
            preferred_username,
            name,
            summary,
            inbox,
            shared_inbox,
            outbox,
            followers,
            following,
            public_key_id,
            public_key_pem,
            icon,
            image,
            host: host.to_string(),
            cached_at: chrono::Utc::now(),
        })
    }

    /// Check if this cache entry is stale.
    #[must_use]
    pub fn is_stale(&self, ttl_secs: i64) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.cached_at);
        age.num_seconds() > ttl_secs
    }
}

/// Remote actor cache using Redis.
#[derive(Clone)]
pub struct RemoteActorCache {
    redis: Arc<RedisClient>,
    ttl_secs: i64,
}

impl RemoteActorCache {
    /// Create a new remote actor cache.
    #[must_use]
    pub const fn new(redis: Arc<RedisClient>) -> Self {
        Self {
            redis,
            ttl_secs: DEFAULT_CACHE_TTL_SECS,
        }
    }

    /// Create a new remote actor cache with custom TTL.
    #[must_use]
    pub const fn with_ttl(redis: Arc<RedisClient>, ttl: Duration) -> Self {
        Self {
            redis,
            ttl_secs: ttl.as_secs() as i64,
        }
    }

    /// Generate cache key for an actor URL.
    fn cache_key(actor_url: &str) -> String {
        format!("remote_actor:{actor_url}")
    }

    /// Generate cache key for a failed lookup (negative cache).
    fn failed_key(actor_url: &str) -> String {
        format!("remote_actor_failed:{actor_url}")
    }

    /// Get a cached actor by URL.
    pub async fn get(&self, actor_url: &str) -> Result<Option<CachedRemoteActor>, CacheError> {
        let key = Self::cache_key(actor_url);

        let result: Option<String> = self
            .redis
            .get(key.clone())
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        if let Some(json_str) = result {
            let actor: CachedRemoteActor = serde_json::from_str(&json_str)
                .map_err(|e| CacheError::Serialization(e.to_string()))?;

            // Check if stale (should not happen with Redis EXPIRE, but just in case)
            if actor.is_stale(self.ttl_secs) {
                debug!(actor_url = %actor_url, "Cache entry is stale, will refresh");
                return Ok(None);
            }

            debug!(actor_url = %actor_url, "Cache hit for remote actor");
            Ok(Some(actor))
        } else {
            debug!(actor_url = %actor_url, "Cache miss for remote actor");
            Ok(None)
        }
    }

    /// Check if a lookup previously failed (negative cache).
    pub async fn is_failed_lookup(&self, actor_url: &str) -> Result<bool, CacheError> {
        let key = Self::failed_key(actor_url);

        let exists: i64 = self
            .redis
            .exists(key)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        Ok(exists > 0)
    }

    /// Store a cached actor.
    pub async fn set(&self, actor: &CachedRemoteActor) -> Result<(), CacheError> {
        let key = Self::cache_key(&actor.id);
        let json_str =
            serde_json::to_string(actor).map_err(|e| CacheError::Serialization(e.to_string()))?;

        self.redis
            .set::<(), _, _>(
                key,
                json_str,
                Some(Expiration::EX(self.ttl_secs)),
                None,
                false,
            )
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        info!(
            actor_url = %actor.id,
            host = %actor.host,
            "Cached remote actor"
        );

        Ok(())
    }

    /// Mark a lookup as failed (negative cache).
    pub async fn set_failed(&self, actor_url: &str) -> Result<(), CacheError> {
        let key = Self::failed_key(actor_url);

        self.redis
            .set::<(), _, _>(
                key,
                "1",
                Some(Expiration::EX(FAILED_LOOKUP_TTL_SECS)),
                None,
                false,
            )
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        warn!(actor_url = %actor_url, "Marked actor lookup as failed");

        Ok(())
    }

    /// Invalidate a cached actor (e.g., on Update activity).
    pub async fn invalidate(&self, actor_url: &str) -> Result<(), CacheError> {
        let key = Self::cache_key(actor_url);

        self.redis
            .del::<(), _>(key)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        info!(actor_url = %actor_url, "Invalidated cached remote actor");

        Ok(())
    }

    /// Clear the failed lookup cache for an actor.
    pub async fn clear_failed(&self, actor_url: &str) -> Result<(), CacheError> {
        let key = Self::failed_key(actor_url);

        self.redis
            .del::<(), _>(key)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        Ok(())
    }

    /// Get cache statistics (for monitoring).
    /// Note: This method is for debugging/admin purposes.
    /// In production, consider using Redis INFO command instead.
    pub async fn stats(&self) -> Result<CacheStats, CacheError> {
        // For now, return placeholder stats
        // Full implementation would require scanning keys which is expensive
        // Consider using Redis DBSIZE or INFO commands for production
        Ok(CacheStats {
            cached_actors: 0,
            failed_lookups: 0,
        })
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached actors
    pub cached_actors: usize,
    /// Number of failed lookups in negative cache
    pub failed_lookups: usize,
}

/// Cache error type.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Redis error: {0}")]
    Redis(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cached_remote_actor_from_json() {
        let json = json!({
            "id": "https://example.com/users/test",
            "type": "Person",
            "preferredUsername": "test",
            "name": "Test User",
            "summary": "A test user",
            "inbox": "https://example.com/users/test/inbox",
            "outbox": "https://example.com/users/test/outbox",
            "followers": "https://example.com/users/test/followers",
            "following": "https://example.com/users/test/following",
            "endpoints": {
                "sharedInbox": "https://example.com/inbox"
            },
            "publicKey": {
                "id": "https://example.com/users/test#main-key",
                "publicKeyPem": "-----BEGIN PUBLIC KEY-----\nMIIB...\n-----END PUBLIC KEY-----"
            },
            "icon": {
                "type": "Image",
                "url": "https://example.com/avatars/test.png"
            }
        });

        let actor = CachedRemoteActor::from_json(&json, "example.com").unwrap();

        assert_eq!(actor.id, "https://example.com/users/test");
        assert_eq!(actor.actor_type, "Person");
        assert_eq!(actor.preferred_username, "test");
        assert_eq!(actor.name, Some("Test User".to_string()));
        assert_eq!(actor.inbox, "https://example.com/users/test/inbox");
        assert_eq!(
            actor.shared_inbox,
            Some("https://example.com/inbox".to_string())
        );
        assert_eq!(actor.host, "example.com");
        assert_eq!(
            actor.icon,
            Some("https://example.com/avatars/test.png".to_string())
        );
    }

    #[test]
    fn test_cached_remote_actor_from_json_minimal() {
        let json = json!({
            "id": "https://example.com/users/minimal",
            "type": "Person",
            "preferredUsername": "minimal",
            "inbox": "https://example.com/users/minimal/inbox",
            "publicKey": {
                "id": "https://example.com/users/minimal#main-key",
                "publicKeyPem": "-----BEGIN PUBLIC KEY-----\nMIIB...\n-----END PUBLIC KEY-----"
            }
        });

        let actor = CachedRemoteActor::from_json(&json, "example.com").unwrap();

        assert_eq!(actor.id, "https://example.com/users/minimal");
        assert!(actor.name.is_none());
        assert!(actor.shared_inbox.is_none());
    }

    #[test]
    fn test_cache_key_generation() {
        let key = RemoteActorCache::cache_key("https://example.com/users/test");
        assert_eq!(key, "remote_actor:https://example.com/users/test");
    }
}
