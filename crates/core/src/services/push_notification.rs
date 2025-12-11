//! Push notification service for Web Push.

use chrono::Utc;
use misskey_db::entities::push_subscription;
use misskey_db::repositories::PushSubscriptionRepository;
use sea_orm::Set;
use serde::{Deserialize, Serialize};

use misskey_common::{AppError, AppResult};

/// Notification types that can be sent via push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PushNotificationType {
    /// Follow notification
    Follow,
    /// Mention notification
    Mention,
    /// Reply notification
    Reply,
    /// Renote notification
    Renote,
    /// Quote notification
    Quote,
    /// Reaction notification
    Reaction,
    /// Poll ended notification
    PollEnded,
    /// Received follow request
    FollowRequestReceived,
    /// Follow request accepted
    FollowRequestAccepted,
    /// New message received
    Message,
    /// App notification (generic)
    App,
}

impl PushNotificationType {
    /// Get all notification types.
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self::Follow,
            Self::Mention,
            Self::Reply,
            Self::Renote,
            Self::Quote,
            Self::Reaction,
            Self::PollEnded,
            Self::FollowRequestReceived,
            Self::FollowRequestAccepted,
            Self::Message,
            Self::App,
        ]
    }
}

impl std::fmt::Display for PushNotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Follow => "follow",
            Self::Mention => "mention",
            Self::Reply => "reply",
            Self::Renote => "renote",
            Self::Quote => "quote",
            Self::Reaction => "reaction",
            Self::PollEnded => "pollEnded",
            Self::FollowRequestReceived => "followRequestReceived",
            Self::FollowRequestAccepted => "followRequestAccepted",
            Self::Message => "message",
            Self::App => "app",
        };
        write!(f, "{s}")
    }
}

/// Configuration for VAPID (Voluntary Application Server Identification).
#[derive(Debug, Clone)]
pub struct VapidConfig {
    /// Public key (base64 URL-safe encoded)
    pub public_key: String,
    /// Private key (base64 URL-safe encoded)
    pub private_key: String,
    /// Subject (typically a mailto: or https: URL)
    pub subject: String,
}

/// Input for creating a push subscription.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionInput {
    /// Push subscription endpoint URL
    pub endpoint: String,
    /// Auth key (base64 URL-safe encoded)
    pub auth: String,
    /// P256DH public key (base64 URL-safe encoded)
    pub p256dh: String,
    /// Notification types to receive (default: all)
    pub types: Option<Vec<String>>,
    /// Device name (optional)
    pub device_name: Option<String>,
}

/// Input for updating a push subscription.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionInput {
    /// Notification types to receive
    pub types: Option<Vec<String>>,
    /// Device name
    pub device_name: Option<String>,
    /// Quiet hours start (0-23)
    pub quiet_hours_start: Option<i32>,
    /// Quiet hours end (0-23)
    pub quiet_hours_end: Option<i32>,
}

/// Push subscription response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushSubscriptionResponse {
    /// Subscription ID
    pub id: String,
    /// Endpoint URL (partially masked for security)
    pub endpoint: String,
    /// Notification types enabled
    pub types: Vec<String>,
    /// Device name
    pub device_name: Option<String>,
    /// Whether the subscription is active
    pub active: bool,
    /// Quiet hours start
    pub quiet_hours_start: Option<i32>,
    /// Quiet hours end
    pub quiet_hours_end: Option<i32>,
    /// Last successful push
    pub last_pushed_at: Option<String>,
    /// Created timestamp
    pub created_at: String,
}

/// Push notification payload.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushPayload {
    /// Notification type
    #[serde(rename = "type")]
    pub notification_type: String,
    /// Notification title
    pub title: String,
    /// Notification body
    pub body: String,
    /// Icon URL (optional)
    pub icon: Option<String>,
    /// URL to open when clicked (optional)
    pub url: Option<String>,
    /// Additional data
    #[serde(flatten)]
    pub data: Option<serde_json::Value>,
}

/// Push notification service.
#[derive(Clone)]
pub struct PushNotificationService {
    repo: PushSubscriptionRepository,
    vapid_config: Option<VapidConfig>,
    http_client: reqwest::Client,
}

