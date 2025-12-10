//! Accept activity.

use activitypub_federation::kinds::activity::AcceptType;
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Accept activity.
/// Used to accept a Follow request.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptActivity {
    #[serde(rename = "type")]
    pub kind: AcceptType,
    pub id: Url,
    pub actor: Url,
    /// The original Follow activity being accepted.
    pub object: Url,
}

impl AcceptActivity {
    /// Create a new Accept activity.
    #[must_use] 
    pub const fn new(id: Url, actor: Url, object: Url) -> Self {
        Self {
            kind: AcceptType::Accept,
            id,
            actor,
            object,
        }
    }
}
