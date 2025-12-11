//! Push notification service for Web Push.

use std::sync::Arc;

use chrono::Utc;
use misskey_db::entities::push_subscription;
use misskey_db::repositories::PushSubscriptionRepository;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushMessageBuilder,
};

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
    web_push_client: Arc<IsahcWebPushClient>,
}

impl PushNotificationService {
    /// Create a new push notification service.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be initialized.
    pub fn new(
        repo: PushSubscriptionRepository,
        vapid_config: Option<VapidConfig>,
    ) -> AppResult<Self> {
        let web_push_client = IsahcWebPushClient::new()
            .map_err(|e| AppError::Internal(format!("Failed to create Web Push client: {e}")))?;

        Ok(Self {
            repo,
            vapid_config,
            web_push_client: Arc::new(web_push_client),
        })
    }

    /// Check if push notifications are enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.vapid_config.is_some()
    }

    /// Returns the VAPID public key.
    #[must_use]
    pub fn public_key(&self) -> Option<&str> {
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

        // Serialize payload to JSON
        let payload_json = serde_json::to_vec(payload)
            .map_err(|e| AppError::Internal(format!("Failed to serialize payload: {e}")))?;

        // Build subscription info from stored data
        let subscription_info = SubscriptionInfo::new(
            &subscription.endpoint,
            &subscription.p256dh,
            &subscription.auth,
        );

        // Build VAPID signature
        let mut sig_builder = VapidSignatureBuilder::from_base64(
            &vapid.private_key,
            web_push::URL_SAFE_NO_PAD,
            &subscription_info,
        )
        .map_err(|e| {
            AppError::Internal(format!("Failed to create VAPID signature builder: {e}"))
        })?;

        sig_builder.add_claim("sub", vapid.subject.clone());

        let signature = sig_builder
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build VAPID signature: {e}")))?;

        // Build the Web Push message
        let mut message_builder = WebPushMessageBuilder::new(&subscription_info);
        message_builder.set_payload(ContentEncoding::Aes128Gcm, &payload_json);
        message_builder.set_vapid_signature(signature);
        message_builder.set_ttl(86400); // 24 hours TTL

        let message = message_builder
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build Web Push message: {e}")))?;

        // Send the push notification
        self.web_push_client.send(message).await.map_err(|e| {
            tracing::warn!(
                endpoint = %subscription.endpoint,
                error = %e,
                "Web Push send failed"
            );
            AppError::ExternalService(format!("Web Push send failed: {e}"))
        })?;

        tracing::debug!(
            endpoint = %subscription.endpoint,
            notification_type = %payload.notification_type,
            "Push notification sent successfully"
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
