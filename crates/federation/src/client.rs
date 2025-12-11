//! `ActivityPub` HTTP client for delivering activities.
//!
//! Handles sending signed HTTP requests to remote `ActivityPub` inboxes.

#![allow(missing_docs)]

use crate::signature::HttpSigner;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use url::Url;

/// Error type for AP client operations.
#[derive(Debug, thiserror::Error)]
pub enum ApClientError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Signing failed: {0}")]
    SigningError(#[from] crate::signature::SignatureError),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Delivery failed: {status} - {body}")]
    DeliveryFailed { status: u16, body: String },
}

/// `ActivityPub` HTTP client for delivering activities.
#[derive(Clone)]
pub struct ApClient {
    client: Client,
    user_agent: String,
}

impl ApClient {
    /// Create a new AP client.
    #[must_use] 
    pub fn new(instance_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        let user_agent = format!(
            "misskey-rs/0.1.0 (+{instance_url})"
        );

        Self { client, user_agent }
    }

    /// Deliver an activity to a remote inbox.
    pub async fn deliver(
        &self,
        inbox_url: &str,
        activity: &Value,
        private_key_pem: &str,
        key_id: &str,
    ) -> Result<(), ApClientError> {
        let url = Url::parse(inbox_url)
            .map_err(|e| ApClientError::InvalidUrl(e.to_string()))?;

        let body = serde_json::to_vec(activity).unwrap();

        // Create signer
        let signer = HttpSigner::new(private_key_pem, key_id.to_string())?;

        // Add content-type header
        let mut additional_headers = HashMap::new();
        additional_headers.insert(
            "content-type".to_string(),
            "application/activity+json".to_string(),
        );

        // Sign the request
        let headers = signer.sign_request("POST", &url, Some(&body), &additional_headers)?;

        debug!(
            inbox = %inbox_url,
            activity_type = activity.get("type").and_then(|v| v.as_str()).unwrap_or("Unknown"),
            "Delivering activity"
        );

        // Send the request
        let response = self
            .client
            .post(inbox_url)
            .headers(headers)
            .header("User-Agent", &self.user_agent)
            .header("Content-Type", "application/activity+json")
            .header("Accept", "application/activity+json, application/ld+json")
            .body(body)
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            info!(inbox = %inbox_url, status = %status, "Activity delivered successfully");
            Ok(())
        } else if status.as_u16() == 202 {
            // 202 Accepted is also a success for async processing
            info!(inbox = %inbox_url, status = %status, "Activity accepted for processing");
            Ok(())
        } else if status.as_u16() == 410 {
            // 410 Gone - actor has been deleted, should stop delivering
            warn!(inbox = %inbox_url, "Remote actor is gone (410)");
            Ok(()) // Don't retry
        } else {
            let body = response.text().await.unwrap_or_default();
            error!(
                inbox = %inbox_url,
                status = %status,
                body = %body,
                "Activity delivery failed"
            );
            Err(ApClientError::DeliveryFailed {
                status: status.as_u16(),
                body,
            })
        }
    }

    /// Fetch a remote actor by their ID URL.
    pub async fn fetch_actor(&self, actor_url: &str) -> Result<Value, ApClientError> {
        debug!(actor_url = %actor_url, "Fetching remote actor");

        let response = self
            .client
            .get(actor_url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/activity+json, application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"")
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let actor: Value = response.json().await?;
            Ok(actor)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ApClientError::DeliveryFailed {
                status: status.as_u16(),
                body,
            })
        }
    }

    /// Fetch a remote object (note, activity, etc.) by its ID URL.
    pub async fn fetch_object(&self, object_url: &str) -> Result<Value, ApClientError> {
        debug!(object_url = %object_url, "Fetching remote object");

        let response = self
            .client
            .get(object_url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/activity+json, application/ld+json")
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let object: Value = response.json().await?;
            Ok(object)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ApClientError::DeliveryFailed {
                status: status.as_u16(),
                body,
            })
        }
    }

    /// Perform `WebFinger` lookup for a user.
    pub async fn webfinger(&self, acct: &str, domain: &str) -> Result<Value, ApClientError> {
        let url = format!(
            "https://{domain}/.well-known/webfinger?resource=acct:{acct}"
        );

        debug!(acct = %acct, domain = %domain, "Performing WebFinger lookup");

        let response = self
            .client
            .get(&url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/jrd+json, application/json")
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let result: Value = response.json().await?;
            Ok(result)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ApClientError::DeliveryFailed {
                status: status.as_u16(),
                body,
            })
        }
    }
}

impl Default for ApClient {
    fn default() -> Self {
        Self::new("https://localhost")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ApClient::new("https://example.com");
        assert!(client.user_agent.contains("misskey-rs"));
    }
}
