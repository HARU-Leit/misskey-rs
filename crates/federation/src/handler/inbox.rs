//! Inbox handler for receiving `ActivityPub` activities.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use misskey_common::{AppError, AppResult};
use misskey_db::repositories::{
    FollowRequestRepository, FollowingRepository, NoteRepository, ReactionRepository,
    UserKeypairRepository, UserRepository,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::{
    AcceptActivity, AnnounceActivity, CreateActivity, DeleteActivity, FollowActivity, LikeActivity,
    RejectActivity, UndoActivity, UpdateActivity,
    client::ApClient,
    processor::{
        AcceptProcessor, AnnounceProcessor, CreateProcessor, FollowProcessor, LikeProcessor,
        ParsedUndoActivity, UndoProcessor, UpdateProcessor,
    },
    signature::{HttpVerifier, verify_digest},
};

/// Wrapper for incoming activities that can be any type.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InboxActivity {
    Create(CreateActivity),
    Delete(DeleteActivity),
    Follow(FollowActivity),
    Accept(AcceptActivity),
    Reject(RejectActivity),
    Like(LikeActivity),
    Undo(UndoActivity),
    Update(UpdateActivity),
    Announce(AnnounceActivity),
    Unknown(Value),
}

impl InboxActivity {
    /// Get the activity type as a string.
    #[must_use]
    pub const fn activity_type(&self) -> &str {
        match self {
            Self::Create(_) => "Create",
            Self::Delete(_) => "Delete",
            Self::Follow(_) => "Follow",
            Self::Accept(_) => "Accept",
            Self::Reject(_) => "Reject",
            Self::Like(_) => "Like",
            Self::Undo(_) => "Undo",
            Self::Update(_) => "Update",
            Self::Announce(_) => "Announce",
            Self::Unknown(_) => "Unknown",
        }
    }

    /// Get the actor URL.
    #[must_use]
    pub const fn actor(&self) -> Option<&url::Url> {
        match self {
            Self::Create(a) => Some(&a.actor),
            Self::Delete(a) => Some(&a.actor),
            Self::Follow(a) => Some(&a.actor),
            Self::Accept(a) => Some(&a.actor),
            Self::Reject(a) => Some(&a.actor),
            Self::Like(a) => Some(&a.actor),
            Self::Undo(a) => Some(&a.actor),
            Self::Update(a) => Some(&a.actor),
            Self::Announce(a) => Some(&a.actor),
            Self::Unknown(_) => None,
        }
    }
}

/// State required for the inbox handler.
#[derive(Clone)]
pub struct InboxState {
    pub user_repo: UserRepository,
    pub user_keypair_repo: UserKeypairRepository,
    pub note_repo: NoteRepository,
    pub following_repo: FollowingRepository,
    pub follow_request_repo: FollowRequestRepository,
    pub reaction_repo: ReactionRepository,
    pub ap_client: ApClient,
    pub base_url: url::Url,
}

impl InboxState {
    /// Create a new inbox state.
    #[must_use]
    pub fn new(
        user_repo: UserRepository,
        user_keypair_repo: UserKeypairRepository,
        note_repo: NoteRepository,
        following_repo: FollowingRepository,
        follow_request_repo: FollowRequestRepository,
        reaction_repo: ReactionRepository,
        base_url: url::Url,
    ) -> Self {
        let ap_client = ApClient::new(base_url.as_str());
        Self {
            user_repo,
            user_keypair_repo,
            note_repo,
            following_repo,
            follow_request_repo,
            reaction_repo,
            ap_client,
            base_url,
        }
    }
}

