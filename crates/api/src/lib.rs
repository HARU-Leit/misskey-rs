//! HTTP API layer for misskey-rs.
//!
//! This crate provides the REST API and real-time streaming:
//!
//! - **Endpoints**: Misskey-compatible and Mastodon-compatible APIs
//! - **Extractors**: Authentication, validation, pagination
//! - **Middleware**: Logging, CORS, rate limiting
//! - **Streaming**: WebSocket and Server-Sent Events
//!
//! Built on Axum 0.8 with Tower middleware stack.

// Allow dead_code for API compatibility fields in request structs
#![allow(dead_code)]

pub mod endpoints;
pub mod extractors;
pub mod middleware;
pub mod rate_limit;
pub mod response;
pub mod sse;
pub mod streaming;

pub use endpoints::router;
pub use rate_limit::{ApiRateLimiter, RateLimitConfig, RateLimiterState};
pub use sse::{SseBroadcaster, SseEvent};
pub use streaming::{StreamingState, streaming_handler};
