//! EmojiReact activity (Pleroma/Akkoma style emoji reactions).
//!
//! This activity type is used by Pleroma/Akkoma to send emoji reactions.
//! See: <https://docs.akkoma.dev/stable/development/ap_extensions/#emoji-reactions>

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use url::Url;

/// Custom type for `EmojiReact` activity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmojiReactType;

impl Default for EmojiReactType {
    fn default() -> Self {
        Self
    }
}

/// Serialize `EmojiReactType` as the string "EmojiReact".
impl Serialize for EmojiReactType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("EmojiReact")
    }
}

/// Deserialize "EmojiReact" string into `EmojiReactType`.
impl<'de> Deserialize<'de> for EmojiReactType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "EmojiReact" {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "expected 'EmojiReact', got '{s}'"
            )))
        }
    }
}

/// `ActivityPub` EmojiReact activity (Pleroma/Akkoma extension).
///
/// This is used for emoji reactions that carry a specific emoji/content.
/// Mastodon does not support this; it only uses Like for favorites.
///
/// # Example
///
/// ```json
/// {
///   "@context": "https://www.w3.org/ns/activitystreams",
///   "type": "EmojiReact",
///   "id": "https://example.com/activities/react/123",
///   "actor": "https://example.com/users/alice",
///   "object": "https://remote.example/notes/456",
///   "content": "👍"
/// }
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiReactActivity {
    #[serde(rename = "type")]
    pub kind: EmojiReactType,

    /// The activity's unique identifier.
    pub id: Url,

    /// The actor performing the reaction.
    pub actor: Url,

    /// The object (note) being reacted to.
    pub object: Url,

    /// The emoji content of the reaction.
    pub content: String,

    /// Optional tag array for custom emoji definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<Vec<EmojiTag>>,
}

/// Tag structure for custom emoji definitions in `EmojiReact`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiTag {
    /// Type of tag (usually "Emoji").
    #[serde(rename = "type")]
    pub kind: String,

    /// The emoji shortcode (e.g., ":blobcat:").
    pub name: String,

    /// Icon/image for the custom emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<EmojiIcon>,
}

/// Icon structure for custom emoji.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiIcon {
    /// Type (usually "Image").
    #[serde(rename = "type")]
    pub kind: String,

    /// URL of the emoji image.
    pub url: Url,

    /// Media type of the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

impl EmojiReactActivity {
    /// Create a new `EmojiReact` activity.
    #[must_use]
    pub fn new(id: Url, actor: Url, object: Url, content: String) -> Self {
        Self {
            kind: EmojiReactType,
            id,
            actor,
            object,
            content,
            tag: None,
        }
    }

    /// Add custom emoji tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<EmojiTag>) -> Self {
        self.tag = Some(tags);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_react_serialization() {
        let activity = EmojiReactActivity::new(
            Url::parse("https://example.com/activities/react/123").unwrap(),
            Url::parse("https://example.com/users/alice").unwrap(),
            Url::parse("https://remote.example/notes/456").unwrap(),
            "👍".to_string(),
        );

        let json = serde_json::to_value(&activity).unwrap();
        assert_eq!(json["type"], "EmojiReact");
        assert_eq!(json["content"], "👍");
    }

    #[test]
    fn test_emoji_react_deserialization() {
        let json = r#"{
            "type": "EmojiReact",
            "id": "https://example.com/activities/react/123",
            "actor": "https://example.com/users/alice",
            "object": "https://remote.example/notes/456",
            "content": "🎉"
        }"#;

        let activity: EmojiReactActivity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.content, "🎉");
        assert_eq!(activity.actor.as_str(), "https://example.com/users/alice");
    }

    #[test]
    fn test_emoji_react_with_custom_emoji() {
        let tag = EmojiTag {
            kind: "Emoji".to_string(),
            name: ":blobcat:".to_string(),
            icon: Some(EmojiIcon {
                kind: "Image".to_string(),
                url: Url::parse("https://example.com/emoji/blobcat.png").unwrap(),
                media_type: Some("image/png".to_string()),
            }),
        };

        let activity = EmojiReactActivity::new(
            Url::parse("https://example.com/activities/react/123").unwrap(),
            Url::parse("https://example.com/users/alice").unwrap(),
            Url::parse("https://remote.example/notes/456").unwrap(),
            ":blobcat:".to_string(),
        )
        .with_tags(vec![tag]);

        let json = serde_json::to_value(&activity).unwrap();
        assert!(json["tag"].is_array());
        assert_eq!(json["tag"][0]["name"], ":blobcat:");
    }
}
