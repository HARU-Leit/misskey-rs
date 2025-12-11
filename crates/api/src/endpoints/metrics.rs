//! Metrics endpoints for monitoring and observability.
//!
//! Provides endpoints for:
//! - Prometheus metrics export
//! - Health checks
//! - Performance statistics

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use misskey_common::metrics::{MetricsSnapshot, get_metrics};
use serde::Serialize;

use crate::middleware::AppState;

/// Create the metrics router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_metrics_json))
        .route("/prometheus", get(get_metrics_prometheus))
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
}

/// JSON metrics response.
#[derive(Serialize)]
pub struct MetricsResponse {
    pub http: HttpMetrics,
    pub database: DatabaseMetrics,
    pub federation: FederationMetrics,
    pub content: ContentMetrics,
    pub realtime: RealtimeMetrics,
    pub jobs: JobMetrics,
    pub search: SearchMetrics,
}

#[derive(Serialize)]
pub struct HttpMetrics {
    pub requests_total: u64,
    pub requests_active: u64,
    pub requests_2xx: u64,
    pub requests_4xx: u64,
    pub requests_5xx: u64,
    pub latency_avg_us: u64,
}

#[derive(Serialize)]
pub struct DatabaseMetrics {
    pub queries_total: u64,
    pub errors_total: u64,
    pub query_avg_time_us: u64,
}

#[derive(Serialize)]
pub struct FederationMetrics {
    pub activities_received: u64,
    pub activities_delivered: u64,
    pub delivery_failures: u64,
    pub cache_hit_rate: f64,
    pub replay_attacks_blocked: u64,
    pub rate_limited: u64,
}

#[derive(Serialize)]
pub struct ContentMetrics {
    pub notes_created: u64,
    pub notes_deleted: u64,
    pub reactions_created: u64,
    pub users_registered: u64,
    pub follows_created: u64,
}

#[derive(Serialize)]
pub struct RealtimeMetrics {
    pub websocket_connections_active: u64,
    pub websocket_messages_sent: u64,
    pub sse_connections_active: u64,
}

#[derive(Serialize)]
pub struct JobMetrics {
    pub jobs_enqueued: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
}

#[derive(Serialize)]
pub struct SearchMetrics {
    pub queries_total: u64,
    pub avg_time_us: u64,
}

impl From<MetricsSnapshot> for MetricsResponse {
    fn from(s: MetricsSnapshot) -> Self {
        Self {
            http: HttpMetrics {
                requests_total: s.http_requests_total,
                requests_active: s.http_requests_active,
                requests_2xx: s.http_requests_2xx,
                requests_4xx: s.http_requests_4xx,
                requests_5xx: s.http_requests_5xx,
                latency_avg_us: s.http_request_latency_avg_us,
            },
            database: DatabaseMetrics {
                queries_total: s.db_queries_total,
                errors_total: s.db_errors_total,
                query_avg_time_us: s.db_query_avg_time_us,
            },
            federation: FederationMetrics {
                activities_received: s.federation_activities_received,
                activities_delivered: s.federation_activities_delivered,
                delivery_failures: s.federation_delivery_failures,
                cache_hit_rate: s.federation_cache_hit_rate,
                replay_attacks_blocked: s.federation_replay_attacks_blocked,
                rate_limited: s.federation_rate_limited,
            },
            content: ContentMetrics {
                notes_created: s.notes_created,
                notes_deleted: s.notes_deleted,
                reactions_created: s.reactions_created,
                users_registered: s.users_registered,
                follows_created: s.follows_created,
            },
            realtime: RealtimeMetrics {
                websocket_connections_active: s.websocket_connections_active,
                websocket_messages_sent: s.websocket_messages_sent,
                sse_connections_active: s.sse_connections_active,
            },
            jobs: JobMetrics {
                jobs_enqueued: s.jobs_enqueued,
                jobs_completed: s.jobs_completed,
                jobs_failed: s.jobs_failed,
            },
            search: SearchMetrics {
                queries_total: s.search_queries_total,
                avg_time_us: s.search_avg_time_us,
            },
        }
    }
}

/// Get metrics in JSON format.
async fn get_metrics_json() -> Json<MetricsResponse> {
    let snapshot = get_metrics().snapshot();
    Json(MetricsResponse::from(snapshot))
}

