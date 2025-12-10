//! `ActivityPub` delivery job.

use serde::{Deserialize, Serialize};

/// Job to deliver an activity to a remote inbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverJob {
    /// The user ID sending the activity.
    pub user_id: String,

    /// Target inbox URL.
    pub inbox: String,

    /// Activity JSON to deliver.
    pub activity: serde_json::Value,
}

impl DeliverJob {
    /// Create a new deliver job.
    #[must_use] 
    pub const fn new(user_id: String, inbox: String, activity: serde_json::Value) -> Self {
        Self {
            user_id,
            inbox,
            activity,
        }
    }
}
