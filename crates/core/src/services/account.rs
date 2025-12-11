//! Account management service for migration, deletion, export, and import.

use chrono::{DateTime, Utc};
use misskey_common::{AppError, AppResult, Config};
use misskey_db::{
    entities::{
        account_deletion, export_job, follow_request, following, import_job, user, user_profile,
    },
    repositories::{
        AccountDeletionRepository, ExportJobRepository, FollowRequestRepository,
        FollowingRepository, ImportJobRepository, NoteRepository, UserKeypairRepository,
        UserProfileRepository, UserRepository,
    },
};
use sea_orm::Set;
use serde::{Deserialize, Serialize};

use crate::DeliveryService;

/// Account migration status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationStatus {
    /// Migration is pending
    Pending,
    /// Migration is in progress
    InProgress,
    /// Migration completed successfully
    Completed,
    /// Migration failed
    Failed,
    /// Migration was cancelled
    Cancelled,
}

/// Account migration record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRecord {
    /// Migration ID
    pub id: String,
    /// Source account ID
    pub source_account_id: String,
    /// Target account URI (can be remote)
    pub target_account_uri: String,
    /// Migration status
    pub status: MigrationStatus,
    /// When migration was initiated
    pub created_at: DateTime<Utc>,
    /// When migration completed or failed
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Account deletion status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeletionStatus {
    /// Deletion is scheduled
    Scheduled,
    /// Deletion is in progress
    InProgress,
    /// Deletion completed
    Completed,
    /// Deletion was cancelled
    Cancelled,
}

/// Account deletion record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionRecord {
    /// User ID being deleted
    pub user_id: String,
    /// Deletion status
    pub status: DeletionStatus,
    /// Scheduled deletion time
    pub scheduled_at: DateTime<Utc>,
    /// When deletion actually completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Reason for deletion (optional)
    pub reason: Option<String>,
}

/// Export format for account data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// `ActivityPub` Actor JSON
    ActivityPub,
    /// Misskey-specific JSON format
    #[default]
    Misskey,
    /// CSV format (for specific data types)
    Csv,
}

/// Types of data that can be exported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportDataType {
    /// User profile data
    Profile,
    /// Notes/posts
    Notes,
    /// Following list
    Following,
    /// Followers list (for reference only)
    Followers,
    /// Muting list
    Muting,
    /// Blocking list
    Blocking,
    /// Drive files
    DriveFiles,
    /// Favorites/bookmarks
    Favorites,
    /// User lists
    UserLists,
    /// Antennas
    Antennas,
    /// Clips
    Clips,
    /// Custom emojis (admin only)
    Emojis,
}

/// Export job status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportStatus {
    /// Export is queued
    Queued,
    /// Export is in progress
    InProgress,
    /// Export completed
    Completed,
    /// Export failed
    Failed,
}

/// Export job record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportJob {
    /// Job ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Data types to export
    pub data_types: Vec<ExportDataType>,
    /// Export format
    pub format: ExportFormat,
    /// Job status
    pub status: ExportStatus,
    /// Progress (0-100)
    pub progress: u8,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Download URL (when completed)
    pub download_url: Option<String>,
    /// Expiration time for download
    pub expires_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Import job status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    /// Import is queued
    Queued,
    /// Import is validating data
    Validating,
    /// Import is in progress
    InProgress,
    /// Import completed
    Completed,
    /// Import completed with some errors
    PartiallyCompleted,
    /// Import failed
    Failed,
}

/// Import job record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportJob {
    /// Job ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Data type being imported
    pub data_type: ExportDataType,
    /// Job status
    pub status: ImportStatus,
    /// Progress (0-100)
    pub progress: u8,
    /// Total items to import
    pub total_items: u32,
    /// Successfully imported items
    pub imported_items: u32,
    /// Skipped items (duplicates, etc.)
    pub skipped_items: u32,
    /// Failed items
    pub failed_items: u32,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
    /// Detailed errors for individual items
    pub item_errors: Vec<ImportItemError>,
}

/// Error for individual import item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItemError {
    /// Line number or item index
    pub index: u32,
    /// Item identifier (username, note ID, etc.)
    pub identifier: String,
    /// Error message
    pub error: String,
}

/// Exported profile data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedProfile {
    /// Username
    pub username: String,
    /// Display name
    pub name: Option<String>,
    /// Description/bio
    pub description: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Banner URL
    pub banner_url: Option<String>,
    /// Is bot account
    pub is_bot: bool,
    /// Is cat mode enabled
    pub is_cat: bool,
    /// Is locked account
    pub is_locked: bool,
    /// Profile fields
    pub fields: Vec<ProfileField>,
    /// Pinned note IDs
    pub pinned_notes: Vec<String>,
}

/// Profile field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileField {
    /// Field name
    pub name: String,
    /// Field value
    pub value: String,
}

/// Exported following/follower entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedFollow {
    /// User's acct (username@host or just username for local)
    pub acct: String,
    /// `ActivityPub` URI (if available)
    pub uri: Option<String>,
}

/// Exported note data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedNote {
    /// Note ID
    pub id: String,
    /// Note text content
    pub text: Option<String>,
    /// Content warning
    pub cw: Option<String>,
    /// Visibility level
    pub visibility: String,
    /// Reply target note ID
    pub reply_id: Option<String>,
    /// Renote target note ID
    pub renote_id: Option<String>,
    /// Attached file IDs
    pub file_ids: Vec<String>,
    /// Hashtags
    pub tags: Vec<String>,
    /// `ActivityPub` URI
    pub uri: Option<String>,
    /// Human-readable URL
    pub url: Option<String>,
    /// Created timestamp (ISO 8601)
    pub created_at: String,
    /// Updated timestamp (ISO 8601)
    pub updated_at: Option<String>,
}

