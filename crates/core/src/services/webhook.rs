//! Webhook service for event notifications.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::webhook;
use misskey_db::repositories::WebhookRepository;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use std::sync::Arc;

/// Webhook events.
pub mod events {
    pub const NOTE: &str = "note";
    pub const REPLY: &str = "reply";
    pub const RENOTE: &str = "renote";
    pub const MENTION: &str = "mention";
    pub const FOLLOW: &str = "follow";
    pub const FOLLOWED: &str = "followed";
    pub const UNFOLLOW: &str = "unfollow";
    pub const REACTION: &str = "reaction";

    /// Get all valid events.
    #[must_use]
    pub fn all() -> Vec<&'static str> {
        vec![
            NOTE, REPLY, RENOTE, MENTION, FOLLOW, FOLLOWED, UNFOLLOW, REACTION,
        ]
    }

    /// Check if an event is valid.
    #[must_use]
    pub fn is_valid(event: &str) -> bool {
        all().contains(&event)
    }
}

/// Input for creating a webhook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookInput {
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
}

/// Input for updating a webhook.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookInput {
    pub name: Option<String>,
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

/// Response for a webhook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub last_triggered_at: Option<String>,
    pub failure_count: i32,
    pub last_error: Option<String>,
    pub created_at: String,
}

impl From<webhook::Model> for WebhookResponse {
    fn from(w: webhook::Model) -> Self {
        Self {
            id: w.id,
            name: w.name,
            url: w.url,
            events: serde_json::from_value(w.events).unwrap_or_default(),
            is_active: w.is_active,
            last_triggered_at: w.last_triggered_at.map(|t| t.to_rfc3339()),
            failure_count: w.failure_count,
            last_error: w.last_error,
            created_at: w.created_at.to_rfc3339(),
        }
    }
}

/// Response for webhook creation (includes secret).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookWithSecretResponse {
    #[serde(flatten)]
    pub webhook: WebhookResponse,
    pub secret: String,
}

/// Webhook payload for delivery.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookPayload {
    pub event: String,
    pub user_id: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

/// Webhook delivery job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDeliveryJob {
    pub webhook_id: String,
    pub url: String,
    pub secret: String,
    pub payload: String,
    pub retry_count: u32,
    pub max_retries: u32,
}

/// Maximum number of retries for webhook delivery.
const MAX_WEBHOOK_RETRIES: u32 = 5;

/// Maximum consecutive failures before disabling webhook.
const MAX_FAILURE_COUNT: i32 = 10;

/// Service for managing webhooks.
#[derive(Clone)]
pub struct WebhookService {
    webhook_repo: WebhookRepository,
    http_client: Arc<reqwest::Client>,
    id_gen: IdGenerator,
}

