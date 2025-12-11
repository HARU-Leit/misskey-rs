//! API rate limiting middleware.
//!
//! Provides per-user and per-IP rate limiting for API endpoints.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::RwLock;

/// Rate limit configuration for different endpoint types.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window.
    pub max_requests: u32,
    /// Time window duration in seconds.
    pub window_secs: u64,
}

impl RateLimitConfig {
    /// Create a new rate limit config.
    pub const fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
        }
    }
}

/// Default rate limits for different endpoint categories.
pub mod limits {
    use super::RateLimitConfig;

    /// Standard API endpoints (most operations).
    pub const STANDARD: RateLimitConfig = RateLimitConfig::new(300, 60);

    /// Write operations (create note, reaction, etc.).
    pub const WRITE: RateLimitConfig = RateLimitConfig::new(30, 60);

    /// Heavy operations (search, file upload, etc.).
    pub const HEAVY: RateLimitConfig = RateLimitConfig::new(10, 60);

    /// Authentication endpoints.
    pub const AUTH: RateLimitConfig = RateLimitConfig::new(10, 300);

    /// Signup endpoint (very restrictive).
    pub const SIGNUP: RateLimitConfig = RateLimitConfig::new(5, 3600);
}

/// Rate limit state for a single key.
#[derive(Debug, Clone)]
struct RateLimitState {
    /// Request count in current window.
    count: u32,
    /// Window start time.
    window_start: Instant,
}

impl RateLimitState {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
        }
    }
}

/// API rate limiter.
#[derive(Clone)]
pub struct ApiRateLimiter {
    /// State per key (user ID or IP address).
    states: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

impl Default for ApiRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRateLimiter {
    /// Create a new rate limiter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request is allowed and record it.
    pub async fn check(&self, key: &str, config: &RateLimitConfig) -> RateLimitResult {
        let mut states = self.states.write().await;
        let now = Instant::now();
        let window = Duration::from_secs(config.window_secs);

        let state = states
            .entry(key.to_string())
            .or_insert_with(RateLimitState::new);

        // Check if window has expired
        if now.duration_since(state.window_start) >= window {
            state.count = 0;
            state.window_start = now;
        }

        // Check if rate limited
        if state.count >= config.max_requests {
            let retry_after = window
                .saturating_sub(now.duration_since(state.window_start))
                .as_secs();
            return RateLimitResult::Limited {
                retry_after,
                remaining: 0,
                limit: config.max_requests,
            };
        }

        // Increment count and allow
        state.count += 1;
        let remaining = config.max_requests.saturating_sub(state.count);

        RateLimitResult::Allowed {
            remaining,
            limit: config.max_requests,
            reset: window
                .saturating_sub(now.duration_since(state.window_start))
                .as_secs(),
        }
    }

    /// Clean up expired entries.
    pub async fn cleanup(&self, max_window_secs: u64) {
        let mut states = self.states.write().await;
        let now = Instant::now();
        let max_window = Duration::from_secs(max_window_secs * 2);

        states.retain(|_, state| now.duration_since(state.window_start) < max_window);
    }

    /// Get the number of tracked keys.
    pub async fn key_count(&self) -> usize {
        self.states.read().await.len()
    }
}

/// Rate limit check result.
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request is allowed.
    Allowed {
        /// Remaining requests in window.
        remaining: u32,
        /// Total limit.
        limit: u32,
        /// Seconds until window reset.
        reset: u64,
    },
    /// Request is rate limited.
    Limited {
        /// Seconds until rate limit resets.
        retry_after: u64,
        /// Remaining requests (0).
        remaining: u32,
        /// Total limit.
        limit: u32,
    },
}

/// Rate limiter state for middleware.
#[derive(Clone)]
pub struct RateLimiterState {
    /// Per-user rate limiter.
    pub user_limiter: ApiRateLimiter,
    /// Per-IP rate limiter (for unauthenticated requests).
    pub ip_limiter: ApiRateLimiter,
}

impl Default for RateLimiterState {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiterState {
    /// Create a new rate limiter state.
    pub fn new() -> Self {
        Self {
            user_limiter: ApiRateLimiter::new(),
            ip_limiter: ApiRateLimiter::new(),
        }
    }
}

/// Rate limit error response.
#[derive(Debug)]
pub struct RateLimitError {
    pub retry_after: u64,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": {
                "code": "RATE_LIMIT_EXCEEDED",
                "message": "Too many requests",
                "retryAfter": self.retry_after
            }
        });

        (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Retry-After", self.retry_after.to_string()),
                ("Content-Type", "application/json".to_string()),
            ],
            body.to_string(),
        )
            .into_response()
    }
}

