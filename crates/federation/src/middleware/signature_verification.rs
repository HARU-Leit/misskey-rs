//! HTTP Signature verification middleware for Authorized Fetch.
//!
//! This middleware verifies HTTP signatures on incoming requests to protected
//! ActivityPub resources. It supports per-user and per-instance security settings.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use tower::Layer;
use tracing::{debug, warn};

use crate::client::ApClient;
use crate::signature::HttpVerifier;

/// State required for signature verification.
#[derive(Clone)]
pub struct SignatureVerificationState {
    /// ActivityPub client for fetching actor public keys.
    pub ap_client: ApClient,
    /// Whether signature verification is globally required.
    pub require_signatures: bool,
}

impl SignatureVerificationState {
    /// Create a new signature verification state.
    #[must_use]
    pub const fn new(ap_client: ApClient, require_signatures: bool) -> Self {
        Self {
            ap_client,
            require_signatures,
        }
    }
}

/// Marker type indicating the request signature was verified.
///
/// Can be extracted in handlers via `Extension<SignatureVerified>` to confirm
/// the request was properly signed.
#[derive(Clone, Debug)]
pub struct SignatureVerified {
    /// The actor URL that signed this request.
    pub actor_url: Option<String>,
}

/// Layer for adding signature verification to routes.
#[derive(Clone)]
pub struct SignatureVerificationLayer {
    state: Arc<SignatureVerificationState>,
}

impl SignatureVerificationLayer {
    /// Create a new signature verification layer.
    #[must_use]
    pub fn new(state: SignatureVerificationState) -> Self {
        Self {
            state: Arc::new(state),
        }
    }
}

impl<S> Layer<S> for SignatureVerificationLayer {
    type Service = SignatureVerificationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SignatureVerificationService {
            inner,
            state: self.state.clone(),
        }
    }
}

/// Service that verifies HTTP signatures on requests.
#[derive(Clone)]
pub struct SignatureVerificationService<S> {
    inner: S,
    state: Arc<SignatureVerificationState>,
}

impl<S> tower::Service<Request<Body>> for SignatureVerificationService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        tower::Service::poll_ready(&mut self.inner, cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Check if signature verification is required
            if !state.require_signatures {
                // Add marker that verification was skipped (not required)
                req.extensions_mut()
                    .insert(SignatureVerified { actor_url: None });
                return tower::Service::call(&mut inner, req).await;
            }

            // Extract signature header
            let signature_header = req
                .headers()
                .get("signature")
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            let Some(signature_header) = signature_header else {
                warn!("Missing signature header on request requiring authorization");
                return Ok((StatusCode::UNAUTHORIZED, "HTTP signature required").into_response());
            };

            // Parse signature components
            let components = match HttpVerifier::parse_signature_header(&signature_header) {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Invalid signature header format");
                    return Ok(
                        (StatusCode::UNAUTHORIZED, "Invalid signature header format")
                            .into_response(),
                    );
                }
            };

            // Extract actor URL from key_id
            let actor_url = extract_actor_url(&components.key_id);

            // Fetch actor's public key
            let public_key_pem = match fetch_public_key(&state.ap_client, &components.key_id).await
            {
                Ok(key) => key,
                Err(e) => {
                    warn!(error = %e, key_id = %components.key_id, "Failed to fetch public key");
                    return Ok(
                        (StatusCode::UNAUTHORIZED, "Failed to fetch actor public key")
                            .into_response(),
                    );
                }
            };

            // Build headers map for verification
            let headers_map = build_headers_map(&req, &components.headers);

            // Extract method and path for verification
            let method = req.method().as_str();
            let path = req
                .uri()
                .path_and_query()
                .map_or_else(|| req.uri().path().to_string(), |pq| pq.to_string());

            // Verify the signature
            match HttpVerifier::verify(&public_key_pem, &components, method, &path, &headers_map) {
                Ok(true) => {
                    debug!(actor = ?actor_url, "Signature verified successfully");
                    req.extensions_mut().insert(SignatureVerified {
                        actor_url: actor_url.clone(),
                    });
                    tower::Service::call(&mut inner, req).await
                }
                Ok(false) => {
                    warn!(actor = ?actor_url, "Signature verification failed");
                    Ok((StatusCode::UNAUTHORIZED, "Signature verification failed").into_response())
                }
                Err(e) => {
                    warn!(error = %e, "Signature verification error");
                    Ok((StatusCode::UNAUTHORIZED, "Signature verification error").into_response())
                }
            }
        })
    }
}

/// Extract actor URL from key_id (removes #main-key fragment).
fn extract_actor_url(key_id: &str) -> Option<String> {
    key_id.split('#').next().map(String::from)
}

/// Fetch actor's public key from the key_id URL.
async fn fetch_public_key(ap_client: &ApClient, key_id: &str) -> Result<String, String> {
    // Extract actor URL
    let actor_url = extract_actor_url(key_id).ok_or_else(|| "Invalid key_id format".to_string())?;

    // Fetch actor
    let actor_json = ap_client
        .fetch_actor(&actor_url)
        .await
        .map_err(|e| format!("Failed to fetch actor: {e}"))?;

    // Extract public key
    let public_key = actor_json
        .get("publicKey")
        .ok_or_else(|| "Actor missing publicKey".to_string())?;

    let public_key_pem = public_key
        .get("publicKeyPem")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Actor missing publicKeyPem".to_string())?;

    Ok(public_key_pem.to_string())
}

/// Build headers map for signature verification.
fn build_headers_map(req: &Request<Body>, signed_headers: &[String]) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    for header_name in signed_headers {
        let value = if header_name == "(request-target)" {
            let method = req.method().as_str().to_lowercase();
            let path = req
                .uri()
                .path_and_query()
                .map_or_else(|| req.uri().path().to_string(), |pq| pq.to_string());
            format!("{method} {path}")
        } else if let Some(value) = req.headers().get(header_name.as_str()) {
            value.to_str().unwrap_or("").to_string()
        } else {
            continue;
        };

        headers.insert(header_name.clone(), value);
    }

    headers
}

/// Check if a user requires authorized fetch based on their profile settings.
///
/// This function can be used by handlers to check user-specific settings
/// and enforce signature verification accordingly.
pub fn user_requires_authorized_fetch(secure_fetch_only: bool) -> bool {
    secure_fetch_only
}

/// Check if an instance requires authorized fetch.
///
/// Returns true if the instance has require_authorized_fetch enabled.
pub fn instance_requires_authorized_fetch(require_authorized_fetch: bool) -> bool {
    require_authorized_fetch
}
