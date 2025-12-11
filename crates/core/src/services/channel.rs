//! Channel service.

use chrono::Utc;
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::channel;
use misskey_db::repositories::ChannelRepository;
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs8::EncodePrivateKey, pkcs8::EncodePublicKey};
use sea_orm::Set;
use serde::Deserialize;
use validator::Validate;

/// Maximum number of channels per user.
const MAX_CHANNELS_PER_USER: u64 = 10;

/// Input for creating a channel.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateChannelInput {
    #[validate(length(min = 1, max = 128))]
    pub name: String,
    #[validate(length(max = 2048))]
    pub description: Option<String>,
    pub banner_id: Option<String>,
    pub color: Option<String>,
    #[serde(default = "default_true")]
    pub is_searchable: bool,
    #[serde(default = "default_true")]
    pub allow_anyone_to_post: bool,
}

const fn default_true() -> bool {
    true
}

/// Input for updating a channel.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChannelInput {
    pub channel_id: String,
    #[validate(length(min = 1, max = 128))]
    pub name: Option<String>,
    #[validate(length(max = 2048))]
    pub description: Option<Option<String>>,
    pub banner_id: Option<Option<String>>,
    pub color: Option<Option<String>>,
    pub is_searchable: Option<bool>,
    pub allow_anyone_to_post: Option<bool>,
}

/// Service for managing channels.
#[derive(Clone)]
pub struct ChannelService {
    channel_repo: ChannelRepository,
    id_gen: IdGenerator,
    base_url: Option<String>,
}

impl ChannelService {
    /// Create a new channel service.
    #[must_use]
    pub const fn new(channel_repo: ChannelRepository) -> Self {
        Self {
            channel_repo,
            id_gen: IdGenerator::new(),
            base_url: None,
        }
    }

    /// Create a new channel service with federation support.
    #[must_use]
    pub const fn with_federation(channel_repo: ChannelRepository, base_url: String) -> Self {
        Self {
            channel_repo,
            id_gen: IdGenerator::new(),
            base_url: Some(base_url),
        }
    }

