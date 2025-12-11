//! Delete activity.

use activitypub_federation::kinds::activity::DeleteType;
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Delete activity.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteActivity {
    #[serde(rename = "type")]
    pub kind: DeleteType,
    pub id: Url,
    pub actor: Url,
    pub object: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<Url>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<Url>>,
}

impl DeleteActivity {
    /// Create a new Delete activity.
    #[must_use]
    pub const fn new(id: Url, actor: Url, object: Url) -> Self {
        Self {
            kind: DeleteType::Delete,
            id,
            actor,
            object,
            to: None,
            cc: None,
        }
    }
}
