//! Undo activity.

use activitypub_federation::kinds::activity::UndoType;
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Undo activity.
/// Used to undo a previous activity (e.g., unfollow, unreact).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoActivity {
    #[serde(rename = "type")]
    pub kind: UndoType,
    pub id: Url,
    pub actor: Url,
    /// The activity being undone.
    pub object: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<Url>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<Url>>,
}

impl UndoActivity {
    /// Create a new Undo activity.
    #[must_use]
    pub const fn new(id: Url, actor: Url, object: Url) -> Self {
        Self {
            kind: UndoType::Undo,
            id,
            actor,
            object,
            to: None,
            cc: None,
        }
    }
}
