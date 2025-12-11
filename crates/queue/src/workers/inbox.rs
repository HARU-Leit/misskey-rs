//! Inbox worker.

use std::sync::Arc;

use apalis::prelude::*;
use misskey_core::services::delivery::DeliveryService;
use misskey_db::repositories::{
    DriveFileRepository, FollowRequestRepository, FollowingRepository, NoteRepository,
    NotificationRepository, ReactionRepository, UserRepository,
};
use misskey_federation::{
    AcceptActivity, AcceptProcessor, AnnounceActivity, AnnounceProcessor, CreateActivity,
    CreateProcessor, DeleteActivity, DeleteProcessor, FollowActivity, FollowProcessResult,
    FollowProcessor, HttpVerifier, LikeActivity, LikeProcessor, ParsedUndoActivity,
    RejectActivity, RejectProcessor, UndoProcessor, UpdateActivity, UpdateProcessor,
    client::ApClient,
};
use sea_orm::DatabaseConnection;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::jobs::InboxJob;

/// Context for the inbox worker with all required processors.
#[derive(Clone)]
pub struct InboxWorkerContext {
    pub db: Arc<DatabaseConnection>,
    pub ap_client: ApClient,
    pub base_url: Url,
    pub delivery: Option<DeliveryService>,
    /// Whether to require HTTP signature verification.
    pub require_signatures: bool,
}

impl InboxWorkerContext {
    /// Create a new inbox worker context.
    ///
    /// # Panics
    /// Panics if `base_url` is not a valid URL.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn new(db: Arc<DatabaseConnection>, base_url: &str) -> Self {
        Self {
            db,
            ap_client: ApClient::new(base_url),
            base_url: Url::parse(base_url).expect("Invalid base_url"),
            delivery: None,
            require_signatures: true,
        }
    }

    /// Set the delivery service.
    #[must_use]
    pub fn with_delivery(mut self, delivery: DeliveryService) -> Self {
        self.delivery = Some(delivery);
        self
    }

    /// Set whether to require HTTP signature verification.
    #[must_use]
    pub const fn with_signature_requirement(mut self, require: bool) -> Self {
        self.require_signatures = require;
        self
    }

    fn user_repo(&self) -> UserRepository {
        UserRepository::new(Arc::clone(&self.db))
    }

    fn note_repo(&self) -> NoteRepository {
        NoteRepository::new(Arc::clone(&self.db))
    }

    fn reaction_repo(&self) -> ReactionRepository {
        ReactionRepository::new(Arc::clone(&self.db))
    }

    fn following_repo(&self) -> FollowingRepository {
        FollowingRepository::new(Arc::clone(&self.db))
    }

    fn follow_request_repo(&self) -> FollowRequestRepository {
        FollowRequestRepository::new(Arc::clone(&self.db))
    }

    fn ap_client(&self) -> ApClient {
        self.ap_client.clone()
    }

    fn drive_file_repo(&self) -> DriveFileRepository {
        DriveFileRepository::new(Arc::clone(&self.db))
    }

    fn notification_repo(&self) -> NotificationRepository {
        NotificationRepository::new(Arc::clone(&self.db))
    }
}

/// Worker function for processing incoming activities.
///
/// # Errors
/// Returns an error if the activity processing fails.
pub async fn inbox_worker(job: InboxJob, ctx: Data<InboxWorkerContext>) -> Result<(), Error> {
    info!("Processing incoming activity");

    // Verify HTTP signature before processing
    if ctx.require_signatures {
        match verify_signature(&job, &ctx).await {
            Ok(actor_url) => {
                debug!(actor = ?actor_url, "HTTP signature verified successfully");
            }
            Err(e) => {
                warn!(error = %e, "HTTP signature verification failed");
                return Err(Error::Failed(e.into()));
            }
        }
    } else {
        debug!("Signature verification disabled, processing activity directly");
    }

    match process_activity(&job, &ctx).await {
        Ok(()) => {
            info!("Activity processed successfully");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Failed to process activity");
            Err(Error::Failed(e.into()))
        }
    }
}

