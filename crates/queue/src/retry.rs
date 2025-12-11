//! Retry configuration and dead letter queue handling.

#![allow(missing_docs)]

use std::time::Duration;

/// Retry configuration with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay between retries.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Multiplier for exponential backoff.
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_secs(60),       // 1 minute
            max_delay: Duration::from_secs(3600 * 24),    // 24 hours
            multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for the given attempt number (0-indexed).
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt >= self.max_retries {
            return self.max_delay;
        }

        let delay_secs = self.initial_delay.as_secs_f64() * self.multiplier.powi(attempt as i32);
        let delay = Duration::from_secs_f64(delay_secs);

        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }

    /// Check if we should retry after the given number of attempts.
    #[must_use]
    pub const fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

/// Dead letter queue entry for failed jobs.
#[derive(Debug, Clone)]
pub struct DeadLetterEntry<T> {
    /// The failed job.
    pub job: T,
    /// Number of attempts made.
    pub attempts: u32,
    /// Last error message.
    pub last_error: String,
    /// Timestamp of last failure.
    pub failed_at: chrono::DateTime<chrono::Utc>,
}

impl<T> DeadLetterEntry<T> {
    /// Create a new dead letter entry.
    pub fn new(job: T, attempts: u32, error: String) -> Self {
        Self {
            job,
            attempts,
            last_error: error,
            failed_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let config = RetryConfig::default();

        // First retry: 60s
        assert_eq!(config.delay_for_attempt(0), Duration::from_secs(60));
        // Second retry: 120s
        assert_eq!(config.delay_for_attempt(1), Duration::from_secs(120));
        // Third retry: 240s
        assert_eq!(config.delay_for_attempt(2), Duration::from_secs(240));
        // Fourth retry: 480s
        assert_eq!(config.delay_for_attempt(3), Duration::from_secs(480));
    }

    #[test]
    fn test_max_delay() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay: Duration::from_secs(3600),
            max_delay: Duration::from_secs(7200),
            multiplier: 2.0,
        };

        // Should be capped at max_delay
        assert_eq!(config.delay_for_attempt(5), Duration::from_secs(7200));
    }

    #[test]
    fn test_should_retry() {
        let config = RetryConfig {
            max_retries: 3,
            ..Default::default()
        };

        assert!(config.should_retry(0));
        assert!(config.should_retry(1));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3));
        assert!(!config.should_retry(4));
    }
}
