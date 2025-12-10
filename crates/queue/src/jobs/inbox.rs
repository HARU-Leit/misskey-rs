//! `ActivityPub` inbox processing job.

use serde::{Deserialize, Serialize};

/// Job to process an incoming activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxJob {
    /// Activity JSON received.
    pub activity: serde_json::Value,

    /// HTTP signature header.
    pub signature: String,
}

impl InboxJob {
    /// Create a new inbox job.
    #[must_use]
    pub const fn new(activity: serde_json::Value, signature: String) -> Self {
        Self {
            activity,
            signature,
        }
    }
}