/// Verify HTTP signature on an incoming activity.
///
/// Returns the actor URL if verification succeeds.
async fn verify_signature(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    // Parse signature header
    let components = HttpVerifier::parse_signature_header(&job.signature)?;

    // Extract actor URL from key_id
    let actor_url = components
        .key_id
        .split('#')
        .next()
        .map(String::from);

    let Some(ref actor_url_str) = actor_url else {
        return Err("Invalid key_id format in signature".into());
    };

    // Fetch actor's public key
    let actor_json = ctx
        .ap_client
        .fetch_actor(actor_url_str)
        .await
        .map_err(|e| format!("Failed to fetch actor: {e}"))?;

    let public_key = actor_json
        .get("publicKey")
        .ok_or("Actor missing publicKey")?;

    let public_key_pem = public_key
        .get("publicKeyPem")
        .and_then(|v| v.as_str())
        .ok_or("Actor missing publicKeyPem")?;

    // Verify the signature
    let verified = HttpVerifier::verify(
        public_key_pem,
        &components,
        &job.method,
        &job.path,
        &job.headers,
    )?;

    if verified {
        Ok(actor_url)
    } else {
        Err("HTTP signature verification failed".into())
    }
}

async fn process_activity(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get activity type
    let activity_type = job.activity.get("type").and_then(|t| t.as_str());

    match activity_type {
        Some("Create") => {
            info!("Processing Create activity");
            process_create(job, ctx).await?;
        }
        Some("Delete") => {
            info!("Processing Delete activity");
            process_delete(job, ctx).await?;
        }
        Some("Follow") => {
            info!("Processing Follow activity");
            process_follow(job, ctx).await?;
        }
        Some("Accept") => {
            info!("Processing Accept activity");
            process_accept(job, ctx).await?;
        }
        Some("Reject") => {
            info!("Processing Reject activity");
            process_reject(job, ctx).await?;
        }
        Some("Like") => {
            info!("Processing Like activity");
            process_like(job, ctx).await?;
        }
        Some("Announce") => {
            info!("Processing Announce activity");
            process_announce(job, ctx).await?;
        }
        Some("Undo") => {
            info!("Processing Undo activity");
            process_undo(job, ctx).await?;
        }
        Some("Update") => {
            info!("Processing Update activity");
            process_update(job, ctx).await?;
        }
        Some(t) => {
            warn!(activity_type = t, "Unknown activity type");
        }
        None => {
            warn!("Activity has no type");
        }
    }

    Ok(())
}

async fn process_create(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let activity: CreateActivity = serde_json::from_value(job.activity.clone())?;
    let processor = CreateProcessor::new(
        ctx.note_repo(),
        ctx.drive_file_repo(),
        ctx.user_repo(),
        ctx.ap_client(),
    );
    processor.process(&activity).await?;
    Ok(())
}

async fn process_delete(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let activity: DeleteActivity = serde_json::from_value(job.activity.clone())?;
    let processor = DeleteProcessor::new(ctx.user_repo(), ctx.note_repo());
    let result = processor.process(&activity).await?;
    info!(?result, "Delete activity processed");
    Ok(())
}

async fn process_follow(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use misskey_core::services::NotificationService;

    let activity: FollowActivity = serde_json::from_value(job.activity.clone())?;
    let processor = FollowProcessor::with_base_url(
        ctx.user_repo(),
        ctx.following_repo(),
        ctx.follow_request_repo(),
        ctx.ap_client(),
        ctx.base_url.clone(),
    );
    let result = processor.process(&activity).await?;
    info!(?result, "Follow activity processed");

    // Handle notifications and Accept activity queueing based on result
    match result {
        FollowProcessResult::Accepted {
            followee_id,
            follower_id,
            accept_activity,
        } => {
            // Create follow notification for the followee
            let notification_service = NotificationService::new(ctx.notification_repo());
            if let Err(e) = notification_service
                .create_follow_notification(&followee_id, &follower_id)
                .await
            {
                warn!(error = %e, "Failed to create follow notification");
            }

            // Queue Accept activity to send back to the follower
            if let Some(accept_info) = accept_activity
                && let Some(ref delivery) = ctx.delivery
                && let Err(e) = delivery
                    .queue_accept_follow(
                        &accept_info.accepter_id,
                        &accept_info.inbox_url,
                        accept_info.activity,
                    )
                    .await
            {
                warn!(error = %e, "Failed to queue Accept activity");
            }
        }
        FollowProcessResult::Pending {
            followee_id,
            follower_id,
        } => {
            // Create follow request notification for the followee
            let notification_service = NotificationService::new(ctx.notification_repo());
            if let Err(e) = notification_service
                .create_follow_request_notification(&followee_id, &follower_id)
                .await
            {
                warn!(error = %e, "Failed to create follow request notification");
            }
        }
        FollowProcessResult::Rejected { reason } => {
            info!(reason = %reason, "Follow was rejected");
        }
    }

    Ok(())
}