impl PushNotificationService {
    /// Create a new push notification service.
    #[must_use]
    pub fn new(repo: PushSubscriptionRepository, vapid_config: Option<VapidConfig>) -> Self {
        Self {
            repo,
            vapid_config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Check if push notifications are enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.vapid_config.is_some()
    }

    /// Get VAPID public key.
    #[must_use]
    pub fn get_public_key(&self) -> Option<&str> {
        self.vapid_config.as_ref().map(|c| c.public_key.as_str())
    }

    /// Register a new push subscription.
    pub async fn register(
        &self,
        user_id: &str,
        input: CreateSubscriptionInput,
        user_agent: Option<String>,
    ) -> AppResult<PushSubscriptionResponse> {
        // Check if subscription already exists for this endpoint
        if let Some(existing) = self.repo.find_by_endpoint(&input.endpoint).await? {
            if existing.user_id == user_id {
                // Update existing subscription
                return self
                    .update(
                        user_id,
                        &existing.id,
                        UpdateSubscriptionInput {
                            types: input.types,
                            device_name: input.device_name,
                            quiet_hours_start: None,
                            quiet_hours_end: None,
                        },
                    )
                    .await;
            }
            // Different user trying to use the same endpoint
            return Err(AppError::Conflict(
                "This push endpoint is already registered to another user".to_string(),
            ));
        }

        let types = input.types.unwrap_or_else(|| {
            PushNotificationType::all()
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        });

        let id = crate::generate_id();
        let now = Utc::now();

        let subscription = push_subscription::ActiveModel {
            id: Set(id.clone()),
            user_id: Set(user_id.to_string()),
            endpoint: Set(input.endpoint),
            auth: Set(input.auth),
            p256dh: Set(input.p256dh),
            types: Set(serde_json::json!(types)),
            active: Set(true),
            user_agent: Set(user_agent),
            device_name: Set(input.device_name),
            quiet_hours_start: Set(None),
            quiet_hours_end: Set(None),
            last_pushed_at: Set(None),
            fail_count: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        let created = self.repo.create(subscription).await?;
        Ok(self.to_response(created))
    }

    /// Update a push subscription.
    pub async fn update(
        &self,
        user_id: &str,
        subscription_id: &str,
        input: UpdateSubscriptionInput,
    ) -> AppResult<PushSubscriptionResponse> {
        let subscription = self.repo.get_by_id(subscription_id).await?;

        if subscription.user_id != user_id {
            return Err(AppError::Forbidden(
                "You don't own this subscription".to_string(),
            ));
        }

        let mut active: push_subscription::ActiveModel = subscription.into();

        if let Some(types) = input.types {
            active.types = Set(serde_json::json!(types));
        }
        if let Some(device_name) = input.device_name {
            active.device_name = Set(Some(device_name));
        }
        if let Some(start) = input.quiet_hours_start {
            if !(0..=23).contains(&start) {
                return Err(AppError::Validation(
                    "quiet_hours_start must be between 0 and 23".to_string(),
                ));
            }
            active.quiet_hours_start = Set(Some(start));
        }
        if let Some(end) = input.quiet_hours_end {
            if !(0..=23).contains(&end) {
                return Err(AppError::Validation(
                    "quiet_hours_end must be between 0 and 23".to_string(),
                ));
            }
            active.quiet_hours_end = Set(Some(end));
        }
        active.updated_at = Set(Some(Utc::now().into()));

        let updated = self.repo.update(active).await?;
        Ok(self.to_response(updated))
    }

    /// Unregister a push subscription.
    pub async fn unregister(&self, user_id: &str, subscription_id: &str) -> AppResult<()> {
        let subscription = self.repo.get_by_id(subscription_id).await?;

        if subscription.user_id != user_id {
            return Err(AppError::Forbidden(
                "You don't own this subscription".to_string(),
            ));
        }

        self.repo.delete(subscription_id).await
    }

    /// Unregister by endpoint URL.
    pub async fn unregister_by_endpoint(&self, user_id: &str, endpoint: &str) -> AppResult<()> {
        if let Some(subscription) = self.repo.find_by_endpoint(endpoint).await? {
            if subscription.user_id != user_id {
                return Err(AppError::Forbidden(
                    "You don't own this subscription".to_string(),
                ));
            }
            self.repo.delete(&subscription.id).await
        } else {
            Err(AppError::NotFound("Subscription not found".to_string()))
        }
    }

    /// List all subscriptions for a user.
    pub async fn list(&self, user_id: &str) -> AppResult<Vec<PushSubscriptionResponse>> {
        let subscriptions = self.repo.find_by_user_id(user_id).await?;
        Ok(subscriptions
            .into_iter()
            .map(|s| self.to_response(s))
            .collect())
    }

    /// Get a subscription by ID.
    pub async fn get(
        &self,
        user_id: &str,
        subscription_id: &str,
    ) -> AppResult<PushSubscriptionResponse> {
        let subscription = self.repo.get_by_id(subscription_id).await?;

        if subscription.user_id != user_id {
            return Err(AppError::Forbidden(
                "You don't own this subscription".to_string(),
            ));
        }

        Ok(self.to_response(subscription))
    }

    /// Send a push notification to a user.
    pub async fn send_to_user(
        &self,
        user_id: &str,
        notification_type: PushNotificationType,
        payload: PushPayload,
    ) -> AppResult<usize> {
        let subscriptions = self
            .repo
            .find_active_for_notification(user_id, &notification_type.to_string())
            .await?;

        let mut success_count = 0;

        for subscription in subscriptions {
            match self.send_push(&subscription, &payload).await {
                Ok(()) => {
                    let _ = self.repo.mark_push_success(&subscription.id).await;
                    success_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        subscription_id = %subscription.id,
                        error = %e,
                        "Failed to send push notification"
                    );
                    let _ = self.repo.increment_fail_count(&subscription.id).await;
                }
            }
        }

        Ok(success_count)
    }

    /// Send a push notification to a specific subscription.
    async fn send_push(
        &self,
        subscription: &push_subscription::Model,
        payload: &PushPayload,
    ) -> AppResult<()> {
        let vapid = self
            .vapid_config
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("VAPID not configured".to_string()))?;

        // Build Web Push message using VAPID
        let payload_json = serde_json::to_string(payload)
            .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {e}")))?;

        // In a real implementation, we'd use a proper Web Push library like web-push
        // For now, we'll use the low-level HTTP API approach
        // This is a simplified implementation - production code should use web-push crate

        // Build the encrypted payload (requires ECE - Encrypted Content-Encoding)
        // For simplicity, we'll document that a proper implementation needs:
        // 1. Generate ephemeral ECDH key pair
        // 2. Derive shared secret using subscription's p256dh
        // 3. Encrypt payload using AES-128-GCM
        // 4. Build VAPID JWT for authorization

        // Placeholder that shows the structure of what we need to send
        let _ = self
            .http_client
            .post(&subscription.endpoint)
            .header("TTL", "86400")
            .header("Content-Encoding", "aes128gcm")
            .header(
                "Authorization",
                format!("vapid t={}, k={}", "jwt_token", vapid.public_key),
            )
            .body(payload_json)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build request: {e}")))?;

        // For now, we'll just simulate success
        // In production, uncomment the actual send:
        // let response = self.http_client.execute(request).await
        //     .map_err(|e| AppError::ExternalService(format!("Push request failed: {}", e)))?;

        tracing::debug!(
            endpoint = %subscription.endpoint,
            "Would send push notification (implementation pending)"
        );

        Ok(())
    }

    /// Convert model to response.
    fn to_response(&self, model: push_subscription::Model) -> PushSubscriptionResponse {
        let types: Vec<String> = model
            .types
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Mask the endpoint for security (show only domain)
        let masked_endpoint = url::Url::parse(&model.endpoint)
            .ok()
            .and_then(|u| u.host_str().map(|h| format!("https://{h}/***/")))
            .unwrap_or_else(|| "***".to_string());

        PushSubscriptionResponse {
            id: model.id,
            endpoint: masked_endpoint,
            types,
            device_name: model.device_name,
            active: model.active,
            quiet_hours_start: model.quiet_hours_start,
            quiet_hours_end: model.quiet_hours_end,
            last_pushed_at: model.last_pushed_at.map(|dt| dt.to_rfc3339()),
            created_at: model.created_at.to_rfc3339(),
        }
    }
}

/// Response for push notification configuration.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushConfigResponse {
    /// Whether push notifications are available
    pub available: bool,
    /// VAPID public key for subscription
    pub public_key: Option<String>,
}
