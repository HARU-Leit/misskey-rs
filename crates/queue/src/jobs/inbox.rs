//! `ActivityPub` inbox processing job.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Job to process an incoming activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxJob {
    /// Activity JSON received.
    pub activity: serde_json::Value,

    /// HTTP signature header.
    pub signature: String,

    /// HTTP method used for the request.
    pub method: String,

    /// Request path (including query string).
    pub path: String,

    /// HTTP headers required for signature verification.
    pub headers: HashMap<String, String>,

    /// Request body digest.
    pub digest: Option<String>,
}

impl InboxJob {
    /// Create a new inbox job.
    #[must_use]
    pub const fn new(
        activity: serde_json::Value,
        signature: String,
        method: String,
        path: String,
        headers: HashMap<String, String>,
        digest: Option<String>,
    ) -> Self {
        Self {
            activity,
            signature,
            method,
            path,
            headers,
            digest,
        }
    }

    /// Create a simple inbox job (for testing or when signature verification is disabled).
    #[must_use]
    pub fn simple(activity: serde_json::Value, signature: String) -> Self {
        Self {
            activity,
            signature,
            method: "POST".to_string(),
            path: "/inbox".to_string(),
            headers: HashMap::new(),
            digest: None,
        }
    }
}
