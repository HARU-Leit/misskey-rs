//! Metrics collection for misskey-rs.
//!
//! Provides application-level metrics for monitoring performance,
//! tracking usage patterns, and debugging issues.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Global metrics instance.
static METRICS: std::sync::OnceLock<Arc<Metrics>> = std::sync::OnceLock::new();

/// Get the global metrics instance.
pub fn get_metrics() -> &'static Arc<Metrics> {
    METRICS.get_or_init(|| Arc::new(Metrics::new()))
}

/// Initialize global metrics with custom instance.
pub fn init_metrics(metrics: Arc<Metrics>) -> Result<(), Arc<Metrics>> {
    METRICS.set(metrics)
}

/// Application metrics collector.
#[derive(Debug)]
pub struct Metrics {
    // === Request Metrics ===
    /// Total HTTP requests received
    pub http_requests_total: AtomicU64,
    /// Active HTTP requests
    pub http_requests_active: AtomicU64,
    /// HTTP requests by status code category (2xx, 4xx, 5xx)
    pub http_requests_2xx: AtomicU64,
    pub http_requests_4xx: AtomicU64,
    pub http_requests_5xx: AtomicU64,
    /// Total request latency in microseconds
    pub http_request_latency_us_total: AtomicU64,
    /// Request count for average calculation
    pub http_request_latency_count: AtomicU64,

    // === Database Metrics ===
    /// Total database queries executed
    pub db_queries_total: AtomicU64,
    /// Database query errors
    pub db_errors_total: AtomicU64,
    /// Total database query time in microseconds
    pub db_query_time_us_total: AtomicU64,
    /// Database query count for average calculation
    pub db_query_count: AtomicU64,

    // === Federation Metrics ===
    /// Activities received from remote instances
    pub federation_activities_received: AtomicU64,
    /// Activities delivered to remote instances
    pub federation_activities_delivered: AtomicU64,
    /// Federation delivery failures
    pub federation_delivery_failures: AtomicU64,
    /// Remote actor cache hits
    pub federation_cache_hits: AtomicU64,
    /// Remote actor cache misses
    pub federation_cache_misses: AtomicU64,
    /// Replay attacks blocked
    pub federation_replay_attacks_blocked: AtomicU64,
    /// Rate limit rejections
    pub federation_rate_limited: AtomicU64,

    // === Content Metrics ===
    /// Notes created
    pub notes_created: AtomicU64,
    /// Notes deleted
    pub notes_deleted: AtomicU64,
    /// Reactions created
    pub reactions_created: AtomicU64,
    /// Users registered
    pub users_registered: AtomicU64,
    /// Follow relationships created
    pub follows_created: AtomicU64,

    // === Real-time Metrics ===
    /// Active WebSocket connections
    pub websocket_connections_active: AtomicU64,
    /// Total WebSocket messages sent
    pub websocket_messages_sent: AtomicU64,
    /// Active SSE connections
    pub sse_connections_active: AtomicU64,

    // === Job Queue Metrics ===
    /// Jobs enqueued
    pub jobs_enqueued: AtomicU64,
    /// Jobs completed
    pub jobs_completed: AtomicU64,
    /// Jobs failed
    pub jobs_failed: AtomicU64,

    // === Search Metrics ===
    /// Full-text searches performed
    pub search_queries_total: AtomicU64,
    /// Full-text search time in microseconds
    pub search_time_us_total: AtomicU64,
}

impl Metrics {
    /// Create a new metrics instance with all counters at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            http_requests_total: AtomicU64::new(0),
            http_requests_active: AtomicU64::new(0),
            http_requests_2xx: AtomicU64::new(0),
            http_requests_4xx: AtomicU64::new(0),
            http_requests_5xx: AtomicU64::new(0),
            http_request_latency_us_total: AtomicU64::new(0),
            http_request_latency_count: AtomicU64::new(0),