/// Request for note export with options.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportNotesInput {
    /// Maximum number of notes to export (default: 10000)
    #[serde(default = "default_export_limit")]
    pub limit: u32,
    /// Export format (json or csv)
    #[serde(default)]
    pub format: ExportFormat,
}

const fn default_export_limit() -> u32 {
    10000
}

/// Input for initiating account migration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateAccountInput {
    /// Target account URI (e.g., `https://other.instance/@user`)
    pub target_uri: String,
    /// Also set alias on target (if same instance)
    pub set_alias: bool,
}

/// Input for scheduling account deletion.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAccountInput {
    /// Current password for verification
    pub password: String,
    /// Reason for deletion (optional)
    pub reason: Option<String>,
    /// Soft delete (hide) instead of hard delete
    pub soft_delete: bool,
}

/// Input for creating an export job.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExportInput {
    /// Data types to export
    pub data_types: Vec<ExportDataType>,
    /// Export format
    pub format: ExportFormat,
}

/// Input for creating an import job.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateImportInput {
    /// Data type to import
    pub data_type: ExportDataType,
    /// File content (base64 encoded for binary, or JSON string)
    pub data: String,
}

/// Account management service.
#[derive(Clone)]
pub struct AccountService {
    user_repo: UserRepository,
    profile_repo: UserProfileRepository,
    #[allow(dead_code)]
    keypair_repo: UserKeypairRepository,
    note_repo: NoteRepository,
    following_repo: FollowingRepository,
    follow_request_repo: FollowRequestRepository,
    export_job_repo: ExportJobRepository,
    import_job_repo: ImportJobRepository,
    deletion_repo: AccountDeletionRepository,
    delivery_service: DeliveryService,
    job_sender: Option<crate::services::jobs::JobSender>,
    server_url: String,
}

impl AccountService {
    /// Create a new account service.
    pub fn new(
        user_repo: UserRepository,
        profile_repo: UserProfileRepository,
        keypair_repo: UserKeypairRepository,
        note_repo: NoteRepository,
        following_repo: FollowingRepository,
        follow_request_repo: FollowRequestRepository,
        export_job_repo: ExportJobRepository,
        import_job_repo: ImportJobRepository,
        deletion_repo: AccountDeletionRepository,
        delivery_service: DeliveryService,
        config: &Config,
    ) -> Self {
        Self {
            user_repo,
            profile_repo,
            keypair_repo,
            note_repo,
            following_repo,
            follow_request_repo,
            export_job_repo,
            import_job_repo,
            deletion_repo,
            delivery_service,
            job_sender: None,
            server_url: config.server.url.clone(),
        }
    }

    /// Set the job sender for background job processing.
    #[must_use]
    pub fn with_job_sender(mut self, sender: crate::services::jobs::JobSender) -> Self {
        self.job_sender = Some(sender);
        self
    }

    // =====================
    // Account Migration
    // =====================

    /// Initiate account migration to another instance.
    pub async fn migrate_account(
        &self,
        user_id: &str,
        input: MigrateAccountInput,
    ) -> AppResult<MigrationRecord> {
        let user = self.user_repo.get_by_id(user_id).await?;

        // Validate that user is a local user
        if user.host.is_some() {
            return Err(AppError::BadRequest(
                "Can only migrate local accounts".to_string(),
            ));
        }

        // Validate target URI format
        if !input.target_uri.starts_with("https://") && !input.target_uri.starts_with("http://") {
            return Err(AppError::Validation(
                "Invalid target URI format".to_string(),
            ));
        }

        // Create migration record
        let migration_id = crate::generate_id();
        let now = Utc::now();

        let mut migration = MigrationRecord {
            id: migration_id.clone(),
            source_account_id: user_id.to_string(),
            target_account_uri: input.target_uri.clone(),
            status: MigrationStatus::InProgress,
            created_at: now,
            completed_at: None,
            error: None,
        };

        // Update user profile's moved_to_uri field
        let profile = self.profile_repo.find_by_user_id(user_id).await?;
        if let Some(p) = profile {
            let mut active: user_profile::ActiveModel = p.into();
            active.moved_to_uri = Set(Some(input.target_uri.clone()));
            active.updated_at = Set(Some(Utc::now().into()));
            self.profile_repo.update(active).await?;
        } else {
            // Create profile with moved_to_uri
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
                moved_to_uri: Set(Some(input.target_uri.clone())),
                hide_bots: Set(false),
                default_reaction: Set(None),
                receive_dm_from_followers_only: Set(false),
                secure_fetch_only: Set(false),
                created_at: Set(Utc::now().into()),
                updated_at: Set(None),
            };
            self.profile_repo.create(model).await?;
        }

        // Build Move activity
        let actor_url = format!("{}/users/{}", self.server_url, user.id);
        let activity_id = format!("{}/move/{}", actor_url, crate::generate_id());
        let followers_url = format!("{actor_url}/followers");

