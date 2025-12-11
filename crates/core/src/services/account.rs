//! Account management service for migration, deletion, export, and import.

use chrono::{DateTime, Utc};
use misskey_common::{AppError, AppResult, Config};
use misskey_db::{
    entities::{user, user_profile},
    repositories::{
        FollowingRepository, NoteRepository, UserKeypairRepository, UserProfileRepository,
        UserRepository,
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
    /// ActivityPub Actor JSON
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
    /// ActivityPub URI (if available)
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
    /// ActivityPub URI
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

fn default_export_limit() -> u32 {
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
    keypair_repo: UserKeypairRepository,
    note_repo: NoteRepository,
    following_repo: FollowingRepository,
    delivery_service: DeliveryService,
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
        delivery_service: DeliveryService,
        config: &Config,
    ) -> Self {
        Self {
            user_repo,
            profile_repo,
            keypair_repo,
            note_repo,
            following_repo,
            delivery_service,
            server_url: config.server.url.clone(),
        }
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
        match profile {
            Some(p) => {
                let mut active: user_profile::ActiveModel = p.into();
                active.moved_to_uri = Set(Some(input.target_uri.clone()));
                active.updated_at = Set(Some(Utc::now().into()));
                self.profile_repo.update(active).await?;
            }
            None => {
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
                    created_at: Set(Utc::now().into()),
                    updated_at: Set(None),
                };
                self.profile_repo.create(model).await?;
            }
        }

        // Build Move activity
        let actor_url = format!("{}/users/{}", self.server_url, user.id);
        let activity_id = format!("{}/move/{}", actor_url, crate::generate_id());
        let followers_url = format!("{}/followers", actor_url);

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
        let user = self.user_repo.get_by_id(user_id).await?;

        // Validate aliases are valid URIs
        for alias in &aliases {
            if !alias.starts_with("https://") && !alias.starts_with("http://") {
                return Err(AppError::Validation(format!(
                    "Invalid alias URI: {}",
                    alias
                )));
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
                .map_err(|e| AppError::Internal(format!("Failed to parse aliases: {}", e)))?;
            Ok(aliases)
        } else {
            Ok(Vec::new())
        }
    }

    /// Cancel a pending migration.
    pub async fn cancel_migration(&self, user_id: &str, migration_id: &str) -> AppResult<()> {
        // TODO: In a full implementation:
        // 1. Verify migration belongs to user
        // 2. Check migration is in cancellable state
        // 3. Update migration status
        // 4. Clear movedToUri on user if set

        tracing::info!(
            user_id = user_id,
            migration_id = migration_id,
            "Migration cancelled"
        );

        Ok(())
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

        let deletion = DeletionRecord {
            user_id: user_id.to_string(),
            status: DeletionStatus::Scheduled,
            scheduled_at,
            completed_at: None,
            reason: input.reason,
        };

        // TODO: In a full implementation:
        // 1. Store deletion record in database
        // 2. Send confirmation email
        // 3. Set is_deleted or scheduled_deletion_at on user
        // 4. Queue deletion job for scheduled time

        if input.soft_delete {
            // Mark user as suspended/hidden immediately
            let mut active: user::ActiveModel = user.into();
            active.is_suspended = Set(true);
            active.updated_at = Set(Some(Utc::now().into()));
            self.user_repo.update(active).await?;
        }

        tracing::info!(
            user_id = user_id,
            scheduled_at = %scheduled_at,
            soft_delete = input.soft_delete,
            "Account deletion scheduled"
        );

        Ok(deletion)
    }

    /// Cancel scheduled deletion.
    pub async fn cancel_deletion(&self, user_id: &str) -> AppResult<()> {
        let user = self.user_repo.get_by_id(user_id).await?;

        // TODO: In a full implementation:
        // 1. Check if deletion is scheduled
        // 2. Remove deletion record
        // 3. Unsuspend user if soft-deleted

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

        // TODO: In a full implementation:
        // 1. Send Delete activity to all followers (ActivityPub)
        // 2. Delete all notes (or just mark as deleted)
        // 3. Delete all drive files (or just metadata)
        // 4. Remove from all lists, antennas, etc.
        // 5. Clear notification references
        // 6. If hard delete: remove user record entirely
        // 7. If soft delete: anonymize and keep tombstone

        if hard_delete {
            // Hard delete - mark as deleted and anonymize
            // Note: In a production system, you'd want to cascade delete related data
            // For now, we use the same anonymization approach
            self.user_repo.mark_as_deleted(user_id).await?;
        } else {
            // Anonymize user
            let mut active: user::ActiveModel = user.into();
            active.username = Set(format!("deleted_{}", user_id));
            active.name = Set(None);
            active.description = Set(None);
            active.avatar_url = Set(None);
            active.banner_url = Set(None);
            active.is_suspended = Set(true);
            active.updated_at = Set(Some(Utc::now().into()));
            self.user_repo.update(active).await?;
        }

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
            data_types: input.data_types,
            format: input.format,
            status: ExportStatus::Queued,
            progress: 0,
            created_at: now,
            completed_at: None,
            download_url: None,
            expires_at: None,
            error: None,
        };

        // TODO: Queue background job to perform export

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
            serde_json::from_value(profile.pinned_note_ids.clone()).unwrap_or_default();

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
    pub async fn export_notes(
        &self,
        user_id: &str,
        limit: u32,
    ) -> AppResult<Vec<ExportedNote>> {
        let notes = self
            .note_repo
            .find_by_user(user_id, limit as u64, None)
            .await?;

        let result: Vec<ExportedNote> = notes
            .into_iter()
            .map(|note| {
                // Parse file_ids from JSON
                let file_ids: Vec<String> = serde_json::from_value(note.file_ids.clone())
                    .unwrap_or_default();

                // Parse tags from JSON
                let tags: Vec<String> = serde_json::from_value(note.tags.clone())
                    .unwrap_or_default();

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

        tracing::info!(
            user_id = user_id,
            count = result.len(),
            "Notes exported"
        );

        Ok(result)
    }

    /// Export user's notes as CSV string.
    ///
    /// CSV format: id,created_at,visibility,cw,text,reply_id,renote_id,tags,file_ids,uri,url
    pub fn export_notes_as_csv(notes: &[ExportedNote]) -> String {
        let mut csv = String::from("id,created_at,visibility,cw,text,reply_id,renote_id,tags,file_ids,uri,url\n");

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
    pub async fn get_export_status(&self, _user_id: &str, job_id: &str) -> AppResult<ExportJob> {
        // TODO: Fetch from database
        Err(AppError::NotFound(format!(
            "Export job {} not found",
            job_id
        )))
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

        // TODO: Queue background job to perform import

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
                    } else {
                        // TODO: Create follow request
                        job.imported_items += 1;
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

    /// Get import job status.
    pub async fn get_import_status(&self, _user_id: &str, job_id: &str) -> AppResult<ImportJob> {
        // TODO: Fetch from database
        Err(AppError::NotFound(format!(
            "Import job {} not found",
            job_id
        )))
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
