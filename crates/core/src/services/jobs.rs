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
use misskey_db::repositories::{
    AccountDeletionRepository, ExportJobRepository, ImportJobRepository, NotificationRepository,
    PushSubscriptionRepository, UserRepository, WordFilterRepository,
};

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
    /// Process account deletion.
    AccountDeletion {
        deletion_id: String,
        user_id: String,
        hard_delete: bool,
    },
    /// Process data export.
    Export { job_id: String, user_id: String },
    /// Process data import.
    Import { job_id: String, user_id: String },
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

    /// Enqueue an account deletion job.
    pub async fn account_deletion(
        &self,
        deletion_id: String,
        user_id: String,
        hard_delete: bool,
    ) -> Result<(), &'static str> {
        self.enqueue(Job::AccountDeletion {
            deletion_id,
            user_id,
            hard_delete,
        })
        .await
    }

    /// Enqueue an export job.
    pub async fn export(&self, job_id: String, user_id: String) -> Result<(), &'static str> {
        self.enqueue(Job::Export { job_id, user_id }).await
    }

    /// Enqueue an import job.
    pub async fn import(&self, job_id: String, user_id: String) -> Result<(), &'static str> {
        self.enqueue(Job::Import { job_id, user_id }).await
    }
}

/// Job worker context containing services needed for job processing.
pub struct JobWorkerContext {
    /// Push notification service.
    pub push_service: Option<PushNotificationService>,
    /// Webhook service.
    pub webhook_service: Option<WebhookService>,
    /// Word filter repository for cleanup.
    pub word_filter_repo: Option<WordFilterRepository>,
    /// Push subscription repository for cleanup.
    pub push_subscription_repo: Option<PushSubscriptionRepository>,
    /// Notification repository for cleanup.
    pub notification_repo: Option<NotificationRepository>,
    /// Account deletion repository for deletion jobs.
    pub deletion_repo: Option<AccountDeletionRepository>,
    /// User repository for account operations.
    pub user_repo: Option<UserRepository>,
    /// Export job repository.
    pub export_job_repo: Option<ExportJobRepository>,
    /// Import job repository.
    pub import_job_repo: Option<ImportJobRepository>,
}

