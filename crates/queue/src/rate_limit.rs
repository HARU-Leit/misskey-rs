//! Rate limiting for federation instances.
//!
//! Provides per-instance rate limiting to prevent abuse from remote servers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// Rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window.
    pub max_requests: u32,
    /// Time window duration.
    pub window: Duration,
    /// Cooldown period after hitting the limit.
    pub cooldown: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(300),
        }
    }
}

/// Rate limit state for a single instance.
#[derive(Debug, Clone)]
struct InstanceState {
    /// Request count in current window.
    count: u32,
    /// Window start time.
    window_start: Instant,
    /// Cooldown end time (if in cooldown).
    cooldown_until: Option<Instant>,
}

impl InstanceState {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            cooldown_until: None,
        }
    }
}

/// Rate limit check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request is allowed.
    Allowed,
    /// Request is rate limited.
    Limited {
        /// Seconds until rate limit resets.
        retry_after: u64,
    },
    /// Instance is in cooldown.
    Cooldown {
        /// Seconds until cooldown ends.
        retry_after: u64,
    },
}

/// Per-instance rate limiter.
#[derive(Clone)]
pub struct InstanceRateLimiter {
    config: RateLimitConfig,
    states: Arc<RwLock<HashMap<String, InstanceState>>>,
}

impl InstanceRateLimiter {
    /// Create a new rate limiter with the given configuration.
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request from the given instance is allowed.
    pub async fn check(&self, instance: &str) -> RateLimitResult {
        let mut states = self.states.write().await;
        let now = Instant::now();

        let state = states
            .entry(instance.to_string())
            .or_insert_with(InstanceState::new);

        // Check if in cooldown
        if let Some(cooldown_until) = state.cooldown_until {
            if now < cooldown_until {
                let retry_after = cooldown_until.duration_since(now).as_secs();
                return RateLimitResult::Cooldown { retry_after };
            }
            // Cooldown expired, reset state
            state.cooldown_until = None;
            state.count = 0;
            state.window_start = now;
        }

        // Check if window has expired
        if now.duration_since(state.window_start) >= self.config.window {
            state.count = 0;
            state.window_start = now;
        }

        // Check if rate limited
        if state.count >= self.config.max_requests {
            // Enter cooldown
            state.cooldown_until = Some(now + self.config.cooldown);
            let retry_after = self.config.cooldown.as_secs();
            return RateLimitResult::Cooldown { retry_after };
        }

        // Increment count and allow
        state.count += 1;
        RateLimitResult::Allowed
    }

    /// Record a request from the given instance.
    pub async fn record(&self, instance: &str) {
        let _ = self.check(instance).await;
    }

    /// Get current state for an instance.
    pub async fn get_state(&self, instance: &str) -> Option<(u32, u64)> {
        let states = self.states.read().await;
        states.get(instance).map(|s| {
            let remaining = self.config.max_requests.saturating_sub(s.count);
            let window_remaining = self
                .config
                .window
                .saturating_sub(Instant::now().duration_since(s.window_start))
                .as_secs();
            (remaining, window_remaining)
        })
    }

    /// Reset rate limit for an instance.
    pub async fn reset(&self, instance: &str) {
        let mut states = self.states.write().await;
        states.remove(instance);
    }

    /// Clean up expired entries.
    pub async fn cleanup(&self) {
        let mut states = self.states.write().await;
        let now = Instant::now();
        let window = self.config.window;

        states.retain(|_, state| {
            // Keep if in cooldown or window hasn't expired
            state.cooldown_until.is_some()
                || now.duration_since(state.window_start) < window * 2
        });
    }

    /// Get the number of tracked instances.
    pub async fn instance_count(&self) -> usize {
        self.states.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_initial_requests() {
        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(300),
        };
        let limiter = InstanceRateLimiter::new(config);

        for _ in 0..5 {
            assert_eq!(limiter.check("example.com").await, RateLimitResult::Allowed);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_after_limit() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(10),
        };
        let limiter = InstanceRateLimiter::new(config);

        // Use up the limit
        for _ in 0..3 {
            assert_eq!(limiter.check("example.com").await, RateLimitResult::Allowed);
        }

        // Should be in cooldown
        match limiter.check("example.com").await {
            RateLimitResult::Cooldown { retry_after } => {
                assert!(retry_after > 0);
                assert!(retry_after <= 10);
            }
            _ => panic!("Expected Cooldown"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_separate_instances() {
        let config = RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(10),
        };
        let limiter = InstanceRateLimiter::new(config);

        // Use up limit for instance A
        limiter.check("a.example.com").await;
        limiter.check("a.example.com").await;

        // Instance B should still be allowed
        assert_eq!(
            limiter.check("b.example.com").await,
            RateLimitResult::Allowed
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let config = RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(300),
        };
        let limiter = InstanceRateLimiter::new(config);

        // Use up limit
        limiter.check("example.com").await;

        // Reset
        limiter.reset("example.com").await;

        // Should be allowed again
        assert_eq!(limiter.check("example.com").await, RateLimitResult::Allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_get_state() {
        let config = RateLimitConfig {
            max_requests: 10,
            window: Duration::from_secs(60),
            cooldown: Duration::from_secs(300),
        };
        let limiter = InstanceRateLimiter::new(config);

        // Make some requests
        limiter.check("example.com").await;
        limiter.check("example.com").await;
        limiter.check("example.com").await;

        let state = limiter.get_state("example.com").await;
        assert!(state.is_some());
        let (remaining, _) = state.unwrap();
        assert_eq!(remaining, 7); // 10 - 3
    }

    #[tokio::test]
    async fn test_rate_limiter_instance_count() {
        let config = RateLimitConfig::default();
        let limiter = InstanceRateLimiter::new(config);

        limiter.check("a.example.com").await;
        limiter.check("b.example.com").await;
        limiter.check("c.example.com").await;

        assert_eq!(limiter.instance_count().await, 3);
    }
}
