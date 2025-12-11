//! Job processing service for background tasks.
//!
//! This module provides a simple in-memory job queue for processing
//! background tasks like sending push notifications, webhooks, etc.

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::services::push_notification::{
    PushNotificationService, PushNotificationType, PushPayload,
};
use crate::services::webhook::WebhookService;

/// Maximum number of concurrent job workers.
const MAX_WORKERS: usize = 4;

/// Channel buffer size for jobs.
const JOB_BUFFER_SIZE: usize = 1000;

/// Job types that can be processed.
#[derive(Debug, Clone)]
pub enum Job {
    /// Send push notification to a user.
    PushNotification {
        user_id: String,
        notification_type: PushNotificationType,
        payload: PushPayload,
    },
    /// Send webhook to registered endpoints.
    Webhook {
        user_id: String,
        event_type: String,
        payload: serde_json::Value,
    },
    /// Clean up expired data.
    Cleanup { task: CleanupTask },
}

/// Cleanup task types.
#[derive(Debug, Clone)]
pub enum CleanupTask {
    /// Clean up expired word filters.
    ExpiredWordFilters,
    /// Clean up expired push subscriptions.
    ExpiredPushSubscriptions,
    /// Clean up expired sessions.
    ExpiredSessions,
    /// Clean up old notifications.
    OldNotifications,
}

/// Job sender for enqueueing jobs.
#[derive(Clone)]
pub struct JobSender {
    sender: mpsc::Sender<Job>,
}

impl JobSender {
    /// Enqueue a job for processing.
    pub async fn enqueue(&self, job: Job) -> Result<(), &'static str> {
        self.sender.send(job).await.map_err(|_| "Job queue is full")
    }

    /// Enqueue a push notification job.
    pub async fn push_notification(
        &self,
        user_id: String,
        notification_type: PushNotificationType,
        payload: PushPayload,
    ) -> Result<(), &'static str> {
        self.enqueue(Job::PushNotification {
            user_id,
            notification_type,
            payload,
        })
        .await
    }

    /// Enqueue a webhook job.
    pub async fn webhook(
        &self,
        user_id: String,
        event_type: String,
        payload: serde_json::Value,
    ) -> Result<(), &'static str> {
        self.enqueue(Job::Webhook {
            user_id,
            event_type,
            payload,
        })
        .await
    }

    /// Enqueue a cleanup job.
    pub async fn cleanup(&self, task: CleanupTask) -> Result<(), &'static str> {
        self.enqueue(Job::Cleanup { task }).await
    }
}

/// Job worker context containing services needed for job processing.
pub struct JobWorkerContext {
    pub push_service: Option<PushNotificationService>,
    pub webhook_service: Option<WebhookService>,
}

impl Clone for JobWorkerContext {
    fn clone(&self) -> Self {
        Self {
            push_service: self.push_service.clone(),
            webhook_service: self.webhook_service.clone(),
        }
    }
}

/// Job processing service.
pub struct JobService {
    sender: mpsc::Sender<Job>,
    receiver: Option<mpsc::Receiver<Job>>,
}

impl JobService {
    /// Create a new job service.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(JOB_BUFFER_SIZE);
        Self {
            sender,
            receiver: Some(receiver),
        }
    }

    /// Get a job sender for enqueueing jobs.
    #[must_use]
    pub fn sender(&self) -> JobSender {
        JobSender {
            sender: self.sender.clone(),
        }
    }

    /// Start the job processor with the given context.
    /// This consumes the receiver and spawns worker tasks.
    pub fn start(mut self, context: JobWorkerContext) {
        let receiver = self.receiver.take().expect("Job service already started");
        let context = Arc::new(context);

        tokio::spawn(async move {
            info!("Job worker starting with {} workers", MAX_WORKERS);
            run_job_processor(receiver, context).await;
            info!("Job worker stopped");
        });
    }
}

impl Default for JobService {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the job processor.
async fn run_job_processor(mut receiver: mpsc::Receiver<Job>, context: Arc<JobWorkerContext>) {
    // Use a semaphore to limit concurrent workers
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_WORKERS));

    while let Some(job) = receiver.recv().await {
        let permit = semaphore.clone().acquire_owned().await;
        let ctx = context.clone();

        tokio::spawn(async move {
            let _permit = permit;
            process_job(job, &ctx).await;
        });
    }
}

/// Process a single job.
async fn process_job(job: Job, context: &JobWorkerContext) {
    match job {
        Job::PushNotification {
            user_id,
            notification_type,
            payload,
        } => {
            process_push_notification(context, &user_id, notification_type, payload).await;
        }
        Job::Webhook {
            user_id,
            event_type,
            payload,
        } => {
            process_webhook(context, &user_id, &event_type, payload).await;
        }
        Job::Cleanup { task } => {
            process_cleanup(context, task).await;
        }
    }
}

/// Process a push notification job.
async fn process_push_notification(
    context: &JobWorkerContext,
    user_id: &str,
    notification_type: PushNotificationType,
    payload: PushPayload,
) {
    let Some(ref push_service) = context.push_service else {
        debug!("Push service not available, skipping notification");
        return;
    };

    match push_service
        .send_to_user(user_id, notification_type, payload)
        .await
    {
        Ok(count) => {
            debug!(
                user_id = %user_id,
                notification_type = %notification_type,
                success_count = %count,
                "Push notifications sent"
            );
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                notification_type = %notification_type,
                error = %e,
                "Failed to send push notifications"
            );
        }
    }
}

/// Process a webhook job.
async fn process_webhook(
    context: &JobWorkerContext,
    user_id: &str,
    event_type: &str,
    payload: serde_json::Value,
) {
    let Some(ref webhook_service) = context.webhook_service else {
        debug!("Webhook service not available, skipping delivery");
        return;
    };

    match webhook_service.trigger(user_id, event_type, payload).await {
        Ok(()) => {
            debug!(
                user_id = %user_id,
                event_type = %event_type,
                "Webhooks triggered"
            );
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                event_type = %event_type,
                error = %e,
                "Failed to deliver webhooks"
            );
        }
    }
}

/// Process a cleanup job.
async fn process_cleanup(_context: &JobWorkerContext, task: CleanupTask) {
    match task {
        CleanupTask::ExpiredWordFilters => {
            debug!("Cleanup: expired word filters");
            // TODO: Implement cleanup
        }
        CleanupTask::ExpiredPushSubscriptions => {
            debug!("Cleanup: expired push subscriptions");
            // TODO: Implement cleanup
        }
        CleanupTask::ExpiredSessions => {
            debug!("Cleanup: expired sessions");
            // TODO: Implement cleanup
        }
        CleanupTask::OldNotifications => {
            debug!("Cleanup: old notifications");
            // TODO: Implement cleanup
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_sender_enqueue() {
        let service = JobService::new();
        let sender = service.sender();

        // Start with no services
        service.start(JobWorkerContext {
            push_service: None,
            webhook_service: None,
        });

        // Should be able to enqueue a job
        let result = sender.cleanup(CleanupTask::ExpiredWordFilters).await;

        assert!(result.is_ok());
    }
}