    /// Get a channel by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<channel::Model>> {
        self.channel_repo.find_by_id(id).await
    }

    /// Get a channel by ID with ownership check.
    pub async fn get_by_id_for_owner(&self, id: &str, user_id: &str) -> AppResult<channel::Model> {
        let channel = self.channel_repo.get_by_id(id).await?;

        if channel.user_id != user_id {
            return Err(AppError::Forbidden("Not the channel owner".to_string()));
        }

        Ok(channel)
    }

    /// List owned channels.
    pub async fn list_owned(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<channel::Model>> {
        self.channel_repo.find_by_user(user_id, limit, offset).await
    }

    /// List followed channels.
    pub async fn list_followed(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<channel::Model>> {
        self.channel_repo
            .find_followed_by_user(user_id, limit, offset)
            .await
    }

    /// List featured channels.
    pub async fn list_featured(&self, limit: u64, offset: u64) -> AppResult<Vec<channel::Model>> {
        self.channel_repo.find_featured(limit, offset).await
    }

    /// Search channels.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<channel::Model>> {
        if query.trim().is_empty() {
            return self.list_featured(limit, offset).await;
        }

        self.channel_repo.search(query, limit, offset).await
    }

    /// Create a new channel.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateChannelInput,
    ) -> AppResult<channel::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check channel limit
        let count = self.channel_repo.count_by_user(user_id).await?;
        if count >= MAX_CHANNELS_PER_USER {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_CHANNELS_PER_USER} channels allowed per user"
            )));
        }

        // Validate color if provided
        if let Some(ref color) = input.color
            && !is_valid_color(color)
        {
            return Err(AppError::Validation("Invalid color format".to_string()));
        }

        let id = self.id_gen.generate();
        let now = Utc::now();

        let model = channel::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            name: Set(input.name),
            description: Set(input.description),
            banner_id: Set(input.banner_id),
            color: Set(input.color),
            is_archived: Set(false),
            is_searchable: Set(input.is_searchable),
            allow_anyone_to_post: Set(input.allow_anyone_to_post),
            notes_count: Set(0),
            users_count: Set(0),
            last_noted_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(None),
            // Federation fields (not enabled by default)
            uri: Set(None),
            public_key_pem: Set(None),
            private_key_pem: Set(None),
            inbox: Set(None),
            shared_inbox: Set(None),
            host: Set(None), // Local channel
        };

        self.channel_repo.create(model).await
    }

    /// Update a channel.
    pub async fn update(
        &self,
        user_id: &str,
        input: UpdateChannelInput,
    ) -> AppResult<channel::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Get channel and verify ownership
        let channel = self.get_by_id_for_owner(&input.channel_id, user_id).await?;

        // Validate color if provided
        if let Some(Some(ref color)) = input.color
            && !is_valid_color(color)
        {
            return Err(AppError::Validation("Invalid color format".to_string()));
        }

        let now = Utc::now();
        let mut active: channel::ActiveModel = channel.into();

        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(description) = input.description {
            active.description = Set(description);
        }
        if let Some(banner_id) = input.banner_id {
            active.banner_id = Set(banner_id);
        }
        if let Some(color) = input.color {
            active.color = Set(color);
        }
        if let Some(is_searchable) = input.is_searchable {
            active.is_searchable = Set(is_searchable);
        }
        if let Some(allow_anyone_to_post) = input.allow_anyone_to_post {
            active.allow_anyone_to_post = Set(allow_anyone_to_post);
        }

        active.updated_at = Set(Some(now.into()));

        self.channel_repo.update(active).await
    }

    /// Archive a channel (soft delete).
    pub async fn archive(&self, channel_id: &str, user_id: &str) -> AppResult<channel::Model> {
        // Verify ownership
        self.get_by_id_for_owner(channel_id, user_id).await?;
        self.channel_repo.archive(channel_id).await
    }

    /// Delete a channel permanently.
    pub async fn delete(&self, channel_id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        self.get_by_id_for_owner(channel_id, user_id).await?;
        self.channel_repo.delete(channel_id).await
    }

    // ==================== Following ====================

    /// Check if user is following a channel.
    pub async fn is_following(&self, user_id: &str, channel_id: &str) -> AppResult<bool> {
        self.channel_repo.is_following(user_id, channel_id).await
    }

    /// Follow a channel.
    pub async fn follow(&self, user_id: &str, channel_id: &str) -> AppResult<()> {
        // Check if channel exists and is not archived
        let channel = self.channel_repo.get_by_id(channel_id).await?;

        if channel.is_archived {
            return Err(AppError::Validation(
                "Cannot follow an archived channel".to_string(),
            ));
        }

        // Check if already following
        if self.channel_repo.is_following(user_id, channel_id).await? {
            return Err(AppError::Validation(
                "Already following this channel".to_string(),
            ));
        }

        let id = self.id_gen.generate();
        self.channel_repo
            .follow(id, user_id.to_string(), channel_id.to_string())
            .await?;

        Ok(())
    }

    /// Unfollow a channel.
    pub async fn unfollow(&self, user_id: &str, channel_id: &str) -> AppResult<()> {
        // Check if following
        if !self.channel_repo.is_following(user_id, channel_id).await? {
            return Err(AppError::Validation(
                "Not following this channel".to_string(),
            ));
        }

        self.channel_repo.unfollow(user_id, channel_id).await
    }

    // ==================== Note Operations ====================

    /// Record a note posted to channel.
    pub async fn on_note_posted(&self, channel_id: &str) -> AppResult<()> {
        self.channel_repo.increment_notes_count(channel_id).await
    }

    /// Record a note deleted from channel.
    pub async fn on_note_deleted(&self, channel_id: &str) -> AppResult<()> {
        self.channel_repo.decrement_notes_count(channel_id).await
    }

    /// Check if user can post to channel.
    pub async fn can_post(&self, user_id: &str, channel_id: &str) -> AppResult<bool> {
        let channel = self.channel_repo.get_by_id(channel_id).await?;

        if channel.is_archived {
            return Ok(false);
        }

        // Owner can always post
        if channel.user_id == user_id {
            return Ok(true);
        }

        // Check if anyone can post
        if channel.allow_anyone_to_post {
            return Ok(true);
        }

        // Otherwise, only followers can post
        self.channel_repo.is_following(user_id, channel_id).await
    }

    // ==================== Federation ====================

    /// Enable federation for a channel.
    ///
    /// Generates a keypair and sets up the channel for `ActivityPub` federation.
    pub async fn enable_federation(
        &self,
        channel_id: &str,
        user_id: &str,
    ) -> AppResult<channel::Model> {
        // Verify ownership
        let channel = self.get_by_id_for_owner(channel_id, user_id).await?;

        // Check if federation is already enabled
        if channel.uri.is_some() {
            return Err(AppError::Validation(
                "Federation is already enabled for this channel".to_string(),
            ));
        }

        // Check if base_url is configured
        let base_url = self.base_url.as_ref().ok_or_else(|| {
            AppError::Internal("Federation not configured: missing base URL".to_string())
        })?;

        // Generate RSA keypair
        let (private_key_pem, public_key_pem) = generate_keypair()?;

        // Build URIs
        let uri = format!("{base_url}/channels/{channel_id}");
        let inbox = format!("{base_url}/channels/{channel_id}/inbox");
        let shared_inbox = format!("{base_url}/inbox");

        let now = Utc::now();
        let mut active: channel::ActiveModel = channel.into();
        active.uri = Set(Some(uri));
        active.public_key_pem = Set(Some(public_key_pem));
        active.private_key_pem = Set(Some(private_key_pem));
        active.inbox = Set(Some(inbox));
        active.shared_inbox = Set(Some(shared_inbox));
        active.updated_at = Set(Some(now.into()));

        self.channel_repo.update(active).await
    }

    /// Disable federation for a channel.
    ///
    /// Removes the keypair and `ActivityPub` URIs.
    pub async fn disable_federation(
        &self,
        channel_id: &str,
        user_id: &str,
    ) -> AppResult<channel::Model> {
        // Verify ownership
        let channel = self.get_by_id_for_owner(channel_id, user_id).await?;

        // Check if federation is enabled
        if channel.uri.is_none() {
            return Err(AppError::Validation(
                "Federation is not enabled for this channel".to_string(),
            ));
        }

        let now = Utc::now();
        let mut active: channel::ActiveModel = channel.into();
        active.uri = Set(None);
        active.public_key_pem = Set(None);
        active.private_key_pem = Set(None);
        active.inbox = Set(None);
        active.shared_inbox = Set(None);
        active.updated_at = Set(Some(now.into()));

        self.channel_repo.update(active).await
    }

    /// Check if federation is enabled for a channel.
    pub async fn is_federation_enabled(&self, channel_id: &str) -> AppResult<bool> {
        let channel = self.channel_repo.get_by_id(channel_id).await?;
        Ok(channel.uri.is_some())
    }

    /// Get a channel by its `ActivityPub` URI.
    pub async fn get_by_uri(&self, uri: &str) -> AppResult<Option<channel::Model>> {
        self.channel_repo.find_by_uri(uri).await
    }
}

