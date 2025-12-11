//! Create activity.

use activitypub_federation::kinds::activity::CreateType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::objects::ApNote;

/// `ActivityPub` Create activity.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActivity {
    #[serde(rename = "type")]
    pub kind: CreateType,
    pub id: Url,
    pub actor: Url,
    pub object: ApNote,
    pub published: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<Url>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<Url>>,
}

impl CreateActivity {
    /// Create a new Create activity.
    #[must_use]
    pub const fn new(id: Url, actor: Url, object: ApNote, published: DateTime<Utc>) -> Self {
        Self {
            kind: CreateType::Create,
            id,
            actor,
            object,
            published,
            to: None,
            cc: None,
        }
    }
}