        let move_activity = serde_json::json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": activity_id,
            "type": "Move",
            "actor": actor_url,
            "object": actor_url,
            "target": input.target_uri,
            "to": ["https://www.w3.org/ns/activitystreams#Public"],
            "cc": [followers_url]
        });

        // Get follower inboxes to deliver to
        let followers = self.following_repo.get_followers(user_id, 10000, 0).await?;
        let mut inboxes = Vec::new();

        for follow in followers {
            if let Ok(follower) = self.user_repo.get_by_id(&follow.follower_id).await {
                // Only deliver to remote users
                if follower.host.is_some() {
                    // Prefer shared inbox if available
                    if let Some(ref shared_inbox) = follower.shared_inbox {
                        if !inboxes.contains(shared_inbox) {
                            inboxes.push(shared_inbox.clone());
                        }
                    } else if let Some(ref inbox) = follower.inbox
                        && !inboxes.contains(inbox)
                    {
                        inboxes.push(inbox.clone());
                    }
                }
            }
        }

        tracing::info!(
            user_id = user_id,
            target = input.target_uri,
            inbox_count = inboxes.len(),
            activity_id = %activity_id,
            "Account migration initiated, Move activity prepared"
        );

        // Queue the Move activity for delivery to all follower inboxes
        if !inboxes.is_empty() {
            self.delivery_service
                .queue_move(user_id, move_activity, inboxes)
                .await?;
        }

        migration.status = MigrationStatus::Completed;
        migration.completed_at = Some(Utc::now());

        Ok(migration)
    }

    /// Set account aliases (alsoKnownAs).
    pub async fn set_aliases(&self, user_id: &str, aliases: Vec<String>) -> AppResult<()> {
        // Verify user exists
        let _user = self.user_repo.get_by_id(user_id).await?;

        // Validate aliases are valid URIs
        for alias in &aliases {
            if !alias.starts_with("https://") && !alias.starts_with("http://") {
                return Err(AppError::Validation(format!("Invalid alias URI: {alias}")));
            }
        }

        // Get profile
        let profile = self
            .profile_repo
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User profile not found".to_string()))?;

        // Update also_known_as field
        let mut active: user_profile::ActiveModel = profile.into();
        active.also_known_as = Set(Some(serde_json::json!(aliases)));
        active.updated_at = Set(Some(Utc::now().into()));

        self.profile_repo.update(active).await?;

        tracing::info!(user_id = user_id, aliases = ?aliases, "Account aliases updated");

        Ok(())
    }

    /// Get account aliases.
    pub async fn get_aliases(&self, user_id: &str) -> AppResult<Vec<String>> {
        let profile = self
            .profile_repo
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User profile not found".to_string()))?;

        if let Some(aliases_json) = profile.also_known_as {
            let aliases: Vec<String> = serde_json::from_value(aliases_json)
                .map_err(|e| AppError::Internal(format!("Failed to parse aliases: {e}")))?;
            Ok(aliases)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get migration status for a user.
    pub async fn get_migration_status(&self, user_id: &str) -> AppResult<MigrationStatusResponse> {
        let profile = self
            .profile_repo
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User profile not found".to_string()))?;

        // Get aliases
        let aliases = if let Some(aliases_json) = profile.also_known_as {
            serde_json::from_value(aliases_json).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Check if there's a pending migration (moved_to_uri is set)
        let has_pending_migration = profile.moved_to_uri.is_some();
        let moved_to = profile.moved_to_uri.clone();

        // Build migration record if migration is pending
        let migration = if let Some(ref target_uri) = moved_to {
            Some(MigrationRecord {
                id: target_uri.clone(), // Use target URI as migration ID
                source_account_id: user_id.to_string(),
                target_account_uri: target_uri.clone(),
                status: MigrationStatus::Pending,
                created_at: profile.updated_at.map_or_else(
                    || profile.created_at.with_timezone(&Utc),
                    |t| t.with_timezone(&Utc),
                ),
                completed_at: None,
                error: None,
            })
        } else {
            None
        };

        Ok(MigrationStatusResponse {
            has_pending_migration,
            migration,
            aliases,
            moved_to,
        })
    }

    /// Cancel a pending migration.
    ///
    /// This clears the `moved_to_uri` on the user's profile, effectively
    /// cancelling the migration. The `migration_id` is validated to ensure
    /// it matches the user's current migration target.
    pub async fn cancel_migration(&self, user_id: &str, migration_id: &str) -> AppResult<()> {
        // Get the user to verify they exist and are a local user
        let user = self.user_repo.get_by_id(user_id).await?;
        if user.host.is_some() {
            return Err(AppError::BadRequest(
                "Can only cancel migration for local accounts".to_string(),
            ));
        }

        // Get current migration target from profile
        let current_migration = self.profile_repo.get_moved_to_uri(user_id).await?;

        // Verify the migration_id matches the current migration target
        // The migration_id should be the target URI
        match current_migration {
            Some(ref target_uri) if target_uri == migration_id => {
                // Clear the moved_to_uri to cancel migration
                self.profile_repo.set_moved_to_uri(user_id, None).await?;

                tracing::info!(
                    user_id = user_id,
                    migration_target = migration_id,
                    "Migration cancelled successfully"
                );

                Ok(())
            }
            Some(target_uri) => {
                // Migration ID doesn't match current target
                Err(AppError::BadRequest(format!(
                    "Migration ID mismatch: expected {target_uri}, got {migration_id}"
                )))
            }
            None => {
                // No migration in progress
                Err(AppError::NotFound("No pending migration found".to_string()))
            }
        }
    }

    // =====================
    // Account Deletion
    // =====================

    /// Schedule account for deletion.
    pub async fn schedule_deletion(
        &self,
        user_id: &str,
        input: DeleteAccountInput,
    ) -> AppResult<DeletionRecord> {
        let user = self.user_repo.get_by_id(user_id).await?;

        // Validate that user is a local user
        if user.host.is_some() {
            return Err(AppError::BadRequest(
                "Can only delete local accounts".to_string(),
            ));
        }

        // Verify password
        let profile = self
            .profile_repo
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User profile not found".to_string()))?;

        let password_hash = profile
            .password
            .ok_or_else(|| AppError::BadRequest("No password set".to_string()))?;

        if !verify_password(&input.password, &password_hash)? {
            return Err(AppError::Unauthorized);
        }

        // Schedule deletion (e.g., 30 days from now for soft delete)
        let now = Utc::now();
        let scheduled_at = if input.soft_delete {
            now + chrono::Duration::days(30)
        } else {
            now + chrono::Duration::days(7)
        };

        // 1. Store deletion record in database
        let deletion_id = crate::generate_id();
        let db_model = account_deletion::ActiveModel {
            id: Set(deletion_id.clone()),
            user_id: Set(user_id.to_string()),
            status: Set(account_deletion::DeletionStatus::Scheduled),
            reason: Set(input.reason.clone()),
            soft_delete: Set(input.soft_delete),
            scheduled_at: Set(scheduled_at.into()),
            completed_at: Set(None),
            created_at: Set(now.into()),
        };
        self.deletion_repo.create(db_model).await?;

        let deletion = DeletionRecord {
            user_id: user_id.to_string(),
            status: DeletionStatus::Scheduled,
            scheduled_at,
            completed_at: None,
            reason: input.reason,
        };

        // 2. Mark user as suspended/hidden if soft delete
        if input.soft_delete {
            let mut active: user::ActiveModel = user.into();
            active.is_suspended = Set(true);
            active.updated_at = Set(Some(Utc::now().into()));
            self.user_repo.update(active).await?;
        }

        // 3. Queue deletion job for scheduled time (immediately queued, will run at scheduled time)
        if let Some(ref job_sender) = self.job_sender
            && let Err(e) = job_sender
                .account_deletion(
                    deletion_id.clone(),
                    user_id.to_string(),
                    !input.soft_delete,
                )
                .await
        {
            tracing::warn!(error = %e, "Failed to queue deletion job");
        }

        tracing::info!(
            user_id = user_id,
            deletion_id = deletion_id,
            scheduled_at = %scheduled_at,
            soft_delete = input.soft_delete,
            "Account deletion scheduled"
        );

        Ok(deletion)
    }

    /// Get deletion status for a user.
    pub async fn get_deletion_status(&self, user_id: &str) -> AppResult<Option<DeletionRecord>> {
        let deletion = self.deletion_repo.find_pending_by_user_id(user_id).await?;

        match deletion {
            Some(d) => Ok(Some(self.convert_deletion_model(d)?)),
            None => Ok(None),
        }
    }

    /// Convert deletion database model to API response.
    fn convert_deletion_model(&self, model: account_deletion::Model) -> AppResult<DeletionRecord> {
        let status = match model.status {
            account_deletion::DeletionStatus::Scheduled => DeletionStatus::Scheduled,
            account_deletion::DeletionStatus::InProgress => DeletionStatus::InProgress,
            account_deletion::DeletionStatus::Completed => DeletionStatus::Completed,
            account_deletion::DeletionStatus::Cancelled => DeletionStatus::Cancelled,
        };

        Ok(DeletionRecord {
            user_id: model.user_id,
            status,
            scheduled_at: model.scheduled_at.into(),
            completed_at: model.completed_at.map(Into::into),
            reason: model.reason,
        })
    }

    /// Cancel scheduled deletion.
    pub async fn cancel_deletion(&self, user_id: &str) -> AppResult<()> {
        let user = self.user_repo.get_by_id(user_id).await?;

        // Check if deletion is scheduled
        let deletion = self.deletion_repo.find_pending_by_user_id(user_id).await?;

        if let Some(d) = deletion {
            // Mark deletion as cancelled
            self.deletion_repo.mark_cancelled(&d.id).await?;
        }

        // Unsuspend user if suspended
        if user.is_suspended {
            let mut active: user::ActiveModel = user.into();
            active.is_suspended = Set(false);
            active.updated_at = Set(Some(Utc::now().into()));
            self.user_repo.update(active).await?;
        }

        tracing::info!(user_id = user_id, "Account deletion cancelled");

        Ok(())
    }

    /// Execute account deletion (called by background job).
    pub async fn execute_deletion(&self, user_id: &str, hard_delete: bool) -> AppResult<()> {
        let user = self.user_repo.get_by_id(user_id).await?;

        tracing::info!(
            user_id = user_id,
            hard_delete = hard_delete,
            "Executing account deletion"
        );

        // 1. Send Delete activity to all followers (ActivityPub)
        // Only for local users
        if user.host.is_none() {
            let actor_url = format!("{}/users/{}", self.server_url, user.id);
            let activity_id = format!("{}/delete/{}", actor_url, crate::generate_id());

            let delete_activity = serde_json::json!({
                "@context": "https://www.w3.org/ns/activitystreams",
                "id": activity_id,
                "type": "Delete",
                "actor": actor_url,
                "object": actor_url,
                "to": ["https://www.w3.org/ns/activitystreams#Public"]
            });

            // Get follower inboxes to deliver to
            let followers = self.following_repo.get_followers(user_id, 10000, 0).await?;
            let mut inboxes = Vec::new();

            for follow in followers {
                if let Ok(follower) = self.user_repo.find_by_id(&follow.follower_id).await
                    && let Some(follower) = follower
                    && follower.host.is_some()
                {
                    // Prefer shared inbox if available
                    if let Some(ref shared_inbox) = follower.shared_inbox {
                        if !inboxes.contains(shared_inbox) {
                            inboxes.push(shared_inbox.clone());
                        }
                    } else if let Some(ref inbox) = follower.inbox
                        && !inboxes.contains(inbox)
                    {
                        inboxes.push(inbox.clone());
                    }
                }
            }

            if !inboxes.is_empty()
                && let Err(e) = self
                    .delivery_service
                    .queue_delete_actor(user_id, delete_activity, inboxes)
                    .await
            {
                tracing::warn!(error = %e, "Failed to queue Delete activity");
            }
        }

        // 2. Delete all notes (mark as deleted to maintain tombstones)
        if let Err(e) = self.note_repo.delete_by_user(user_id).await {
            tracing::warn!(error = %e, "Failed to delete user notes");
        }

        // 3-5. Clear related data (following, follow requests cleared by cascade delete
        // or handled separately if needed)

        // 6-7. Perform deletion or anonymization
        if hard_delete {
            // Hard delete - mark as deleted
            self.user_repo.mark_as_deleted(user_id).await?;
        } else {
            // Soft delete - anonymize user
            self.user_repo.anonymize(user_id).await?;
        }

        tracing::info!(
            user_id = user_id,
            hard_delete = hard_delete,
            "Account deletion executed successfully"
        );

        Ok(())
    }

    // =====================
    // Account Export
    // =====================

    /// Create an export job.
    pub async fn create_export(
        &self,
        user_id: &str,
        input: CreateExportInput,
    ) -> AppResult<ExportJob> {
        let _user = self.user_repo.get_by_id(user_id).await?;

        let job_id = crate::generate_id();
        let now = Utc::now();

        let job = ExportJob {
            id: job_id.clone(),
            user_id: user_id.to_string(),
            data_types: input.data_types.clone(),
            format: input.format,
            status: ExportStatus::Queued,
            progress: 0,
            created_at: now,
            completed_at: None,
            download_url: None,
            expires_at: None,
            error: None,
        };

        // Store job in database
        let data_types_json: Vec<String> = input
            .data_types
            .iter()
            .map(|dt| format!("{dt:?}").to_lowercase())
            .collect();

        let format_db = match input.format {
            ExportFormat::ActivityPub => export_job::ExportFormat::ActivityPub,
            ExportFormat::Misskey => export_job::ExportFormat::Json,
            ExportFormat::Csv => export_job::ExportFormat::Csv,
        };

        let db_model = export_job::ActiveModel {
            id: Set(job_id.clone()),
            user_id: Set(user_id.to_string()),
            data_types: Set(serde_json::json!(data_types_json)),
            format: Set(format_db),
            status: Set(export_job::ExportStatus::Pending),
            progress: Set(0),
            error_message: Set(None),
            file_path: Set(None),
            download_url: Set(None),
            created_at: Set(now.into()),
            completed_at: Set(None),
            expires_at: Set(None),
        };
        self.export_job_repo.create(db_model).await?;

        // Queue background job to perform export
        if let Some(ref job_sender) = self.job_sender
            && let Err(e) = job_sender.export(job_id.clone(), user_id.to_string()).await
        {
            tracing::warn!(error = %e, "Failed to queue export job");
        }

        tracing::info!(user_id = user_id, job_id = job_id, "Export job created");

        Ok(job)
    }

    /// Export user profile data.
    pub async fn export_profile(&self, user_id: &str) -> AppResult<ExportedProfile> {
        let user = self.user_repo.get_by_id(user_id).await?;
        let profile = self
            .profile_repo
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User profile not found".to_string()))?;

        // Parse profile fields (fields is JsonBinary, not Option)
        let fields: Vec<ProfileField> =
            serde_json::from_value(profile.fields.clone()).unwrap_or_default();

        // Parse pinned notes (pinned_note_ids is JsonBinary, not Option)
        let pinned_notes: Vec<String> =
            serde_json::from_value(profile.pinned_note_ids).unwrap_or_default();

        Ok(ExportedProfile {
            username: user.username,
            name: user.name,
            description: user.description,
            avatar_url: user.avatar_url,
            banner_url: user.banner_url,
            is_bot: user.is_bot,
            is_cat: user.is_cat,
            is_locked: user.is_locked,
            fields,
            pinned_notes,
        })
    }

    /// Export following list.
    pub async fn export_following(&self, user_id: &str) -> AppResult<Vec<ExportedFollow>> {
        let following = self.following_repo.get_following(user_id, 10000, 0).await?;

        let mut result = Vec::new();
        for follow in following {
            let followee = self.user_repo.get_by_id(&follow.followee_id).await?;

            let acct = if let Some(host) = &followee.host {
                format!("{}@{}", followee.username, host)
            } else {
                followee.username.clone()
            };

            result.push(ExportedFollow {
                acct,
                uri: followee.uri,
            });
        }

        Ok(result)
    }

    /// Export followers list.
    pub async fn export_followers(&self, user_id: &str) -> AppResult<Vec<ExportedFollow>> {
        let followers = self.following_repo.get_followers(user_id, 10000, 0).await?;

        let mut result = Vec::new();
        for follow in followers {
            let follower = self.user_repo.get_by_id(&follow.follower_id).await?;

            let acct = if let Some(host) = &follower.host {
                format!("{}@{}", follower.username, host)
            } else {
                follower.username.clone()
            };

            result.push(ExportedFollow {
                acct,
                uri: follower.uri,
            });
        }

        Ok(result)
    }

    /// Export user's notes.
    ///
    /// Returns notes in chronological order (newest first) with pagination support.
    /// Each note includes text, CW, visibility, timestamps, and metadata.
    pub async fn export_notes(&self, user_id: &str, limit: u32) -> AppResult<Vec<ExportedNote>> {
        let notes = self
            .note_repo
            .find_by_user(user_id, u64::from(limit), None)
            .await?;

        let result: Vec<ExportedNote> = notes
            .into_iter()
            .map(|note| {
                // Parse file_ids from JSON
                let file_ids: Vec<String> =
                    serde_json::from_value(note.file_ids.clone()).unwrap_or_default();

                // Parse tags from JSON
                let tags: Vec<String> =
                    serde_json::from_value(note.tags.clone()).unwrap_or_default();

                // Convert visibility enum to string
                let visibility = match note.visibility {
                    misskey_db::entities::note::Visibility::Public => "public",
                    misskey_db::entities::note::Visibility::Home => "home",
                    misskey_db::entities::note::Visibility::Followers => "followers",
                    misskey_db::entities::note::Visibility::Specified => "specified",
                }
                .to_string();

                ExportedNote {
                    id: note.id,
                    text: note.text,
                    cw: note.cw,
                    visibility,
                    reply_id: note.reply_id,
                    renote_id: note.renote_id,
                    file_ids,
                    tags,
                    uri: note.uri,
                    url: note.url,
                    created_at: note.created_at.to_rfc3339(),
                    updated_at: note.updated_at.map(|dt| dt.to_rfc3339()),
                }
            })
            .collect();

        tracing::info!(user_id = user_id, count = result.len(), "Notes exported");

        Ok(result)
    }

    /// Export user's notes as CSV string.
    ///
    /// CSV format: `id,created_at,visibility,cw,text,reply_id,renote_id,tags,file_ids,uri,url`
    #[must_use]
    pub fn export_notes_as_csv(notes: &[ExportedNote]) -> String {
        let mut csv = String::from(
            "id,created_at,visibility,cw,text,reply_id,renote_id,tags,file_ids,uri,url\n",
        );

        for note in notes {
            // Escape CSV fields (double quotes and newlines)
            let escape_csv = |s: &str| {
                if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            };

            let text = note.text.as_deref().unwrap_or("");
            let cw = note.cw.as_deref().unwrap_or("");
            let reply_id = note.reply_id.as_deref().unwrap_or("");
            let renote_id = note.renote_id.as_deref().unwrap_or("");
            let uri = note.uri.as_deref().unwrap_or("");
            let url = note.url.as_deref().unwrap_or("");
            let tags = note.tags.join(";");
            let file_ids = note.file_ids.join(";");

            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{}\n",
                escape_csv(&note.id),
                escape_csv(&note.created_at),
                escape_csv(&note.visibility),
                escape_csv(cw),
                escape_csv(text),
                escape_csv(reply_id),
                escape_csv(renote_id),
                escape_csv(&tags),
                escape_csv(&file_ids),
                escape_csv(uri),
                escape_csv(url),
            ));
        }

        csv
    }

    /// Get export job status.
    pub async fn get_export_status(&self, user_id: &str, job_id: &str) -> AppResult<ExportJob> {
        let job = self
            .export_job_repo
            .get_by_id_and_user(job_id, user_id)
            .await?;

        self.convert_export_job_model(job)
    }

    /// Convert export job database model to API response.
    fn convert_export_job_model(&self, model: export_job::Model) -> AppResult<ExportJob> {
        // Parse data_types from JSON
        let data_types: Vec<ExportDataType> =
            serde_json::from_value(model.data_types).unwrap_or_default();

        // Convert format
        let format = match model.format {
            export_job::ExportFormat::Json => ExportFormat::Misskey,
            export_job::ExportFormat::Csv => ExportFormat::Csv,
            export_job::ExportFormat::ActivityPub => ExportFormat::ActivityPub,
        };

        // Convert status
        let status = match model.status {
            export_job::ExportStatus::Pending => ExportStatus::Queued,
            export_job::ExportStatus::Processing => ExportStatus::InProgress,
            export_job::ExportStatus::Completed => ExportStatus::Completed,
            export_job::ExportStatus::Failed => ExportStatus::Failed,
        };

        Ok(ExportJob {
            id: model.id,
            user_id: model.user_id,
            data_types,
            format,
            status,
            progress: model.progress.try_into().unwrap_or(0),
            created_at: model.created_at.into(),
            completed_at: model.completed_at.map(Into::into),
            download_url: model.download_url,
            expires_at: model.expires_at.map(Into::into),
            error: model.error_message,
        })
    }

    // =====================
    // Account Import
    // =====================

    /// Create an import job.
    pub async fn create_import(
        &self,
        user_id: &str,
        input: CreateImportInput,
    ) -> AppResult<ImportJob> {
        let _user = self.user_repo.get_by_id(user_id).await?;

        let job_id = crate::generate_id();
        let now = Utc::now();

        // Parse data to count items
        let total_items = match input.data_type {
            ExportDataType::Following | ExportDataType::Muting | ExportDataType::Blocking => {
                // Expect CSV or JSON array of accounts
                self.count_import_items(&input.data)?
            }
            _ => 0,
        };

        let job = ImportJob {
            id: job_id.clone(),
            user_id: user_id.to_string(),
            data_type: input.data_type,
            status: ImportStatus::Queued,
            progress: 0,
            total_items,
            imported_items: 0,
            skipped_items: 0,
            failed_items: 0,
            created_at: now,
            completed_at: None,
            error: None,
            item_errors: Vec::new(),
        };

        // Store job in database
        let data_type_db = match input.data_type {
            ExportDataType::Following => import_job::ImportDataType::Following,
            ExportDataType::Muting => import_job::ImportDataType::Muting,
            ExportDataType::Blocking => import_job::ImportDataType::Blocking,
            ExportDataType::UserLists => import_job::ImportDataType::UserLists,
            _ => import_job::ImportDataType::Following, // Default fallback
        };

        let db_model = import_job::ActiveModel {
            id: Set(job_id.clone()),
            user_id: Set(user_id.to_string()),
            data_type: Set(data_type_db),
            status: Set(import_job::ImportStatus::Queued),
            progress: Set(0),
            total_items: Set(total_items as i32),
            imported_items: Set(0),
            skipped_items: Set(0),
            failed_items: Set(0),
            error_message: Set(None),
            item_errors: Set(serde_json::json!([])),
            created_at: Set(now.into()),
            completed_at: Set(None),
        };
        self.import_job_repo.create(db_model).await?;

        // Queue background job to perform import
        if let Some(ref job_sender) = self.job_sender
            && let Err(e) = job_sender.import(job_id.clone(), user_id.to_string()).await
        {
            tracing::warn!(error = %e, "Failed to queue import job");
        }

        tracing::info!(
            user_id = user_id,
            job_id = job_id,
            data_type = ?input.data_type,
            total_items = total_items,
            "Import job created"
        );

        Ok(job)
    }

    /// Count items in import data.
    fn count_import_items(&self, data: &str) -> AppResult<u32> {
        // Try to parse as JSON array first
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(data) {
            return Ok(items.len() as u32);
        }

        // Try to parse as CSV (line count)
        let lines: Vec<&str> = data.lines().filter(|l| !l.trim().is_empty()).collect();
        Ok(lines.len() as u32)
    }

    /// Import following list from CSV.
    pub async fn import_following(&self, user_id: &str, data: &str) -> AppResult<ImportJob> {
        let job_id = crate::generate_id();
        let now = Utc::now();

        // Get the current user for creating follow relationships
        let user = self.user_repo.get_by_id(user_id).await?;

        let accounts: Vec<&str> = data
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .collect();

        let mut job = ImportJob {
            id: job_id.clone(),
            user_id: user_id.to_string(),
            data_type: ExportDataType::Following,
            status: ImportStatus::InProgress,
            progress: 0,
            total_items: accounts.len() as u32,
            imported_items: 0,
            skipped_items: 0,
            failed_items: 0,
            created_at: now,
            completed_at: None,
            error: None,
            item_errors: Vec::new(),
        };

        for (index, acct) in accounts.iter().enumerate() {
            let acct = acct.trim();

            // Parse acct format: username@host or just username
            let (username, host) = if acct.contains('@') {
                let parts: Vec<&str> = acct.splitn(2, '@').collect();
                (parts[0], Some(parts.get(1).copied().unwrap_or("")))
            } else {
                (acct, None)
            };

            // Try to find or resolve the user
            match self
                .user_repo
                .find_by_username_and_host(username, host)
                .await
            {
                Ok(Some(target)) => {
                    // Check if already following
                    if self
                        .following_repo
                        .is_following(user_id, &target.id)
                        .await?
                    {
                        job.skipped_items += 1;
                    } else if self.follow_request_repo.exists(user_id, &target.id).await? {
                        // Follow request already pending
                        job.skipped_items += 1;
                    } else {
                        // Create follow or follow request depending on target's settings
                        match self.create_follow_or_request(user_id, &target, &user).await {
                            Ok(()) => job.imported_items += 1,
                            Err(e) => {
                                job.item_errors.push(ImportItemError {
                                    index: index as u32,
                                    identifier: acct.to_string(),
                                    error: e.to_string(),
                                });
                                job.failed_items += 1;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // User not found - would need to resolve from remote
                    job.item_errors.push(ImportItemError {
                        index: index as u32,
                        identifier: acct.to_string(),
                        error: "User not found".to_string(),
                    });
                    job.failed_items += 1;
                }
                Err(e) => {
                    job.item_errors.push(ImportItemError {
                        index: index as u32,
                        identifier: acct.to_string(),
                        error: e.to_string(),
                    });
                    job.failed_items += 1;
                }
            }

            job.progress = ((index + 1) * 100 / accounts.len()) as u8;
        }

        job.status = if job.failed_items > 0 && job.imported_items > 0 {
            ImportStatus::PartiallyCompleted
        } else if job.failed_items > 0 {
            ImportStatus::Failed
        } else {
            ImportStatus::Completed
        };
        job.completed_at = Some(Utc::now());

        tracing::info!(
            user_id = user_id,
            job_id = job_id,
            imported = job.imported_items,
            skipped = job.skipped_items,
            failed = job.failed_items,
            "Following import completed"
        );

        Ok(job)
    }

    /// Create a follow relationship or follow request depending on target's settings.
    ///
    /// If the target user has a locked account, this creates a follow request.
    /// Otherwise, it creates a direct follow relationship.
    async fn create_follow_or_request(
        &self,
        follower_id: &str,
        target: &user::Model,
        follower: &user::Model,
    ) -> AppResult<()> {
        if target.is_locked {
            // Target has a locked account - create follow request
            let request = follow_request::ActiveModel {
                id: Set(crate::generate_id()),
                follower_id: Set(follower_id.to_string()),
                followee_id: Set(target.id.clone()),
                follower_host: Set(follower.host.clone()),
                followee_host: Set(target.host.clone()),
                follower_inbox: Set(follower.inbox.clone()),
                follower_shared_inbox: Set(follower.shared_inbox.clone()),
                ..Default::default()
            };
            self.follow_request_repo.create(request).await?;
            tracing::debug!(
                follower_id = follower_id,
                followee_id = %target.id,
                "Created follow request during import"
            );
        } else {
            // Target has an unlocked account - create direct follow
            let follow = following::ActiveModel {
                id: Set(crate::generate_id()),
                follower_id: Set(follower_id.to_string()),
                followee_id: Set(target.id.clone()),
                follower_host: Set(follower.host.clone()),
                followee_host: Set(target.host.clone()),
                followee_inbox: Set(target.inbox.clone()),
                followee_shared_inbox: Set(target.shared_inbox.clone()),
                created_at: Set(Utc::now().into()),
            };
            self.following_repo.create(follow).await?;
            tracing::debug!(
                follower_id = follower_id,
                followee_id = %target.id,
                "Created follow during import"
            );
        }
        Ok(())
    }

    /// Get import job status.
    pub async fn get_import_status(&self, user_id: &str, job_id: &str) -> AppResult<ImportJob> {
        let job = self
            .import_job_repo
            .get_by_id_and_user(job_id, user_id)
            .await?;

        self.convert_import_job_model(job)
    }

    /// Convert import job database model to API response.
    fn convert_import_job_model(&self, model: import_job::Model) -> AppResult<ImportJob> {
        // Convert data type
        let data_type = match model.data_type {
            import_job::ImportDataType::Following => ExportDataType::Following,
            import_job::ImportDataType::Muting => ExportDataType::Muting,
            import_job::ImportDataType::Blocking => ExportDataType::Blocking,
            import_job::ImportDataType::UserLists => ExportDataType::UserLists,
        };

        // Convert status
        let status = match model.status {
            import_job::ImportStatus::Queued => ImportStatus::Queued,
            import_job::ImportStatus::Validating => ImportStatus::Validating,
            import_job::ImportStatus::Processing => ImportStatus::InProgress,
            import_job::ImportStatus::Completed => ImportStatus::Completed,
            import_job::ImportStatus::PartiallyCompleted => ImportStatus::PartiallyCompleted,
            import_job::ImportStatus::Failed => ImportStatus::Failed,
        };

        // Parse item errors from JSON
        let item_errors: Vec<ImportItemError> =
            serde_json::from_value(model.item_errors).unwrap_or_default();

        Ok(ImportJob {
            id: model.id,
            user_id: model.user_id,
            data_type,
            status,
            progress: model.progress.try_into().unwrap_or(0),
            total_items: model.total_items.try_into().unwrap_or(0),
            imported_items: model.imported_items.try_into().unwrap_or(0),
            skipped_items: model.skipped_items.try_into().unwrap_or(0),
            failed_items: model.failed_items.try_into().unwrap_or(0),
            created_at: model.created_at.into(),
            completed_at: model.completed_at.map(Into::into),
            error: model.error_message,
            item_errors,
        })
    }

    /// Parse Mastodon-format CSV data (handles header row and column extraction).
    ///
    /// Mastodon exports CSVs with headers like "Account address" or "account".
    /// This function extracts the account addresses from the first column.
    ///
    /// Supported formats:
    /// - Simple list: one account per line (username@host)
    /// - Mastodon CSV: header row + comma-separated data
    #[must_use]
    pub fn parse_mastodon_csv(data: &str) -> Vec<String> {
        let mut accounts = Vec::new();
        let lines: Vec<&str> = data.lines().collect();

        if lines.is_empty() {
            return accounts;
        }

        // Check if first line looks like a header
        let first_line = lines[0].trim().to_lowercase();
        let has_header = first_line.contains("account")
            || first_line.contains("address")
            || first_line.starts_with('#');

        let start_index = usize::from(has_header);

        for line in lines.into_iter().skip(start_index) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle CSV format: extract first column
            let account = if line.contains(',') {
                // CSV format: get first column
                let parts: Vec<&str> = line.splitn(2, ',').collect();
                parts[0].trim().trim_matches('"')
            } else {
                // Simple format: whole line is the account
                line
            };

            if !account.is_empty() {
                accounts.push(account.to_string());
            }
        }

        accounts
    }

    /// Parse account string into (username, host) tuple.
    #[must_use]
    pub fn parse_acct(acct: &str) -> (String, Option<String>) {
        let acct = acct.trim().trim_start_matches('@');
        if acct.contains('@') {
            let parts: Vec<&str> = acct.splitn(2, '@').collect();
            (
                parts[0].to_string(),
                Some(parts.get(1).copied().unwrap_or("").to_string()),
            )
        } else {
            (acct.to_string(), None)
        }
    }
}

/// Response for migration status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationStatusResponse {
    /// Whether migration is in progress
    pub has_pending_migration: bool,
    /// Current migration record (if any)
    pub migration: Option<MigrationRecord>,
    /// Aliases set on this account
    pub aliases: Vec<String>,
    /// URI this account has moved to (if migrated)
    pub moved_to: Option<String>,
}

/// Response for deletion status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionStatusResponse {
    /// Whether deletion is scheduled
    pub is_scheduled: bool,
    /// Deletion record (if any)
    pub deletion: Option<DeletionRecord>,
}

/// Verify a password against a hash.
fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    use argon2::{Argon2, PasswordVerifier, password_hash::PasswordHash};

    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| AppError::Internal(format!("Invalid hash: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
