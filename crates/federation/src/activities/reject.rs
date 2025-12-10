//! Reject activity.

use activitypub_federation::kinds::activity::RejectType;
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Reject activity.
/// Used to reject a Follow request.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectActivity {
    #[serde(rename = "type")]
    pub kind: RejectType,
    pub id: Url,
    pub actor: Url,
    /// The original Follow activity being rejected.
    pub object: Url,
}

impl RejectActivity {
    /// Create a new Reject activity.
    #[must_use] 
    pub const fn new(id: Url, actor: Url, object: Url) -> Self {
        Self {
            kind: RejectType::Reject,
            id,
            actor,
            object,
        }
    }
}
