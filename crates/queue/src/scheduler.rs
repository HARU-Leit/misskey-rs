//! Scheduled jobs for periodic maintenance tasks.

#![allow(missing_docs)]

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::interval;

/// Scheduled job types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduledJob {
    /// Clean up expired mutes.
    CleanupExpiredMutes,
    /// Clean up old notes (if configured).
    CleanupOldNotes { retention_days: u32 },
    /// Check instance health.
    InstanceHealthCheck,
    /// Aggregate statistics/charts.
    AggregateCharts,
    /// Process scheduled notes (due for posting).
    ProcessScheduledNotes,
    /// Clean up old completed scheduled notes.
    CleanupScheduledNotes { retention_days: u32 },
    /// Process recurring posts (due for posting).
    ProcessRecurringPosts,
}

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Interval for mute cleanup (default: 1 hour).
    pub mute_cleanup_interval: Duration,
    /// Interval for instance health check (default: 5 minutes).
    pub health_check_interval: Duration,
    /// Interval for chart aggregation (default: 1 hour).
    pub chart_aggregation_interval: Duration,
    /// Whether to run old note cleanup.
    pub enable_note_cleanup: bool,
    /// Retention period for old notes in days.
    pub note_retention_days: u32,
    /// Interval for processing scheduled notes (default: 30 seconds).
    pub scheduled_note_interval: Duration,
    /// Retention period for old completed scheduled notes in days.
    pub scheduled_note_retention_days: u32,
    /// Interval for processing recurring posts (default: 1 minute).
    pub recurring_post_interval: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            mute_cleanup_interval: Duration::from_secs(3600),
            health_check_interval: Duration::from_secs(300),
            chart_aggregation_interval: Duration::from_secs(3600),
            enable_note_cleanup: false,
            note_retention_days: 365,
            scheduled_note_interval: Duration::from_secs(30),
            scheduled_note_retention_days: 30,
            recurring_post_interval: Duration::from_secs(60),
        }
    }
}

/// Scheduler state for tracking job runs.
#[derive(Debug, Clone, Default)]
pub struct SchedulerState {
    pub last_mute_cleanup: Option<DateTime<Utc>>,
    pub last_health_check: Option<DateTime<Utc>>,
    pub last_chart_aggregation: Option<DateTime<Utc>>,
    pub last_note_cleanup: Option<DateTime<Utc>>,
    pub last_scheduled_note_process: Option<DateTime<Utc>>,
    pub last_scheduled_note_cleanup: Option<DateTime<Utc>>,
    pub last_recurring_post_process: Option<DateTime<Utc>>,
}

/// Job executor trait for scheduled jobs.
#[async_trait::async_trait]
pub trait JobExecutor: Send + Sync {
    /// Execute the cleanup expired mutes job.
    async fn cleanup_expired_mutes(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Execute the instance health check job.
    async fn instance_health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Execute the chart aggregation job.
    async fn aggregate_charts(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Execute the old note cleanup job.
    async fn cleanup_old_notes(
        &self,
        retention_days: u32,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Process scheduled notes that are due for posting.
    async fn process_scheduled_notes(
        &self,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Clean up old completed scheduled notes.
    async fn cleanup_scheduled_notes(
        &self,
        retention_days: u32,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Process recurring posts that are due for posting.
    async fn process_recurring_posts(
        &self,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
}

/// Run the scheduler with the given configuration and executor.
pub async fn run_scheduler<E: JobExecutor + 'static>(config: SchedulerConfig, executor: Arc<E>) {
    let executor_mute = executor.clone();
    let executor_health = executor.clone();
    let executor_chart = executor.clone();
    let executor_note = executor.clone();
    let executor_scheduled = executor.clone();
    let executor_scheduled_cleanup = executor.clone();
    let executor_recurring = executor;

    let mute_interval = config.mute_cleanup_interval;
    let health_interval = config.health_check_interval;
    let chart_interval = config.chart_aggregation_interval;
    let enable_note_cleanup = config.enable_note_cleanup;
    let note_retention_days = config.note_retention_days;
    let scheduled_note_interval = config.scheduled_note_interval;
    let scheduled_note_retention_days = config.scheduled_note_retention_days;
    let recurring_post_interval = config.recurring_post_interval;

    // Spawn mute cleanup task
    tokio::spawn(async move {
        let mut interval = interval(mute_interval);
        loop {
            interval.tick().await;
            match executor_mute.cleanup_expired_mutes().await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(count, "Cleaned up expired mutes");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to cleanup expired mutes");
                }
            }
        }
    });

    // Spawn health check task
    tokio::spawn(async move {
        let mut interval = interval(health_interval);
        loop {
            interval.tick().await;
            if let Err(e) = executor_health.instance_health_check().await {
                tracing::error!(error = %e, "Instance health check failed");
            }
        }
    });

    // Spawn chart aggregation task
    tokio::spawn(async move {
        let mut interval = interval(chart_interval);
        loop {
            interval.tick().await;
            if let Err(e) = executor_chart.aggregate_charts().await {
                tracing::error!(error = %e, "Chart aggregation failed");
            }
        }
    });

    // Spawn note cleanup task (if enabled)
    if enable_note_cleanup {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(86400)); // Daily
            loop {
                interval.tick().await;
                match executor_note.cleanup_old_notes(note_retention_days).await {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!(
                                count,
                                retention_days = note_retention_days,
                                "Cleaned up old notes"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to cleanup old notes");
                    }
                }
            }
        });
    }

    // Spawn scheduled note processing task
    tokio::spawn(async move {
        let mut interval = interval(scheduled_note_interval);
        loop {
            interval.tick().await;
            match executor_scheduled.process_scheduled_notes().await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(count, "Processed scheduled notes");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to process scheduled notes");
                }
            }
        }
    });

    // Spawn scheduled note cleanup task (daily)
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(86400)); // Daily
        loop {
            interval.tick().await;
            match executor_scheduled_cleanup
                .cleanup_scheduled_notes(scheduled_note_retention_days)
                .await
            {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(
                            count,
                            retention_days = scheduled_note_retention_days,
                            "Cleaned up old scheduled notes"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to cleanup old scheduled notes");
                }
            }
        }
    });

    // Spawn recurring post processing task
    tokio::spawn(async move {
        let mut interval = interval(recurring_post_interval);
        loop {
            interval.tick().await;
            match executor_recurring.process_recurring_posts().await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(count, "Processed recurring posts");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to process recurring posts");
                }
            }
        }
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.mute_cleanup_interval, Duration::from_secs(3600));
        assert_eq!(config.health_check_interval, Duration::from_secs(300));
        assert!(!config.enable_note_cleanup);
    }

    #[test]
    fn test_scheduler_state_default() {
        let state = SchedulerState::default();
        assert!(state.last_mute_cleanup.is_none());
        assert!(state.last_health_check.is_none());
    }
}
