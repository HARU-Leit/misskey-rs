//! Federation security features.
//!
//! Provides replay attack protection and per-instance rate limiting
//! for incoming `ActivityPub` activities.

use chrono::{DateTime, Duration, Utc};
use fred::clients::Client as RedisClient;
use fred::interfaces::KeysInterface;
use fred::types::{Expiration, SetOptions};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Default maximum clock skew allowed: 5 minutes
const DEFAULT_MAX_CLOCK_SKEW_SECS: i64 = 5 * 60;

/// Default activity deduplication window: 48 hours
const DEFAULT_DEDUPE_WINDOW_SECS: i64 = 48 * 60 * 60;

/// Default rate limit window: 60 seconds
const DEFAULT_RATE_LIMIT_WINDOW_SECS: i64 = 60;

/// Default rate limit: 100 activities per window
const DEFAULT_RATE_LIMIT_MAX: u64 = 100;

/// Replay attack protection for `ActivityPub` activities.
#[derive(Clone)]
pub struct ReplayProtection {
    redis: Arc<RedisClient>,
    max_clock_skew: Duration,
    dedupe_window_secs: i64,
}

impl ReplayProtection {
    /// Create a new replay protection instance with default settings.
    #[must_use] 
    pub const fn new(redis: Arc<RedisClient>) -> Self {
        Self {
            redis,
            max_clock_skew: Duration::seconds(DEFAULT_MAX_CLOCK_SKEW_SECS),
            dedupe_window_secs: DEFAULT_DEDUPE_WINDOW_SECS,
        }
    }

    /// Create with custom settings.
    #[must_use] 
    pub const fn with_settings(
        redis: Arc<RedisClient>,
        max_clock_skew_secs: i64,
        dedupe_window_secs: i64,
    ) -> Self {
        Self {
            redis,
            max_clock_skew: Duration::seconds(max_clock_skew_secs),
            dedupe_window_secs,
        }
    }

    /// Validate the Date header is within acceptable clock skew.
    pub fn validate_timestamp(&self, date_header: &str) -> Result<(), ReplayError> {
        let activity_time = parse_http_date(date_header)?;
        let now = Utc::now();
        let diff = now.signed_duration_since(activity_time);

        if diff.abs() > self.max_clock_skew {
            warn!(
                date_header = %date_header,
                clock_skew_secs = diff.num_seconds(),
                max_allowed_secs = self.max_clock_skew.num_seconds(),
                "Signature expired due to clock skew"
            );
            return Err(ReplayError::ClockSkewTooLarge {
                skew_secs: diff.num_seconds(),
                max_secs: self.max_clock_skew.num_seconds(),
            });
        }

        debug!(
            date_header = %date_header,
            clock_skew_secs = diff.num_seconds(),
            "Timestamp validation passed"
        );

        Ok(())
    }

    /// Check if an activity ID has been seen before (deduplication).
    /// Returns Ok(()) if the activity is new, Err if it's a duplicate.
    pub async fn check_and_record_activity(&self, activity_id: &str) -> Result<(), ReplayError> {
        let key = format!("activity_seen:{activity_id}");

        // Try to set the key only if it doesn't exist (SETNX)
        let result: Option<String> = self
            .redis
            .set(
                key.clone(),
                "1",
                Some(Expiration::EX(self.dedupe_window_secs)),
                Some(SetOptions::NX),
                false,
            )
            .await
            .map_err(|e| ReplayError::Redis(e.to_string()))?;

        // NX returns None if key already exists, Some("OK") if set
        if result.is_some() {
            debug!(activity_id = %activity_id, "New activity recorded");
            Ok(())
        } else {
            warn!(activity_id = %activity_id, "Duplicate activity detected");
            Err(ReplayError::DuplicateActivity(activity_id.to_string()))
        }
    }

    /// Full replay protection check: timestamp + deduplication.
    pub async fn validate(
        &self,
        date_header: &str,
        activity_id: &str,
    ) -> Result<(), ReplayError> {
        // First check timestamp
        self.validate_timestamp(date_header)?;

        // Then check for duplicates
        self.check_and_record_activity(activity_id).await?;

        Ok(())
    }
}

/// Per-instance rate limiter for federation.
#[derive(Clone)]
pub struct FederationRateLimiter {
    redis: Arc<RedisClient>,
    window_secs: i64,
    max_activities: u64,
}

impl FederationRateLimiter {
    /// Create a new rate limiter with default settings.
    #[must_use] 
    pub const fn new(redis: Arc<RedisClient>) -> Self {
        Self {
            redis,
            window_secs: DEFAULT_RATE_LIMIT_WINDOW_SECS,
            max_activities: DEFAULT_RATE_LIMIT_MAX,
        }
    }

    /// Create with custom settings.
    #[must_use] 
    pub const fn with_settings(redis: Arc<RedisClient>, window_secs: i64, max_activities: u64) -> Self {
        Self {
            redis,
            window_secs,
            max_activities,
        }
    }

    /// Check if an instance is within rate limits.
    /// Returns Ok(remaining) if allowed, Err if rate limited.
    pub async fn check(&self, instance_host: &str) -> Result<u64, RateLimitError> {
        let window = current_window(self.window_secs);
        let key = format!("federation_rate:{instance_host}:{window}");

        // Increment counter
        let count: u64 = self
            .redis
            .incr(key.clone())
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        // Set expiry on first increment
        if count == 1 {
            self.redis
                .expire::<(), _>(key, self.window_secs, None)
                .await
                .map_err(|e| RateLimitError::Redis(e.to_string()))?;
        }

        if count > self.max_activities {
            warn!(
                instance = %instance_host,
                count = count,
                limit = self.max_activities,
                "Rate limit exceeded for instance"
            );
            return Err(RateLimitError::Exceeded {
                instance: instance_host.to_string(),
                count,
                limit: self.max_activities,
            });
        }

        let remaining = self.max_activities.saturating_sub(count);
        debug!(
            instance = %instance_host,
            count = count,
            remaining = remaining,
            "Rate limit check passed"
        );

        Ok(remaining)
    }