impl WebhookService {
    /// Create a new webhook service.
    #[must_use]
    #[allow(clippy::expect_used)] // Client build only fails with incompatible TLS settings
    pub fn new(webhook_repo: WebhookRepository) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            webhook_repo,
            http_client: Arc::new(http_client),
            id_gen: IdGenerator::new(),
        }
    }

    // ==================== Management ====================

    /// Create a new webhook.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateWebhookInput,
    ) -> AppResult<WebhookWithSecretResponse> {
        // Validate name
        if input.name.is_empty() || input.name.len() > 100 {
            return Err(AppError::Validation(
                "Name must be between 1 and 100 characters".to_string(),
            ));
        }

        // Validate URL
        if !input.url.starts_with("http://") && !input.url.starts_with("https://") {
            return Err(AppError::Validation(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        // Validate events
        if input.events.is_empty() {
            return Err(AppError::Validation(
                "At least one event must be specified".to_string(),
            ));
        }
        for event in &input.events {
            if !events::is_valid(event) {
                return Err(AppError::Validation(format!("Invalid event: {event}")));
            }
        }

        // Check limit
        if self.webhook_repo.user_at_limit(user_id).await? {
            return Err(AppError::Validation(
                "Maximum number of webhooks reached".to_string(),
            ));
        }

        // Generate secret
        let secret = self.generate_secret();

        let now = chrono::Utc::now();
        let id = self.id_gen.generate();

        let model = webhook::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            name: Set(input.name),
            url: Set(input.url),
            secret: Set(secret.clone()),
            events: Set(json!(input.events)),
            is_active: Set(true),
            last_triggered_at: Set(None),
            failure_count: Set(0),
            last_error: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        let webhook = self.webhook_repo.create(model).await?;

        Ok(WebhookWithSecretResponse {
            webhook: webhook.into(),
            secret,
        })
    }

    /// Update a webhook.
    pub async fn update(
        &self,
        user_id: &str,
        webhook_id: &str,
        input: UpdateWebhookInput,
    ) -> AppResult<WebhookResponse> {
        let webhook = self.webhook_repo.get_by_id(webhook_id).await?;

        // Verify ownership
        if webhook.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only update your own webhooks".to_string(),
            ));
        }

        let mut active: webhook::ActiveModel = webhook.into();

        if let Some(name) = input.name {
            if name.is_empty() || name.len() > 100 {
                return Err(AppError::Validation(
                    "Name must be between 1 and 100 characters".to_string(),
                ));
            }
            active.name = Set(name);
        }

        if let Some(url) = input.url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(AppError::Validation(
                    "URL must start with http:// or https://".to_string(),
                ));
            }
            active.url = Set(url);
        }

        if let Some(requested_events) = input.events {
            if requested_events.is_empty() {
                return Err(AppError::Validation(
                    "At least one event must be specified".to_string(),
                ));
            }
            for event in &requested_events {
                if !events::is_valid(event) {
                    return Err(AppError::Validation(format!("Invalid event: {event}")));
                }
            }
            active.events = Set(json!(requested_events));
        }

        if let Some(is_active) = input.is_active {
            active.is_active = Set(is_active);
            // Reset failure count when re-enabling
            if is_active {
                active.failure_count = Set(0);
                active.last_error = Set(None);
            }
        }

        active.updated_at = Set(Some(chrono::Utc::now().into()));

        let updated = self.webhook_repo.update(active).await?;
        Ok(updated.into())
    }

    /// Delete a webhook.
    pub async fn delete(&self, user_id: &str, webhook_id: &str) -> AppResult<()> {
        self.webhook_repo.delete(webhook_id, user_id).await
    }

    /// List webhooks for a user.
    pub async fn list(&self, user_id: &str) -> AppResult<Vec<WebhookResponse>> {
        let webhooks = self.webhook_repo.find_by_user_id(user_id).await?;
        Ok(webhooks.into_iter().map(Into::into).collect())
    }

    /// Get a webhook by ID.
    pub async fn get(&self, user_id: &str, webhook_id: &str) -> AppResult<WebhookResponse> {
        let webhook = self.webhook_repo.get_by_id(webhook_id).await?;

        // Verify ownership
        if webhook.user_id != user_id {
            return Err(AppError::NotFound(format!("Webhook: {webhook_id}")));
        }

        Ok(webhook.into())
    }

    /// Regenerate the secret for a webhook.
    pub async fn regenerate_secret(
        &self,
        user_id: &str,
        webhook_id: &str,
    ) -> AppResult<WebhookWithSecretResponse> {
        let webhook = self.webhook_repo.get_by_id(webhook_id).await?;

        // Verify ownership
        if webhook.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only modify your own webhooks".to_string(),
            ));
        }

        let new_secret = self.generate_secret();

        let mut active: webhook::ActiveModel = webhook.into();
        active.secret = Set(new_secret.clone());
        active.updated_at = Set(Some(chrono::Utc::now().into()));

        let updated = self.webhook_repo.update(active).await?;

        Ok(WebhookWithSecretResponse {
            webhook: updated.into(),
            secret: new_secret,
        })
    }

    // ==================== Delivery ====================

    /// Trigger webhooks for an event.
    pub async fn trigger(
        &self,
        user_id: &str,
        event: &str,
        data: serde_json::Value,
    ) -> AppResult<()> {
        let webhooks = self
            .webhook_repo
            .find_active_by_user_and_event(user_id, event)
            .await?;

        for webhook in webhooks {
            let payload = WebhookPayload {
                event: event.to_string(),
                user_id: user_id.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                data: data.clone(),
            };

            // Spawn async delivery with retry (don't block)
            let service = self.clone();
            let job = WebhookDeliveryJob {
                webhook_id: webhook.id.clone(),
                url: webhook.url.clone(),
                secret: webhook.secret.clone(),
                payload: serde_json::to_string(&payload).unwrap_or_default(),
                retry_count: 0,
                max_retries: MAX_WEBHOOK_RETRIES,
            };

            tokio::spawn(async move {
                let _ = service.deliver_with_retry(job).await;
            });
        }

        Ok(())
    }

    /// Deliver a webhook payload with retry logic.
    async fn deliver_with_retry(&self, mut job: WebhookDeliveryJob) -> AppResult<()> {
        loop {
            match self.deliver_once(&job).await {
                Ok(()) => {
                    // Success - record and exit
                    self.webhook_repo.record_success(&job.webhook_id).await?;
                    tracing::debug!(
                        webhook_id = %job.webhook_id,
                        url = %job.url,
                        "Webhook delivered successfully"
                    );
                    return Ok(());
                }
                Err(e) => {
                    job.retry_count += 1;

                    if job.retry_count > job.max_retries {
                        // Max retries reached - record failure
                        let error = format!("Max retries exceeded: {e}");
                        self.webhook_repo
                            .record_failure(&job.webhook_id, &error)
                            .await?;

                        // Check if webhook should be disabled
                        if let Ok(Some(webhook)) =
                            self.webhook_repo.find_by_id(&job.webhook_id).await
                            && webhook.failure_count >= MAX_FAILURE_COUNT
                        {
                            tracing::warn!(
                                webhook_id = %job.webhook_id,
                                failure_count = webhook.failure_count,
                                "Disabling webhook due to too many failures"
                            );
                            let _ = self.webhook_repo.disable(&job.webhook_id).await;
                        }

                        tracing::warn!(
                            webhook_id = %job.webhook_id,
                            url = %job.url,
                            error = %e,
                            "Webhook delivery failed after max retries"
                        );
                        return Err(e);
                    }

                    // Calculate backoff delay: 2^retry_count seconds (1, 2, 4, 8, 16...)
                    let delay_secs = 2u64.pow(job.retry_count);
                    tracing::debug!(
                        webhook_id = %job.webhook_id,
                        retry_count = job.retry_count,
                        delay_secs = delay_secs,
                        error = %e,
                        "Webhook delivery failed, retrying"
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }

    /// Attempt a single webhook delivery.
    async fn deliver_once(&self, job: &WebhookDeliveryJob) -> AppResult<()> {
        // Generate signature
        let signature = self.sign_payload(&job.payload, &job.secret);

        // Send request
        let response = self
            .http_client
            .post(&job.url)
            .header("Content-Type", "application/json")
            .header("X-Misskey-Signature", &signature)
            .header("User-Agent", "Misskey-Webhook/1.0")
            .body(job.payload.clone())
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Request failed: {e}")))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AppError::ExternalService(format!(
                "HTTP {}",
                response.status()
            )))
        }
    }

    /// Test a webhook by sending a test payload.
    pub async fn test(&self, user_id: &str, webhook_id: &str) -> AppResult<bool> {
        let webhook = self.webhook_repo.get_by_id(webhook_id).await?;

        // Verify ownership
        if webhook.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only test your own webhooks".to_string(),
            ));
        }

        let payload = WebhookPayload {
            event: "test".to_string(),
            user_id: user_id.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data: json!({ "message": "This is a test webhook delivery" }),
        };

        let payload_json = serde_json::to_string(&payload)
            .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {e}")))?;

        let signature = self.sign_payload(&payload_json, &webhook.secret);

        let result = self
            .http_client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Misskey-Signature", signature)
            .body(payload_json)
            .send()
            .await;

        match result {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    // ==================== Helper Methods ====================

    fn generate_secret(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    #[allow(clippy::expect_used)] // HMAC accepts any key size, this cannot fail
    fn sign_payload(&self, payload: &str, secret: &str) -> String {
        use hmac::{Hmac, Mac};

        type HmacSha256 = Hmac<Sha256>;

        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let result = mac.finalize();

        format!("sha256={}", hex::encode(result.into_bytes()))
    }
}
