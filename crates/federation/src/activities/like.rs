//! Like activity (reaction).

use activitypub_federation::kinds::activity::LikeType;
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Like activity.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LikeActivity {
    #[serde(rename = "type")]
    pub kind: LikeType,
    pub id: Url,
    pub actor: Url,
    pub object: Url,

    /// Misskey reaction content (emoji).
    #[serde(rename = "_misskey_reaction", skip_serializing_if = "Option::is_none")]
    pub misskey_reaction: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

impl LikeActivity {
    /// Create a new Like activity.
    #[must_use] 
    pub const fn new(id: Url, actor: Url, object: Url) -> Self {
        Self {
            kind: LikeType::Like,
            id,
            actor,
            object,
            misskey_reaction: None,
            content: None,
        }
    }

    /// Set reaction content.
    #[must_use] 
    pub fn with_reaction(mut self, reaction: String) -> Self {
        self.misskey_reaction = Some(reaction.clone());
        self.content = Some(reaction);
        self
    }
}