async fn process_accept(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let activity: AcceptActivity = serde_json::from_value(job.activity.clone())?;
    let processor = AcceptProcessor::new(
        ctx.user_repo(),
        ctx.following_repo(),
        ctx.follow_request_repo(),
    );
    processor.process(&activity).await?;
    Ok(())
}

async fn process_reject(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let activity: RejectActivity = serde_json::from_value(job.activity.clone())?;
    let processor = RejectProcessor::new(ctx.user_repo(), ctx.follow_request_repo());
    processor.process(&activity).await?;
    Ok(())
}

async fn process_like(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let activity: LikeActivity = serde_json::from_value(job.activity.clone())?;
    let processor = LikeProcessor::new(
        ctx.user_repo(),
        ctx.note_repo(),
        ctx.reaction_repo(),
        ctx.ap_client(),
    );
    processor.process(&activity).await?;
    Ok(())
}

async fn process_announce(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let activity: AnnounceActivity = serde_json::from_value(job.activity.clone())?;
    let processor = AnnounceProcessor::new(ctx.user_repo(), ctx.note_repo(), ctx.ap_client());
    processor.process(&activity).await?;
    Ok(())
}

#[allow(clippy::unwrap_used)] // Static URL "https://invalid" is always valid
async fn process_undo(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Undo requires parsing the inner object to determine what to undo
    let id = job
        .activity
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?;
    let actor = job
        .activity
        .get("actor")
        .and_then(|v| v.as_str())
        .ok_or("Missing actor")?;
    let object = job.activity.get("object").ok_or("Missing object")?;

    // Object can be a URL string or an embedded object
    let (object_type, object_id, object_object) = if object.as_str().is_some() {
        // Just a URL reference, we'd need to look up what it refers to
        // For now, we can't handle this case well
        warn!("Undo with URL reference not fully supported");
        return Ok(());
    } else if let Some(obj_map) = object.as_object() {
        let obj_type = obj_map
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let obj_id = obj_map.get("id").and_then(|v| v.as_str()).unwrap_or("");
        // For Follow, the object.object is the followee
        // For Like, the object.object is the note
        let obj_obj = obj_map
            .get("object")
            .and_then(|v| v.as_str())
            .and_then(|s| Url::parse(s).ok());
        (obj_type.to_string(), obj_id.to_string(), obj_obj)
    } else {
        warn!("Invalid Undo object format");
        return Ok(());
    };

    let parsed = ParsedUndoActivity {
        id: Url::parse(id)?,
        actor: Url::parse(actor)?,
        object_type,
        object_id: Url::parse(&object_id)
            .unwrap_or_else(|_| Url::parse("https://invalid").unwrap()),
        object_object,
    };

    let processor = UndoProcessor::new(
        ctx.user_repo(),
        ctx.following_repo(),
        ctx.reaction_repo(),
        ctx.note_repo(),
    );
    let result = processor.process(&parsed).await?;
    info!(?result, "Undo activity processed");
    Ok(())
}

async fn process_update(
    job: &InboxJob,
    ctx: &InboxWorkerContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let activity: UpdateActivity = serde_json::from_value(job.activity.clone())?;
    let processor = UpdateProcessor::new(ctx.user_repo(), ctx.note_repo());
    let result = processor.process(&activity).await?;
    info!(?result, "Update activity processed");
    Ok(())
}
