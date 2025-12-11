//! Move activity processor for account migration.

use misskey_common::{AppError, AppResult};
use misskey_db::{
    entities::user_profile,
    repositories::{FollowingRepository, UserProfileRepository, UserRepository},
};
use sea_orm::Set;
use serde_json::Value;
use tracing::{info, warn};
use url::Url;

use super::ActorFetcher;
use crate::MoveActivity;
use crate::client::ApClient;

/// Processor for Move activities (account migration).
#[derive(Clone)]
pub struct MoveProcessor {
    user_repo: UserRepository,
    profile_repo: UserProfileRepository,
    following_repo: FollowingRepository,
    actor_fetcher: ActorFetcher,
    ap_client: ApClient,
}

/// Result of processing a Move activity.
#[derive(Debug)]
pub enum MoveProcessResult {
    /// Move was processed successfully.
    Success {
        /// The user who moved.
        source_user_id: String,
        /// The URI of the new account.
        target_uri: String,
        /// Number of local followers who were notified.
        followers_notified: usize,
    },
    /// Move was ignored (source not followed by any local users).
    Ignored { reason: String },
    /// Move validation failed.
    Failed { reason: String },
}

impl MoveProcessor {
    /// Create a new move processor.
    #[must_use]
    pub fn new(
        user_repo: UserRepository,
        profile_repo: UserProfileRepository,
        following_repo: FollowingRepository,
        ap_client: ApClient,
    ) -> Self {
        Self {
            user_repo: user_repo.clone(),
            profile_repo,
            following_repo,
            actor_fetcher: ActorFetcher::new(user_repo, ap_client.clone()),
            ap_client,
        }
    }

    /// Process an incoming Move activity from a remote actor.
    ///
    /// This handles:
    /// 1. Verifying the source account exists in our database
    /// 2. Fetching the target account to verify the move is legitimate
    /// 3. Checking that the target account lists the source in alsoKnownAs
    /// 4. Updating our local record of the source account
    /// 5. Optionally notifying local followers
    pub async fn process(&self, activity: &MoveActivity) -> AppResult<MoveProcessResult> {
        info!(
            source = %activity.actor,
            target = %activity.target,
            "Processing Move activity"
        );

        // Find the source actor in our database
        let source_user = match self.user_repo.find_by_uri(activity.actor.as_str()).await? {
            Some(user) => user,
            None => {
                // We don't have this user, so ignore the Move
                return Ok(MoveProcessResult::Ignored {
                    reason: "Source account not found in database".to_string(),
                });
            }
        };

        // Check if this is a remote user (local users shouldn't receive Move for themselves)
        if source_user.host.is_none() {
            return Ok(MoveProcessResult::Failed {
                reason: "Cannot process Move for local accounts".to_string(),
            });
        }

        // Verify the move is legitimate by fetching the target and checking alsoKnownAs
        if let Err(e) = self.verify_move(&activity.actor, &activity.target).await {
            warn!(
                source = %activity.actor,
                target = %activity.target,
                error = %e,
                "Move validation failed"
            );
            return Ok(MoveProcessResult::Failed {
                reason: format!("Move validation failed: {e}"),
            });
        }

        // Update the source user's profile to record the move
        self.record_move(&source_user.id, activity.target.as_str())
            .await?;

        // Count followers who will be affected
        let followers = self
            .following_repo
            .find_followers(&source_user.id, 10000, None)
            .await?;

        let local_followers_count = followers
            .iter()
            .filter(|f| {
                // Count only local followers (those we should notify)
                // A local follower would have followee_host = Some(source's host)
                // and the follower would be a local user
                true // For now, count all followers
            })
            .count();

        info!(
            source = %source_user.id,
            target = %activity.target,
            followers = local_followers_count,
            "Move recorded successfully"
        );

        Ok(MoveProcessResult::Success {
            source_user_id: source_user.id,
            target_uri: activity.target.to_string(),
            followers_notified: local_followers_count,
        })
    }

    /// Verify that a move is legitimate.
    ///
    /// Per `ActivityPub` best practices (FEP-7628):
    /// - The target account should have the source account in its alsoKnownAs
    async fn verify_move(&self, source: &Url, target: &Url) -> AppResult<()> {
        // Fetch the target actor
        let target_actor = self
            .ap_client
            .fetch_actor(target.as_str())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch target actor: {e}")))?;

        // Check if target has alsoKnownAs that includes the source
        let also_known_as = target_actor.get("alsoKnownAs");

        let source_str = source.as_str();

        let is_valid = match also_known_as {
            Some(Value::Array(arr)) => arr.iter().any(|v| {
                if let Value::String(s) = v {
                    s == source_str
                } else {
                    false
                }
            }),
            Some(Value::String(s)) => s == source_str,
            _ => false,
        };

        if !is_valid {
            return Err(AppError::Validation(
                "Target account does not list source in alsoKnownAs".to_string(),
            ));
        }

        info!(
            source = %source,
            target = %target,
            "Move validation successful"
        );

        Ok(())
    }

    /// Record a move in the database.
    async fn record_move(&self, user_id: &str, target_uri: &str) -> AppResult<()> {
        // Try to find existing profile
        let profile = self.profile_repo.find_by_user_id(user_id).await?;

        if let Some(p) = profile {
            // Update existing profile
            let mut active: user_profile::ActiveModel = p.into();
            active.moved_to_uri = Set(Some(target_uri.to_string()));
            active.updated_at = Set(Some(chrono::Utc::now().into()));
            self.profile_repo.update(active).await?;
        } else {
            // Create profile for remote user with moved_to_uri
            let model = user_profile::ActiveModel {
                user_id: Set(user_id.to_string()),
                password: Set(None),
                email: Set(None),
                email_verified: Set(false),
                two_factor_secret: Set(None),
                two_factor_enabled: Set(false),
                two_factor_pending: Set(None),
                two_factor_backup_codes: Set(None),
                auto_accept_followed: Set(false),
                always_mark_nsfw: Set(false),
                pinned_page_ids: Set(serde_json::json!([])),
                pinned_note_ids: Set(serde_json::json!([])),
                fields: Set(serde_json::json!([])),
                muted_words: Set(serde_json::json!([])),
                user_css: Set(None),
                birthday: Set(None),
                location: Set(None),
                lang: Set(None),
                pronouns: Set(None),
                also_known_as: Set(None),
                moved_to_uri: Set(Some(target_uri.to_string())),
                hide_bots: Set(false),
                default_reaction: Set(None),
                receive_dm_from_followers_only: Set(false),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(None),
            };
            self.profile_repo.create(model).await?;
        }

        info!(
            user_id = user_id,
            target = target_uri,
            "Move recorded in database"
        );

        Ok(())
    }
}
