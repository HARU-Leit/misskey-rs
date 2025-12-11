//! Remote actor fetching utility.

use chrono::Utc;
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{entities::user, repositories::UserRepository};
use sea_orm::Set;
use serde_json::Value;
use tracing::{debug, info};
use url::Url;

use crate::client::ApClient;

/// Utility for fetching and creating remote actors.
#[derive(Clone)]
pub struct ActorFetcher {
    user_repo: UserRepository,
    ap_client: ApClient,
    id_gen: IdGenerator,
}

impl ActorFetcher {
    /// Create a new actor fetcher.
    #[must_use]
    pub fn new(user_repo: UserRepository, ap_client: ApClient) -> Self {
        Self {
            user_repo,
            ap_client,
            id_gen: IdGenerator::new(),
        }
    }

    /// Find an existing remote actor or fetch from remote server.
    pub async fn find_or_fetch(&self, actor_url: &Url) -> AppResult<user::Model> {
        // First, try to find by URI
        if let Some(user) = self.user_repo.find_by_uri(actor_url.as_str()).await? {
            debug!(actor_url = %actor_url, "Found existing remote actor");
            return Ok(user);
        }

        // Fetch actor from remote server
        info!(actor_url = %actor_url, "Fetching remote actor");
        let actor_json = self
            .ap_client
            .fetch_actor(actor_url.as_str())
            .await
            .map_err(|e| AppError::Federation(format!("Failed to fetch remote actor: {e}")))?;

        // Parse and save the actor
        self.create_remote_user_from_actor(&actor_json, actor_url)
            .await
    }

    /// Create a remote user from an ActivityPub actor JSON.
    async fn create_remote_user_from_actor(
        &self,
        actor: &Value,
        actor_url: &Url,
    ) -> AppResult<user::Model> {
        // Extract host from URL
        let host = actor_url
            .host_str()
            .ok_or_else(|| AppError::BadRequest("Invalid actor URL: no host".to_string()))?
            .to_string();

        // Extract required fields
        let preferred_username = actor
            .get("preferredUsername")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::BadRequest("Actor missing preferredUsername".to_string()))?;

        // Extract optional fields
        let name = actor.get("name").and_then(Value::as_str).map(String::from);
        let summary = actor
            .get("summary")
            .and_then(Value::as_str)
            .map(String::from);

        let inbox = actor.get("inbox").and_then(Value::as_str).map(String::from);

        let shared_inbox = actor
            .get("endpoints")
            .and_then(|e| e.get("sharedInbox"))
            .and_then(Value::as_str)
            .map(String::from)
            .or_else(|| {
                actor
                    .get("sharedInbox")
                    .and_then(Value::as_str)
                    .map(String::from)
            });

        let avatar_url = actor
            .get("icon")
            .and_then(|icon| {
                if icon.is_object() {
                    icon.get("url").and_then(Value::as_str)
                } else if icon.is_string() {
                    icon.as_str()
                } else {
                    None
                }
            })
            .map(String::from);

        let banner_url = actor
            .get("image")
            .and_then(|image| {
                if image.is_object() {
                    image.get("url").and_then(Value::as_str)
                } else if image.is_string() {
                    image.as_str()
                } else {
                    None
                }
            })
            .map(String::from);

        let is_locked = actor
            .get("manuallyApprovesFollowers")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let is_bot = actor
            .get("type")
            .and_then(Value::as_str)
            .map(|t| t == "Service" || t == "Application")
            .unwrap_or(false);

        let is_cat = actor.get("isCat").and_then(Value::as_bool).unwrap_or(false);

        let featured = actor
            .get("featured")
            .and_then(Value::as_str)
            .map(String::from);

        // Check if a user with the same username@host already exists
        if let Some(existing) = self
            .user_repo
            .find_by_username_and_host(preferred_username, Some(&host))
            .await?
        {
            // User exists, update URI and return
            info!(
                username = %preferred_username,
                host = %host,
                "Found existing user with matching username@host, updating URI"
            );
            let mut active: user::ActiveModel = existing.into();
            active.uri = Set(Some(actor_url.to_string()));
            active.inbox = Set(inbox);
            active.shared_inbox = Set(shared_inbox);
            active.last_fetched_at = Set(Some(Utc::now().into()));
            active.updated_at = Set(Some(Utc::now().into()));
            return self.user_repo.update(active).await;
        }

        // Generate unique ID
        let id = self.id_gen.generate();

        // Create new remote user
        let model = user::ActiveModel {
            id: Set(id),
            username: Set(preferred_username.to_string()),
            username_lower: Set(preferred_username.to_lowercase()),
            host: Set(Some(host)),
            name: Set(name),
            description: Set(summary),
            avatar_url: Set(avatar_url),
            banner_url: Set(banner_url),
            is_bot: Set(is_bot),
            is_cat: Set(is_cat),
            is_locked: Set(is_locked),
            inbox: Set(inbox),
            shared_inbox: Set(shared_inbox),
            featured: Set(featured),
            uri: Set(Some(actor_url.to_string())),
            last_fetched_at: Set(Some(Utc::now().into())),
            created_at: Set(Utc::now().into()),
            ..Default::default()
        };

        let user = self.user_repo.create(model).await?;

        info!(
            user_id = %user.id,
            username = %user.username,
            host = ?user.host,
            "Created remote user"
        );

        Ok(user)
    }
}