    /// Get current rate limit status for an instance.
    pub async fn status(&self, instance_host: &str) -> Result<RateLimitStatus, RateLimitError> {
        let window = current_window(self.window_secs);
        let key = format!("federation_rate:{instance_host}:{window}");

        let count: Option<u64> = self
            .redis
            .get(key.clone())
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let ttl: i64 = self
            .redis
            .ttl(key)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let count = count.unwrap_or(0);

        Ok(RateLimitStatus {
            instance: instance_host.to_string(),
            current_count: count,
            limit: self.max_activities,
            remaining: self.max_activities.saturating_sub(count),
            reset_in_secs: if ttl > 0 { ttl } else { self.window_secs },
        })
    }
}

/// Rate limit status for an instance.
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub instance: String,
    pub current_count: u64,
    pub limit: u64,
    pub remaining: u64,
    pub reset_in_secs: i64,
}

/// Combined security checker for incoming activities.
#[derive(Clone)]
pub struct ActivitySecurityChecker {
    replay_protection: ReplayProtection,
    rate_limiter: FederationRateLimiter,
}

impl ActivitySecurityChecker {
    /// Create a new security checker.
    #[must_use] 
    pub fn new(redis: Arc<RedisClient>) -> Self {
        Self {
            replay_protection: ReplayProtection::new(Arc::clone(&redis)),
            rate_limiter: FederationRateLimiter::new(redis),
        }
    }

    /// Create with custom settings.
    #[must_use] 
    pub fn with_settings(
        redis: Arc<RedisClient>,
        max_clock_skew_secs: i64,
        dedupe_window_secs: i64,
        rate_limit_window_secs: i64,
        rate_limit_max: u64,
    ) -> Self {
        Self {
            replay_protection: ReplayProtection::with_settings(
                Arc::clone(&redis),
                max_clock_skew_secs,
                dedupe_window_secs,
            ),
            rate_limiter: FederationRateLimiter::with_settings(
                redis,
                rate_limit_window_secs,
                rate_limit_max,
            ),
        }
    }

    /// Perform all security checks for an incoming activity.
    pub async fn check(
        &self,
        date_header: &str,
        activity_id: &str,
        instance_host: &str,
    ) -> Result<SecurityCheckResult, SecurityError> {
        // Check rate limit first (cheapest check)
        let remaining = self.rate_limiter.check(instance_host).await?;

        // Then check for replay attacks
        self.replay_protection.validate(date_header, activity_id).await?;

        info!(
            instance = %instance_host,
            activity_id = %activity_id,
            rate_limit_remaining = remaining,
            "Activity passed security checks"
        );

        Ok(SecurityCheckResult {
            rate_limit_remaining: remaining,
        })
    }

    /// Get the replay protection component.
    #[must_use] 
    pub const fn replay_protection(&self) -> &ReplayProtection {
        &self.replay_protection
    }

    /// Get the rate limiter component.
    #[must_use] 
    pub const fn rate_limiter(&self) -> &FederationRateLimiter {
        &self.rate_limiter
    }
}

/// Result of security checks.
#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    pub rate_limit_remaining: u64,
}

/// Replay attack error.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("Clock skew too large: {skew_secs}s (max: {max_secs}s)")]
    ClockSkewTooLarge { skew_secs: i64, max_secs: i64 },
    #[error("Duplicate activity: {0}")]
    DuplicateActivity(String),
    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),
    #[error("Redis error: {0}")]
    Redis(String),
}

/// Rate limit error.
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for {instance}: {count}/{limit}")]
    Exceeded {
        instance: String,
        count: u64,
        limit: u64,
    },
    #[error("Redis error: {0}")]
    Redis(String),
}

/// Combined security error.
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Replay attack: {0}")]
    Replay(#[from] ReplayError),
    #[error("Rate limit: {0}")]
    RateLimit(#[from] RateLimitError),
}

/// Parse HTTP Date header format (RFC 7231).
/// Example: "Sun, 06 Nov 1994 08:49:37 GMT"
fn parse_http_date(date_str: &str) -> Result<DateTime<Utc>, ReplayError> {
    // Try RFC 7231 format first
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try common HTTP date format
    let formats = [
        "%a, %d %b %Y %H:%M:%S GMT",     // RFC 7231
        "%a, %d %b %Y %H:%M:%S %z",       // With timezone
        "%A, %d-%b-%y %H:%M:%S GMT",      // RFC 850
        "%a %b %d %H:%M:%S %Y",           // ANSI C's asctime()
    ];

    for format in &formats {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, format) {
            return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
        }
    }

    Err(ReplayError::InvalidDateFormat(date_str.to_string()))
}

/// Get the current time window identifier.
fn current_window(window_secs: i64) -> i64 {
    Utc::now().timestamp() / window_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_date_rfc7231() {
        let date = "Sun, 06 Nov 1994 08:49:37 GMT";
        let result = parse_http_date(date);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_http_date_current() {
        let now = Utc::now();
        let date = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let result = parse_http_date(&date);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_http_date_invalid() {
        let date = "not a valid date";
        let result = parse_http_date(date);
        assert!(result.is_err());
    }

    #[test]
    fn test_current_window() {
        let window1 = current_window(60);
        let window2 = current_window(60);
        assert_eq!(window1, window2);
    }
}
