//! Follow activity.

use activitypub_federation::kinds::activity::FollowType;
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Follow activity.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowActivity {
    #[serde(rename = "type")]
    pub kind: FollowType,
    pub id: Url,
    pub actor: Url,
    pub object: Url,
}

impl FollowActivity {
    /// Create a new Follow activity.
    #[must_use] 
    pub const fn new(id: Url, actor: Url, object: Url) -> Self {
        Self {
            kind: FollowType::Follow,
            id,
            actor,
            object,
        }
    }
}
