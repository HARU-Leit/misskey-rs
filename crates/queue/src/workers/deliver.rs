//! Deliver worker.

use apalis::prelude::*;
use chrono::Utc;
use misskey_common::{calculate_digest, crypto::parse_private_key, sign_request};
use misskey_db::repositories::UserKeypairRepository;
use reqwest::Client;
use std::collections::HashMap;
use tracing::{error, info, warn};
use url::Url;

use crate::jobs::DeliverJob;

/// Context for the deliver worker.
#[derive(Clone)]
pub struct DeliverContext {
    pub keypair_repo: UserKeypairRepository,
    pub http_client: Client,
    pub user_agent: String,
}

impl DeliverContext {
    /// Create a new deliver context.
    ///
    /// # Panics
    /// Panics if the HTTP client fails to build.
    #[must_use]
    #[allow(clippy::expect_used)] // Client build only fails with incompatible TLS settings
    pub fn new(keypair_repo: UserKeypairRepository, user_agent: String) -> Self {
        Self {
            keypair_repo,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            user_agent,
        }
    }
}

/// Worker function for delivering activities.
///
/// # Errors
/// Returns an error if the activity delivery fails.
pub async fn deliver_worker(job: DeliverJob, ctx: Data<DeliverContext>) -> Result<(), Error> {
    info!(
        user_id = %job.user_id,
        inbox = %job.inbox,
        "Delivering activity"
    );

    match deliver_activity(&job, &ctx).await {
        Ok(()) => {
            info!(inbox = %job.inbox, "Activity delivered successfully");
            Ok(())
        }
        Err(e) => {
            error!(inbox = %job.inbox, error = %e, "Failed to deliver activity");
            Err(Error::Failed(e.into()))
        }
    }
}

async fn deliver_activity(
    job: &DeliverJob,
    ctx: &DeliverContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get user's keypair
    let keypair = ctx
        .keypair_repo
        .get_by_user_id(&job.user_id)
        .await
        .map_err(|e| format!("Failed to get keypair: {e}"))?;

    // Parse inbox URL
    let inbox_url = Url::parse(&job.inbox)?;
    let host = inbox_url
        .host_str()
        .ok_or("Invalid inbox URL: no host")?
        .to_string();
    let path = inbox_url.path().to_string();

    // Serialize activity
    let body = serde_json::to_vec(&job.activity)?;

    // Calculate digest
    let digest = calculate_digest(&body);

    // Current date in HTTP format
    let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

    // Build headers for signing
    let mut headers = HashMap::new();
    headers.insert("host".to_string(), host.clone());
    headers.insert("date".to_string(), date.clone());
    headers.insert("digest".to_string(), digest.clone());

    // Parse private key and sign
    let private_key = parse_private_key(&keypair.private_key)?;
    let signature = sign_request(
        &private_key,
        &keypair.key_id,
        "POST",
        &path,
        &headers,
        &["(request-target)", "host", "date", "digest"],
    )?;

    // Send request
    let response = ctx
        .http_client
        .post(&job.inbox)
        .header("Host", host)
        .header("Date", date)
        .header("Digest", digest)
        .header("Signature", signature)
        .header("Content-Type", "application/activity+json")
        .header("Accept", "application/activity+json")
        .header("User-Agent", &ctx.user_agent)
        .body(body)
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(())
    } else if status.as_u16() == 410 {
        // Gone - remote actor deleted
        warn!(inbox = %job.inbox, "Remote actor gone (410)");
        Ok(())
    } else if status.is_client_error() {
        // Client error - don't retry
        let body = response.text().await.unwrap_or_default();
        Err(format!("Client error {status}: {body}").into())
    } else {
        // Server error - retry
        let body = response.text().await.unwrap_or_default();
        Err(format!("Server error {status}: {body}").into())
    }
}