/// Extract client IP from request.
fn extract_client_ip(req: &Request<Body>) -> Option<IpAddr> {
    // Try X-Forwarded-For header first
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(first_ip) = xff_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    None
}

/// Rate limiting middleware.
pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiterState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, RateLimitError> {
    rate_limit_with_config(limiter, req, next, &limits::STANDARD).await
}

/// Rate limiting middleware for write operations.
pub async fn rate_limit_write_middleware(
    State(limiter): State<RateLimiterState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, RateLimitError> {
    rate_limit_with_config(limiter, req, next, &limits::WRITE).await
}

/// Rate limiting middleware for heavy operations.
pub async fn rate_limit_heavy_middleware(
    State(limiter): State<RateLimiterState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, RateLimitError> {
    rate_limit_with_config(limiter, req, next, &limits::HEAVY).await
}

/// Rate limiting middleware for auth operations.
pub async fn rate_limit_auth_middleware(
    State(limiter): State<RateLimiterState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, RateLimitError> {
    rate_limit_with_config(limiter, req, next, &limits::AUTH).await
}

/// Rate limiting middleware with custom config.
async fn rate_limit_with_config(
    limiter: RateLimiterState,
    req: Request<Body>,
    next: Next,
    config: &RateLimitConfig,
) -> Result<Response, RateLimitError> {
    // Determine the rate limit key
    // Try to get user ID from extensions (set by auth middleware)
    let key = if let Some(user) = req.extensions().get::<misskey_db::entities::user::Model>() {
        format!("user:{}", user.id)
    } else if let Some(ip) = extract_client_ip(&req) {
        format!("ip:{ip}")
    } else {
        // Fallback to a generic key
        "unknown".to_string()
    };

    // Choose the appropriate limiter
    let result = if key.starts_with("user:") {
        limiter.user_limiter.check(&key, config).await
    } else {
        limiter.ip_limiter.check(&key, config).await
    };

    match result {
        RateLimitResult::Allowed {
            remaining,
            limit,
            reset,
        } => {
            let mut response = next.run(req).await;

            // Add rate limit headers
            let headers = response.headers_mut();
            headers.insert("X-RateLimit-Limit", limit.into());
            headers.insert("X-RateLimit-Remaining", remaining.into());
            headers.insert("X-RateLimit-Reset", reset.into());

            Ok(response)
        }
        RateLimitResult::Limited { retry_after, .. } => Err(RateLimitError { retry_after }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_rate_limiter_allows_requests() {
        let limiter = ApiRateLimiter::new();
        let config = RateLimitConfig::new(5, 60);

        for _ in 0..5 {
            match limiter.check("test_user", &config).await {
                RateLimitResult::Allowed { .. } => {}
                _ => panic!("Expected Allowed"),
            }
        }
    }

    #[tokio::test]
    async fn test_api_rate_limiter_blocks_after_limit() {
        let limiter = ApiRateLimiter::new();
        let config = RateLimitConfig::new(3, 60);

        // Use up the limit
        for _ in 0..3 {
            limiter.check("test_user", &config).await;
        }

        // Should be limited
        match limiter.check("test_user", &config).await {
            RateLimitResult::Limited { retry_after, .. } => {
                assert!(retry_after > 0);
            }
            _ => panic!("Expected Limited"),
        }
    }

    #[tokio::test]
    async fn test_api_rate_limiter_separate_keys() {
        let limiter = ApiRateLimiter::new();
        let config = RateLimitConfig::new(2, 60);

        // Use up limit for user A
        limiter.check("user_a", &config).await;
        limiter.check("user_a", &config).await;

        // User B should still be allowed
        match limiter.check("user_b", &config).await {
            RateLimitResult::Allowed { .. } => {}
            _ => panic!("Expected Allowed for user_b"),
        }
    }

    #[tokio::test]
    async fn test_rate_limit_headers() {
        let limiter = ApiRateLimiter::new();
        let config = RateLimitConfig::new(10, 60);

        match limiter.check("test", &config).await {
            RateLimitResult::Allowed {
                remaining,
                limit,
                reset,
            } => {
                assert_eq!(limit, 10);
                assert_eq!(remaining, 9);
                assert!(reset <= 60);
            }
            _ => panic!("Expected Allowed"),
        }
    }

    #[tokio::test]
    async fn test_cleanup() {
        let limiter = ApiRateLimiter::new();
        let config = RateLimitConfig::new(10, 1); // 1 second window

        limiter.check("user1", &config).await;
        limiter.check("user2", &config).await;

        assert_eq!(limiter.key_count().await, 2);

        // Wait a bit and cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
        limiter.cleanup(1).await;

        // Keys should still exist (not expired yet within 2x window)
        // In real usage, cleanup would be called periodically
    }
}