/// Generate RSA keypair for `ActivityPub` signatures.
fn generate_keypair() -> AppResult<(String, String)> {
    use rand::rngs::OsRng;

    let mut rng = OsRng;
    let bits = 2048;

    let private_key = RsaPrivateKey::new(&mut rng, bits)
        .map_err(|e| AppError::Internal(format!("Failed to generate RSA key: {e}")))?;

    let public_key = RsaPublicKey::from(&private_key);

    let private_key_pem = private_key
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode private key: {e}")))?
        .to_string();

    let public_key_pem = public_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode public key: {e}")))?;

    Ok((private_key_pem, public_key_pem))
}

/// Validate hex color format.
fn is_valid_color(color: &str) -> bool {
    // Accept formats: #RGB, #RRGGBB
    if !color.starts_with('#') {
        return false;
    }

    let hex = &color[1..];

    match hex.len() {
        3 | 6 => hex.chars().all(|c| c.is_ascii_hexdigit()),
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

    fn create_test_channel(id: &str, user_id: &str, name: &str) -> channel::Model {
        channel::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            description: None,
            banner_id: None,
            color: None,
            is_archived: false,
            is_searchable: true,
            allow_anyone_to_post: true,
            notes_count: 0,
            users_count: 0,
            last_noted_at: None,
            created_at: Utc::now().into(),
            updated_at: None,
            // Federation fields
            uri: None,
            public_key_pem: None,
            private_key_pem: None,
            inbox: None,
            shared_inbox: None,
            host: None,
        }
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let channel = create_test_channel("ch1", "user1", "My Channel");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[channel.clone()]])
                .into_connection(),
        );

        let repo = ChannelRepository::new(db);
        let service = ChannelService::new(repo);

        let result = service.get_by_id("ch1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Channel");
    }

    #[test]
    fn test_is_valid_color() {
        assert!(is_valid_color("#fff"));
        assert!(is_valid_color("#FFF"));
        assert!(is_valid_color("#ffffff"));
        assert!(is_valid_color("#FFFFFF"));
        assert!(is_valid_color("#123abc"));

        assert!(!is_valid_color("fff"));
        assert!(!is_valid_color("#ff"));
        assert!(!is_valid_color("#fffffff"));
        assert!(!is_valid_color("#gggggg"));
        assert!(!is_valid_color(""));
    }

    #[tokio::test]
    async fn test_get_by_id_for_owner_forbidden() {
        let channel = create_test_channel("ch1", "user1", "My Channel");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[channel.clone()]])
                .into_connection(),
        );

        let repo = ChannelRepository::new(db);
        let service = ChannelService::new(repo);

        let result = service.get_by_id_for_owner("ch1", "user2").await;

        assert!(result.is_err());
    }
}
