//! Inbox worker.

use std::sync::Arc;

use apalis::prelude::*;
use misskey_db::repositories::{
    FollowRequestRepository, FollowingRepository, NoteRepository, ReactionRepository,
    UserRepository,
};
use misskey_federation::{
    AcceptActivity, AcceptProcessor, AnnounceActivity, AnnounceProcessor, CreateActivity,
    CreateProcessor, DeleteActivity, DeleteProcessor, FollowActivity, FollowProcessor,
    LikeActivity, LikeProcessor, ParsedUndoActivity, RejectActivity, RejectProcessor,
    UndoProcessor, UpdateActivity, UpdateProcessor, client::ApClient,
};
use sea_orm::DatabaseConnection;
use tracing::{error, info, warn};
use url::Url;

use crate::jobs::InboxJob;

/// Context for the inbox worker with all required processors.
#[derive(Clone)]
pub struct InboxWorkerContext {
    pub db: Arc<DatabaseConnection>,
    pub ap_client: ApClient,
}

impl InboxWorkerContext {
    /// Create a new inbox worker context.
    #[must_use]
    pub fn new(db: Arc<DatabaseConnection>, base_url: &str) -> Self {
        Self {
            db,
            ap_client: ApClient::new(base_url),
        }
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
}

/// Worker function for processing incoming activities.
///
/// # Errors
/// Returns an error if the activity processing fails.
pub async fn inbox_worker(job: InboxJob, ctx: Data<InboxWorkerContext>) -> Result<(), Error> {
    info!("Processing incoming activity");

    // TODO: Verify HTTP signature before processing
    // For now, we'll process the activity directly

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
    let processor = CreateProcessor::new(ctx.note_repo(), ctx.user_repo(), ctx.ap_client());
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
    let activity: FollowActivity = serde_json::from_value(job.activity.clone())?;
    let processor = FollowProcessor::new(
        ctx.user_repo(),
        ctx.following_repo(),
        ctx.follow_request_repo(),
        ctx.ap_client(),
    );
    let result = processor.process(&activity).await?;
    info!(?result, "Follow activity processed");
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
    let processor = AnnounceProcessor::new(ctx.user_repo(), ctx.note_repo());
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
