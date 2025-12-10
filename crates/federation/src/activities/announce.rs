//! Announce activity.

use activitypub_federation::kinds::activity::AnnounceType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Announce activity.
/// Used to share/boost a note (Renote in Misskey).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnounceActivity {
    #[serde(rename = "type")]
    pub kind: AnnounceType,
    pub id: Url,
    pub actor: Url,
    /// The object being announced (note URL).
    pub object: Url,
    pub published: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<Url>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<Url>>,
}

impl AnnounceActivity {
    /// Create a new Announce activity.
    #[must_use] 
    pub const fn new(id: Url, actor: Url, object: Url, published: DateTime<Utc>) -> Self {
        Self {
            kind: AnnounceType::Announce,
            id,
            actor,
            object,
            published,
            to: None,
            cc: None,
        }
    }

    /// Set the public audience.
    #[must_use]
    pub fn public(mut self) -> Self {
        self.to = Some(vec![Url::parse("https://www.w3.org/ns/activitystreams#Public").unwrap()]);
        self
    }
}