            db_queries_total: AtomicU64::new(0),
            db_errors_total: AtomicU64::new(0),
            db_query_time_us_total: AtomicU64::new(0),
            db_query_count: AtomicU64::new(0),

            federation_activities_received: AtomicU64::new(0),
            federation_activities_delivered: AtomicU64::new(0),
            federation_delivery_failures: AtomicU64::new(0),
            federation_cache_hits: AtomicU64::new(0),
            federation_cache_misses: AtomicU64::new(0),
            federation_replay_attacks_blocked: AtomicU64::new(0),
            federation_rate_limited: AtomicU64::new(0),

            notes_created: AtomicU64::new(0),
            notes_deleted: AtomicU64::new(0),
            reactions_created: AtomicU64::new(0),
            users_registered: AtomicU64::new(0),
            follows_created: AtomicU64::new(0),

            websocket_connections_active: AtomicU64::new(0),
            websocket_messages_sent: AtomicU64::new(0),
            sse_connections_active: AtomicU64::new(0),

            jobs_enqueued: AtomicU64::new(0),
            jobs_completed: AtomicU64::new(0),
            jobs_failed: AtomicU64::new(0),

            search_queries_total: AtomicU64::new(0),
            search_time_us_total: AtomicU64::new(0),
        }
    }

    /// Record an HTTP request.
    pub fn record_http_request(&self, status_code: u16, latency: Duration) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);

        match status_code {
            200..=299 => self.http_requests_2xx.fetch_add(1, Ordering::Relaxed),
            400..=499 => self.http_requests_4xx.fetch_add(1, Ordering::Relaxed),
            500..=599 => self.http_requests_5xx.fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };

        self.http_request_latency_us_total
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
        self.http_request_latency_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Start tracking an active request.
    pub fn start_request(&self) {
        self.http_requests_active.fetch_add(1, Ordering::Relaxed);
    }

    /// End tracking an active request.
    pub fn end_request(&self) {
        self.http_requests_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record a database query.
    pub fn record_db_query(&self, duration: Duration, is_error: bool) {
        self.db_queries_total.fetch_add(1, Ordering::Relaxed);
        self.db_query_time_us_total
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.db_query_count.fetch_add(1, Ordering::Relaxed);

        if is_error {
            self.db_errors_total.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a federation activity received.
    pub fn record_activity_received(&self) {
        self.federation_activities_received
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a federation activity delivered.
    pub fn record_activity_delivered(&self, success: bool) {
        if success {
            self.federation_activities_delivered
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.federation_delivery_failures
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a cache hit/miss for remote actors.
    pub fn record_cache_access(&self, hit: bool) {
        if hit {
            self.federation_cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.federation_cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a blocked replay attack.
    pub fn record_replay_attack_blocked(&self) {
        self.federation_replay_attacks_blocked
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rate limit rejection.
    pub fn record_rate_limited(&self) {
        self.federation_rate_limited.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a search query.
    pub fn record_search(&self, duration: Duration) {
        self.search_queries_total.fetch_add(1, Ordering::Relaxed);
        self.search_time_us_total
            .fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    /// Get a snapshot of all metrics.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            http_requests_total: self.http_requests_total.load(Ordering::Relaxed),
            http_requests_active: self.http_requests_active.load(Ordering::Relaxed),
            http_requests_2xx: self.http_requests_2xx.load(Ordering::Relaxed),
            http_requests_4xx: self.http_requests_4xx.load(Ordering::Relaxed),
            http_requests_5xx: self.http_requests_5xx.load(Ordering::Relaxed),
            http_request_latency_avg_us: self.average_latency_us(),

            db_queries_total: self.db_queries_total.load(Ordering::Relaxed),
            db_errors_total: self.db_errors_total.load(Ordering::Relaxed),
            db_query_avg_time_us: self.average_db_query_time_us(),

            federation_activities_received: self
                .federation_activities_received
                .load(Ordering::Relaxed),
            federation_activities_delivered: self
                .federation_activities_delivered
                .load(Ordering::Relaxed),
            federation_delivery_failures: self
                .federation_delivery_failures
                .load(Ordering::Relaxed),
            federation_cache_hit_rate: self.cache_hit_rate(),
            federation_replay_attacks_blocked: self
                .federation_replay_attacks_blocked
                .load(Ordering::Relaxed),
            federation_rate_limited: self.federation_rate_limited.load(Ordering::Relaxed),

            notes_created: self.notes_created.load(Ordering::Relaxed),
            notes_deleted: self.notes_deleted.load(Ordering::Relaxed),
            reactions_created: self.reactions_created.load(Ordering::Relaxed),
            users_registered: self.users_registered.load(Ordering::Relaxed),
            follows_created: self.follows_created.load(Ordering::Relaxed),

            websocket_connections_active: self
                .websocket_connections_active
                .load(Ordering::Relaxed),
            websocket_messages_sent: self.websocket_messages_sent.load(Ordering::Relaxed),
            sse_connections_active: self.sse_connections_active.load(Ordering::Relaxed),

            jobs_enqueued: self.jobs_enqueued.load(Ordering::Relaxed),
            jobs_completed: self.jobs_completed.load(Ordering::Relaxed),
            jobs_failed: self.jobs_failed.load(Ordering::Relaxed),

            search_queries_total: self.search_queries_total.load(Ordering::Relaxed),
            search_avg_time_us: self.average_search_time_us(),
        }
    }

    /// Calculate average HTTP request latency.
    fn average_latency_us(&self) -> u64 {
        let total = self.http_request_latency_us_total.load(Ordering::Relaxed);
        let count = self.http_request_latency_count.load(Ordering::Relaxed);
        if count > 0 {
            total / count
        } else {
            0
        }
    }

    /// Calculate average database query time.
    fn average_db_query_time_us(&self) -> u64 {
        let total = self.db_query_time_us_total.load(Ordering::Relaxed);
        let count = self.db_query_count.load(Ordering::Relaxed);
        if count > 0 {
            total / count
        } else {
            0
        }
    }

    /// Calculate cache hit rate.
    fn cache_hit_rate(&self) -> f64 {
        let hits = self.federation_cache_hits.load(Ordering::Relaxed);
        let misses = self.federation_cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Calculate average search time.
    fn average_search_time_us(&self) -> u64 {
        let total = self.search_time_us_total.load(Ordering::Relaxed);
        let count = self.search_queries_total.load(Ordering::Relaxed);
        if count > 0 {
            total / count
        } else {
            0
        }
    }

    /// Export metrics in Prometheus format.
    #[must_use]
    pub fn to_prometheus(&self) -> String {
        let snapshot = self.snapshot();
        let mut output = String::new();

        // HTTP metrics
        output.push_str("# HELP misskey_http_requests_total Total HTTP requests\n");
        output.push_str("# TYPE misskey_http_requests_total counter\n");
        output.push_str(&format!(
            "misskey_http_requests_total {}\n",
            snapshot.http_requests_total
        ));

        output.push_str("# HELP misskey_http_requests_active Active HTTP requests\n");
        output.push_str("# TYPE misskey_http_requests_active gauge\n");
        output.push_str(&format!(
            "misskey_http_requests_active {}\n",
            snapshot.http_requests_active
        ));

        output.push_str("# HELP misskey_http_requests_by_status HTTP requests by status\n");
        output.push_str("# TYPE misskey_http_requests_by_status counter\n");
        output.push_str(&format!(
            "misskey_http_requests_by_status{{status=\"2xx\"}} {}\n",
            snapshot.http_requests_2xx
        ));
        output.push_str(&format!(
            "misskey_http_requests_by_status{{status=\"4xx\"}} {}\n",
            snapshot.http_requests_4xx
        ));
        output.push_str(&format!(
            "misskey_http_requests_by_status{{status=\"5xx\"}} {}\n",
            snapshot.http_requests_5xx
        ));

        output.push_str("# HELP misskey_http_request_latency_avg_us Average request latency\n");
        output.push_str("# TYPE misskey_http_request_latency_avg_us gauge\n");
        output.push_str(&format!(
            "misskey_http_request_latency_avg_us {}\n",
            snapshot.http_request_latency_avg_us
        ));

        // Database metrics
        output.push_str("# HELP misskey_db_queries_total Total database queries\n");
        output.push_str("# TYPE misskey_db_queries_total counter\n");
        output.push_str(&format!(
            "misskey_db_queries_total {}\n",
            snapshot.db_queries_total
        ));

        output.push_str("# HELP misskey_db_errors_total Database errors\n");
        output.push_str("# TYPE misskey_db_errors_total counter\n");
        output.push_str(&format!(
            "misskey_db_errors_total {}\n",
            snapshot.db_errors_total
        ));

        // Federation metrics
        output.push_str(
            "# HELP misskey_federation_activities_received Activities received from remote\n",
        );
        output.push_str("# TYPE misskey_federation_activities_received counter\n");
        output.push_str(&format!(
            "misskey_federation_activities_received {}\n",
            snapshot.federation_activities_received
        ));

        output.push_str(
            "# HELP misskey_federation_activities_delivered Activities delivered to remote\n",
        );
        output.push_str("# TYPE misskey_federation_activities_delivered counter\n");
        output.push_str(&format!(
            "misskey_federation_activities_delivered {}\n",
            snapshot.federation_activities_delivered
        ));

        output.push_str("# HELP misskey_federation_cache_hit_rate Remote actor cache hit rate\n");
        output.push_str("# TYPE misskey_federation_cache_hit_rate gauge\n");
        output.push_str(&format!(
            "misskey_federation_cache_hit_rate {:.4}\n",
            snapshot.federation_cache_hit_rate
        ));

        output.push_str(
            "# HELP misskey_federation_replay_attacks_blocked Replay attacks blocked\n",
        );
        output.push_str("# TYPE misskey_federation_replay_attacks_blocked counter\n");
        output.push_str(&format!(
            "misskey_federation_replay_attacks_blocked {}\n",
            snapshot.federation_replay_attacks_blocked
        ));

        // Content metrics
        output.push_str("# HELP misskey_notes_created Notes created\n");
        output.push_str("# TYPE misskey_notes_created counter\n");
        output.push_str(&format!(
            "misskey_notes_created {}\n",
            snapshot.notes_created
        ));

        // Real-time metrics
        output.push_str("# HELP misskey_websocket_connections Active WebSocket connections\n");
        output.push_str("# TYPE misskey_websocket_connections gauge\n");
        output.push_str(&format!(
            "misskey_websocket_connections {}\n",
            snapshot.websocket_connections_active
        ));

        output.push_str("# HELP misskey_sse_connections Active SSE connections\n");
        output.push_str("# TYPE misskey_sse_connections gauge\n");
        output.push_str(&format!(
            "misskey_sse_connections {}\n",
            snapshot.sse_connections_active
        ));

        // Job queue metrics
        output.push_str("# HELP misskey_jobs_enqueued Jobs enqueued\n");
        output.push_str("# TYPE misskey_jobs_enqueued counter\n");
        output.push_str(&format!(
            "misskey_jobs_enqueued {}\n",
            snapshot.jobs_enqueued
        ));

        output.push_str("# HELP misskey_jobs_completed Jobs completed\n");
        output.push_str("# TYPE misskey_jobs_completed counter\n");
        output.push_str(&format!(
            "misskey_jobs_completed {}\n",
            snapshot.jobs_completed
        ));

        output.push_str("# HELP misskey_jobs_failed Jobs failed\n");
        output.push_str("# TYPE misskey_jobs_failed counter\n");
        output.push_str(&format!("misskey_jobs_failed {}\n", snapshot.jobs_failed));

        output
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of all metrics at a point in time.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSnapshot {
    // HTTP
    pub http_requests_total: u64,
    pub http_requests_active: u64,
    pub http_requests_2xx: u64,
    pub http_requests_4xx: u64,
    pub http_requests_5xx: u64,
    pub http_request_latency_avg_us: u64,

    // Database
    pub db_queries_total: u64,
    pub db_errors_total: u64,
    pub db_query_avg_time_us: u64,

    // Federation
    pub federation_activities_received: u64,
    pub federation_activities_delivered: u64,
    pub federation_delivery_failures: u64,
    pub federation_cache_hit_rate: f64,
    pub federation_replay_attacks_blocked: u64,
    pub federation_rate_limited: u64,

    // Content
    pub notes_created: u64,
    pub notes_deleted: u64,
    pub reactions_created: u64,
    pub users_registered: u64,
    pub follows_created: u64,

    // Real-time
    pub websocket_connections_active: u64,
    pub websocket_messages_sent: u64,
    pub sse_connections_active: u64,

    // Jobs
    pub jobs_enqueued: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,

    // Search
    pub search_queries_total: u64,
    pub search_avg_time_us: u64,
}

/// Timer guard for measuring operation duration.
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Start a new timer.
    #[must_use]
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed duration since timer start.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = Metrics::new();
        assert_eq!(metrics.http_requests_total.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.notes_created.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_http_request() {
        let metrics = Metrics::new();

        metrics.record_http_request(200, Duration::from_millis(50));
        metrics.record_http_request(404, Duration::from_millis(10));
        metrics.record_http_request(500, Duration::from_millis(100));

        assert_eq!(metrics.http_requests_total.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.http_requests_2xx.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.http_requests_4xx.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.http_requests_5xx.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_active_request_tracking() {
        let metrics = Metrics::new();

        metrics.start_request();
        metrics.start_request();
        assert_eq!(metrics.http_requests_active.load(Ordering::Relaxed), 2);

        metrics.end_request();
        assert_eq!(metrics.http_requests_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_db_query() {
        let metrics = Metrics::new();

        metrics.record_db_query(Duration::from_micros(500), false);
        metrics.record_db_query(Duration::from_micros(1000), true);

        assert_eq!(metrics.db_queries_total.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.db_errors_total.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let metrics = Metrics::new();

        metrics.record_cache_access(true);
        metrics.record_cache_access(true);
        metrics.record_cache_access(true);
        metrics.record_cache_access(false);

        let rate = metrics.cache_hit_rate();
        assert!((rate - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_cache_hit_rate_zero() {
        let metrics = Metrics::new();
        assert_eq!(metrics.cache_hit_rate(), 0.0);
    }

    #[test]
    fn test_snapshot() {
        let metrics = Metrics::new();
        metrics.notes_created.fetch_add(10, Ordering::Relaxed);
        metrics.notes_deleted.fetch_add(2, Ordering::Relaxed);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.notes_created, 10);
        assert_eq!(snapshot.notes_deleted, 2);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Metrics::new();
        metrics.record_http_request(200, Duration::from_millis(50));

        let prometheus = metrics.to_prometheus();
        assert!(prometheus.contains("misskey_http_requests_total 1"));
        assert!(prometheus.contains("misskey_http_requests_by_status{status=\"2xx\"} 1"));
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_average_latency_empty() {
        let metrics = Metrics::new();
        assert_eq!(metrics.average_latency_us(), 0);
    }

    #[test]
    fn test_average_latency() {
        let metrics = Metrics::new();
        metrics.record_http_request(200, Duration::from_micros(100));
        metrics.record_http_request(200, Duration::from_micros(200));
        assert_eq!(metrics.average_latency_us(), 150);
    }
}
