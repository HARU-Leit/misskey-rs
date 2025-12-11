//! Background job queue for misskey-rs.
//!
//! This crate provides asynchronous job processing using Redis:
//!
//! - **Jobs**: `ActivityPub` delivery, inbox processing
//! - **Workers**: Concurrent job execution with Apalis
//! - **Pub/Sub**: Real-time event broadcasting
//! - **Rate limiting**: Per-instance federation rate limits
//! - **Retry**: Exponential backoff with dead letter queue
//! - **Scheduler**: Periodic tasks (cleanup, aggregation)
//! - **Shared Inbox**: Optimized batch delivery

pub mod delivery_impl;
pub mod jobs;
pub mod pubsub;
pub mod rate_limit;
pub mod retry;
pub mod scheduler;
pub mod shared_inbox;
pub mod workers;

pub use delivery_impl::RedisDeliveryService;
pub use jobs::*;
pub use pubsub::{PubSubEvent, PubSubSseBridge, RedisPubSub, channels as pubsub_channels};
pub use rate_limit::{InstanceRateLimiter, RateLimitConfig, RateLimitResult};
pub use retry::{DeadLetterEntry, RetryConfig};
pub use scheduler::{JobExecutor, ScheduledJob, SchedulerConfig, SchedulerState};
pub use shared_inbox::{BatchDeliveryTarget, RecipientInfo};
pub use workers::*;