impl Clone for JobWorkerContext {
    fn clone(&self) -> Self {
        Self {
            push_service: self.push_service.clone(),
            webhook_service: self.webhook_service.clone(),
            word_filter_repo: self.word_filter_repo.clone(),
            push_subscription_repo: self.push_subscription_repo.clone(),
            notification_repo: self.notification_repo.clone(),
            deletion_repo: self.deletion_repo.clone(),
            user_repo: self.user_repo.clone(),
            export_job_repo: self.export_job_repo.clone(),
            import_job_repo: self.import_job_repo.clone(),
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
    #[allow(clippy::expect_used)] // Panic is intentional - start should only be called once
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
        Job::AccountDeletion {
            deletion_id,
            user_id,
            hard_delete,
        } => {
            process_account_deletion(context, &deletion_id, &user_id, hard_delete).await;
        }
        Job::Export { job_id, user_id } => {
            process_export(context, &job_id, &user_id).await;
        }
        Job::Import { job_id, user_id } => {
            process_import(context, &job_id, &user_id).await;
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

/// Retention days for old read notifications.
const NOTIFICATION_RETENTION_DAYS: i64 = 90;

/// Days to consider push subscription as stale.
const PUSH_SUBSCRIPTION_STALE_DAYS: i64 = 180;

/// Minimum fail count before deleting inactive push subscriptions.
const PUSH_SUBSCRIPTION_MAX_FAIL_COUNT: i32 = 5;

/// Process a cleanup job.
async fn process_cleanup(context: &JobWorkerContext, task: CleanupTask) {
    match task {
        CleanupTask::ExpiredWordFilters => {
            debug!("Cleanup: expired word filters");
            if let Some(repo) = &context.word_filter_repo {
                match repo.delete_expired().await {
                    Ok(count) => {
                        if count > 0 {
                            info!(count, "Deleted expired word filters");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to cleanup expired word filters");
                    }
                }
            }
        }
        CleanupTask::ExpiredPushSubscriptions => {
            debug!("Cleanup: expired push subscriptions");
            if let Some(repo) = &context.push_subscription_repo {
                // Delete failed subscriptions
                match repo.delete_failed(PUSH_SUBSCRIPTION_MAX_FAIL_COUNT).await {
                    Ok(count) => {
                        if count > 0 {
                            info!(count, "Deleted failed push subscriptions");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to cleanup failed push subscriptions");
                    }
                }

                // Delete stale subscriptions
                match repo.delete_stale(PUSH_SUBSCRIPTION_STALE_DAYS).await {
                    Ok(count) => {
                        if count > 0 {
                            info!(count, "Deleted stale push subscriptions");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to cleanup stale push subscriptions");
                    }
                }
            }
        }
        CleanupTask::ExpiredSessions => {
            debug!("Cleanup: expired sessions");
            // Session cleanup is handled by OAuth token expiry
            // Sessions in this system are based on access tokens which have their own cleanup
            // via OAuthRepository::delete_expired_tokens
        }
        CleanupTask::OldNotifications => {
            debug!("Cleanup: old notifications");
            if let Some(repo) = &context.notification_repo {
                match repo.delete_old_read(NOTIFICATION_RETENTION_DAYS).await {
                    Ok(count) => {
                        if count > 0 {
                            info!(
                                count,
                                days = NOTIFICATION_RETENTION_DAYS,
                                "Deleted old read notifications"
                            );
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to cleanup old notifications");
                    }
                }
            }
        }
    }
}

/// Process account deletion job.
async fn process_account_deletion(
    context: &JobWorkerContext,
    deletion_id: &str,
    user_id: &str,
    hard_delete: bool,
) {
    let (Some(deletion_repo), Some(user_repo)) = (&context.deletion_repo, &context.user_repo)
    else {
        error!("Account deletion repositories not available");
        return;
    };

    info!(
        deletion_id = %deletion_id,
        user_id = %user_id,
        hard_delete = %hard_delete,
        "Processing account deletion"
    );

    // Mark deletion as in progress
    if let Err(e) = deletion_repo.mark_in_progress(deletion_id).await {
        error!(error = %e, "Failed to mark deletion as in progress");
        return;
    }

    // Perform deletion
    let result = if hard_delete {
        user_repo.mark_as_deleted(user_id).await
    } else {
        user_repo.anonymize(user_id).await
    };

    match result {
        Ok(()) => {
            // Mark deletion as completed
            if let Err(e) = deletion_repo.mark_completed(deletion_id).await {
                error!(error = %e, "Failed to mark deletion as completed");
            } else {
                info!(
                    deletion_id = %deletion_id,
                    user_id = %user_id,
                    "Account deletion completed"
                );
            }
        }
        Err(e) => {
            error!(
                deletion_id = %deletion_id,
                user_id = %user_id,
                error = %e,
                "Account deletion failed"
            );
        }
    }
}

/// Process export job.
async fn process_export(context: &JobWorkerContext, job_id: &str, user_id: &str) {
    let Some(export_repo) = &context.export_job_repo else {
        error!("Export job repository not available");
        return;
    };

    info!(job_id = %job_id, user_id = %user_id, "Processing export job");

    // Mark job as processing
    if let Err(e) = export_repo.mark_processing(job_id).await {
        error!(error = %e, "Failed to mark export as processing");
        return;
    }

    // Get the export job details
    let job = match export_repo.find_by_id(job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            error!(job_id = %job_id, "Export job not found");
            return;
        }
        Err(e) => {
            error!(error = %e, "Failed to get export job");
            return;
        }
    };

    // Process each data type and update progress
    // For now, we simulate the export process
    // In a full implementation, this would call the AccountService export methods
    let data_types: Vec<String> =
        serde_json::from_value(job.data_types.clone()).unwrap_or_default();
    let total = data_types.len();

    for (i, data_type) in data_types.iter().enumerate() {
        let progress = ((i + 1) * 100 / total.max(1)) as i32;
        debug!(
            job_id = %job_id,
            data_type = %data_type,
            progress = %progress,
            "Exporting data type"
        );

        if let Err(e) = export_repo.update_progress(job_id, progress).await {
            error!(error = %e, "Failed to update export progress");
        }
    }

    // Mark as completed
    // In a full implementation, this would set the download_url and expires_at
    if let Err(e) = export_repo.mark_completed(job_id, None, None).await {
        error!(error = %e, "Failed to mark export as completed");
    } else {
        info!(job_id = %job_id, user_id = %user_id, "Export job completed");
    }
}

/// Process import job.
async fn process_import(context: &JobWorkerContext, job_id: &str, user_id: &str) {
    let Some(import_repo) = &context.import_job_repo else {
        error!("Import job repository not available");
        return;
    };

    info!(job_id = %job_id, user_id = %user_id, "Processing import job");

    // Mark job as processing
    if let Err(e) = import_repo.mark_processing(job_id).await {
        error!(error = %e, "Failed to mark import as processing");
        return;
    }

    // Get the import job details
    let job = match import_repo.find_by_id(job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            error!(job_id = %job_id, "Import job not found");
            return;
        }
        Err(e) => {
            error!(error = %e, "Failed to get import job");
            return;
        }
    };

    // Process import based on data type
    // For now, we simulate the import process
    // In a full implementation, this would call the AccountService import methods
    let total_items = job.total_items as usize;
    let mut imported: i32 = 0;
    let skipped = 0;
    let failed = 0;

    // Simulate processing items
    for i in 0..total_items {
        let progress = ((i + 1) * 100 / total_items.max(1)) as i32;

        // Update progress periodically
        if (i % 10 == 0 || i == total_items - 1)
            && let Err(e) = import_repo.update_progress(job_id, progress).await
        {
            error!(error = %e, "Failed to update import progress");
        }

        // In real implementation, process each item here
        imported += 1;
    }

    // Mark as completed with results
    if let Err(e) = import_repo
        .mark_completed(job_id, imported, skipped, failed)
        .await
    {
        error!(error = %e, "Failed to mark import as completed");
    } else {
        info!(
            job_id = %job_id,
            user_id = %user_id,
            imported = %imported,
            skipped = %skipped,
            failed = %failed,
            "Import job completed"
        );
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
            word_filter_repo: None,
            push_subscription_repo: None,
            notification_repo: None,
            deletion_repo: None,
            user_repo: None,
            export_job_repo: None,
            import_job_repo: None,
        });

        // Should be able to enqueue a job
        let result = sender.cleanup(CleanupTask::ExpiredWordFilters).await;

        assert!(result.is_ok());
    }
}