/// Get metrics in Prometheus text format.
async fn get_metrics_prometheus() -> Response {
    let prometheus_output = get_metrics().to_prometheus();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        prometheus_output,
    )
        .into_response()
}

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Simple health check (liveness probe).
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness check response.
#[derive(Serialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: ReadinessChecks,
}

#[derive(Serialize)]
pub struct ReadinessChecks {
    pub database: CheckResult,
    pub redis: CheckResult,
}

#[derive(Serialize)]
pub struct CheckResult {
    pub status: String,
    pub latency_ms: Option<u64>,
}

/// Readiness check (readiness probe).
async fn readiness_check(State(state): State<AppState>) -> (StatusCode, Json<ReadinessResponse>) {
    let start = std::time::Instant::now();

    // Check database connectivity via announcement service count
    let db_check = match state.announcement_service.count().await {
        Ok(_) => CheckResult {
            status: "ok".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(e) => CheckResult {
            status: format!("error: {e}"),
            latency_ms: None,
        },
    };

    let db_ok = db_check.status == "ok";

    // Redis check would go here
    let redis_check = CheckResult {
        status: "ok".to_string(),
        latency_ms: Some(0),
    };

    let all_ready = db_ok;
    let status = if all_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(ReadinessResponse {
            ready: all_ready,
            checks: ReadinessChecks {
                database: db_check,
                redis: redis_check,
            },
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_response_from_snapshot() {
        let snapshot = MetricsSnapshot {
            http_requests_total: 100,
            http_requests_active: 5,
            http_requests_2xx: 90,
            http_requests_4xx: 8,
            http_requests_5xx: 2,
            http_request_latency_avg_us: 1500,

            db_queries_total: 50,
            db_errors_total: 2,
            db_query_avg_time_us: 500,

            federation_activities_received: 200,
            federation_activities_delivered: 150,
            federation_delivery_failures: 10,
            federation_cache_hit_rate: 0.8,
            federation_replay_attacks_blocked: 3,
            federation_rate_limited: 1,

            notes_created: 500,
            notes_deleted: 10,
            reactions_created: 1000,
            users_registered: 25,
            follows_created: 100,

            websocket_connections_active: 10,
            websocket_messages_sent: 5000,
            sse_connections_active: 5,

            jobs_enqueued: 1000,
            jobs_completed: 990,
            jobs_failed: 5,

            search_queries_total: 50,
            search_avg_time_us: 2000,
        };

        let response = MetricsResponse::from(snapshot);

        assert_eq!(response.http.requests_total, 100);
        assert_eq!(response.http.latency_avg_us, 1500);
        assert_eq!(response.federation.cache_hit_rate, 0.8);
        assert_eq!(response.content.notes_created, 500);
        assert_eq!(response.search.queries_total, 50);
    }

    #[test]
    fn test_metrics_response_from_zero_snapshot() {
        let snapshot = MetricsSnapshot {
            http_requests_total: 0,
            http_requests_active: 0,
            http_requests_2xx: 0,
            http_requests_4xx: 0,
            http_requests_5xx: 0,
            http_request_latency_avg_us: 0,

            db_queries_total: 0,
            db_errors_total: 0,
            db_query_avg_time_us: 0,

            federation_activities_received: 0,
            federation_activities_delivered: 0,
            federation_delivery_failures: 0,
            federation_cache_hit_rate: 0.0,
            federation_replay_attacks_blocked: 0,
            federation_rate_limited: 0,

            notes_created: 0,
            notes_deleted: 0,
            reactions_created: 0,
            users_registered: 0,
            follows_created: 0,

            websocket_connections_active: 0,
            websocket_messages_sent: 0,
            sse_connections_active: 0,

            jobs_enqueued: 0,
            jobs_completed: 0,
            jobs_failed: 0,

            search_queries_total: 0,
            search_avg_time_us: 0,
        };

        let response = MetricsResponse::from(snapshot);

        // Should not panic on zero values
        assert_eq!(response.http.latency_avg_us, 0);
        assert_eq!(response.database.query_avg_time_us, 0);
        assert_eq!(response.federation.cache_hit_rate, 0.0);
    }
}