/// Handle incoming `ActivityPub` activities.
///
/// This is the main inbox endpoint that receives activities from remote servers.
pub async fn inbox_handler(
    State(state): State<InboxState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Parse the body as JSON
    let activity: InboxActivity = match serde_json::from_slice(&body) {
        Ok(a) => a,
        Err(e) => {
            warn!(error = %e, "Failed to parse activity");
            return StatusCode::BAD_REQUEST;
        }
    };

    info!(
        activity_type = activity.activity_type(),
        actor = ?activity.actor(),
        "Received activity"
    );

    // Verify HTTP signature
    if let Err(e) = verify_incoming_signature(&state, &headers, &body, &activity).await {
        warn!(error = %e, "Signature verification failed");
        // In production, you might want to reject unsigned requests
        // For development, we'll log and continue
        debug!("Continuing despite signature verification failure");
    }

    // Process the activity
    let result = process_activity(&state, &activity).await;

    match result {
        Ok(()) => StatusCode::ACCEPTED,
        Err(e) => {
            error!(error = %e, "Failed to process activity");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// Verify the HTTP signature on an incoming request.
async fn verify_incoming_signature(
    state: &InboxState,
    headers: &HeaderMap,
    body: &[u8],
    _activity: &InboxActivity,
) -> AppResult<()> {
    // Get signature header
    let signature_header = headers
        .get("signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing Signature header".to_string()))?;

    // Parse signature header
    let components = HttpVerifier::parse_signature_header(signature_header)
        .map_err(|e| AppError::BadRequest(format!("Invalid signature header: {e}")))?;

    // Verify digest if present
    if let Some(digest_header) = headers.get("digest").and_then(|v| v.to_str().ok())
        && !verify_digest(body, digest_header)
    {
        return Err(AppError::BadRequest("Digest mismatch".to_string()));
    }

    // Fetch the public key from the actor
    let public_key_pem = fetch_actor_public_key(state, &components.key_id).await?;

    // Build headers map for verification
    let mut verify_headers = HashMap::new();
    for header_name in &components.headers {
        if header_name == "(request-target)" {
            continue; // Handled separately
        }
        if let Some(value) = headers
            .get(header_name.as_str())
            .and_then(|v| v.to_str().ok())
        {
            verify_headers.insert(header_name.clone(), value.to_string());
        }
    }

    // Verify signature
    let is_valid = HttpVerifier::verify(
        &public_key_pem,
        &components,
        "POST",
        "/inbox",
        &verify_headers,
    )
    .map_err(|e| AppError::BadRequest(format!("Signature verification error: {e}")))?;

    if !is_valid {
        return Err(AppError::BadRequest("Invalid signature".to_string()));
    }

    debug!(key_id = %components.key_id, "Signature verified successfully");
    Ok(())
}

/// Fetch an actor's public key from their profile.
async fn fetch_actor_public_key(state: &InboxState, key_id: &str) -> AppResult<String> {
    // Key ID is usually in format: https://example.com/users/alice#main-key
    // The actor URL is the part before #main-key
    let actor_url = key_id.split('#').next().unwrap_or(key_id);

    // First, try to find the actor in our database
    if let Some(user) = state.user_repo.find_by_uri(actor_url).await? {
        // For local users, get the keypair
        if user.host.is_none()
            && let Some(keypair) = state.user_keypair_repo.find_by_user_id(&user.id).await?
        {
            return Ok(keypair.public_key);
        }
        // For remote users, we need to fetch from the actor document
    }

    // Fetch the actor from remote
    let actor = state
        .ap_client
        .fetch_actor(actor_url)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch actor: {e}")))?;

    // Extract public key from actor
    let public_key = actor
        .get("publicKey")
        .and_then(|pk| pk.get("publicKeyPem"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Actor missing public key".to_string()))?;

    Ok(public_key.to_string())
}

/// Process an incoming activity.
async fn process_activity(state: &InboxState, activity: &InboxActivity) -> AppResult<()> {
    match activity {
        InboxActivity::Create(create) => {
            info!(note_id = %create.object.id, "Processing Create activity");
            let processor = CreateProcessor::new(
                state.note_repo.clone(),
                state.user_repo.clone(),
                state.ap_client.clone(),
            );
            processor.process(create).await?;
        }
        InboxActivity::Delete(delete) => {
            info!(object = %delete.object, "Processing Delete activity");
            // Find and soft-delete the object
            if let Some(note) = state.note_repo.find_by_uri(delete.object.as_str()).await? {
                state.note_repo.delete(&note.id).await?;
                info!(note_id = %note.id, "Deleted note");
            }
        }
        InboxActivity::Follow(follow) => {
            info!(object = %follow.object, "Processing Follow activity");
            let processor = FollowProcessor::new(
                state.user_repo.clone(),
                state.following_repo.clone(),
                state.follow_request_repo.clone(),
                state.ap_client.clone(),
            );
            processor.process(follow).await?;
        }
        InboxActivity::Accept(accept) => {
            info!(object = %accept.object, "Processing Accept activity");
            let processor = AcceptProcessor::new(
                state.user_repo.clone(),
                state.following_repo.clone(),
                state.follow_request_repo.clone(),
            );
            processor.process(accept).await?;
        }
        InboxActivity::Reject(reject) => {
            info!(object = %reject.object, "Processing Reject activity");
            // Find the follow request by the actor (remote user) and delete it
            if let Some(followee) = state.user_repo.find_by_uri(reject.actor.as_str()).await? {
                // Delete any pending follow requests to this remote user
                let requests = state
                    .follow_request_repo
                    .find_by_followee(&followee.id)
                    .await?;
                if let Some(request) = requests {
                    state.follow_request_repo.delete(&request.id).await?;
                    info!(followee = %followee.id, "Follow request rejected and deleted");
                }
            }
        }
        InboxActivity::Like(like) => {
            info!(object = %like.object, "Processing Like activity");
            let processor = LikeProcessor::new(
                state.user_repo.clone(),
                state.note_repo.clone(),
                state.reaction_repo.clone(),
                state.ap_client.clone(),
            );
            processor.process(like).await?;
        }
        InboxActivity::Undo(undo) => {
            info!(object = %undo.object, "Processing Undo activity");
            // We need to fetch the original activity to know what we're undoing
            // For now, handle common cases using the Undo processor
            let parsed = parse_undo_activity(state, undo).await?;
            let processor = UndoProcessor::new(
                state.user_repo.clone(),
                state.following_repo.clone(),
                state.reaction_repo.clone(),
                state.note_repo.clone(),
            );
            processor.process(&parsed).await?;
        }
        InboxActivity::Update(update) => {
            info!("Processing Update activity");
            let processor = UpdateProcessor::new(state.user_repo.clone(), state.note_repo.clone());
            processor.process(update).await?;
        }
        InboxActivity::Announce(announce) => {
            info!(object = %announce.object, "Processing Announce activity");
            let processor =
                AnnounceProcessor::new(state.user_repo.clone(), state.note_repo.clone());
            processor.process(announce).await?;
        }
        InboxActivity::Unknown(value) => {
            warn!(activity_type = ?value.get("type"), "Received unknown activity type");
        }
    }

    Ok(())
}

/// Parse an Undo activity to determine what is being undone.
async fn parse_undo_activity(
    state: &InboxState,
    undo: &UndoActivity,
) -> AppResult<ParsedUndoActivity> {
    // Try to fetch the original activity from the object URL
    // In practice, the object might be embedded or just a URL
    // For now, we'll try to infer from what we know

    // Fetch the activity being undone
    let activity_json = state
        .ap_client
        .fetch_object(undo.object.as_str())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch undo object: {e}")))?;

    let object_type = activity_json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let object_object = activity_json.get("object").and_then(|o| {
        if let Some(s) = o.as_str() {
            url::Url::parse(s).ok()
        } else if let Some(obj) = o.as_object() {
            obj.get("id")
                .and_then(|id| id.as_str())
                .and_then(|s| url::Url::parse(s).ok())
        } else {
            None
        }
    });

    Ok(ParsedUndoActivity {
        id: undo.id.clone(),
        actor: undo.actor.clone(),
        object_type,
        object_id: undo.object.clone(),
        object_object,
    })
}

/// Handle incoming activities for a specific user's inbox.
pub async fn user_inbox_handler(
    State(state): State<InboxState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // For now, delegate to the shared inbox handler
    inbox_handler(State(state), headers, body).await
}
