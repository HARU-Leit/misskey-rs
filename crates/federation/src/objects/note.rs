//! `ActivityPub` Note and Question objects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

/// Object type for notes and questions.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ApObjectType {
    Note,
    Question,
}

/// `ActivityPub` Note object.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApNote {
    #[serde(rename = "type")]
    pub kind: ApObjectType,
    pub id: Url,
    pub attributed_to: Url,
    pub content: String,
    pub published: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<Url>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<Url>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<Vec<ApTag>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<Vec<ApAttachment>>,

    // Question/Poll fields (when type = Question)
    /// Single choice poll options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<ApPollOption>>,

    /// Multiple choice poll options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<ApPollOption>>,

    /// Poll end time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,

    /// Poll closed time (set when poll is closed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<DateTime<Utc>>,

    /// Total number of voters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voters_count: Option<u32>,

    // FEP-c16b: Quote Posts
    // https://codeberg.org/fediverse/fep/src/branch/main/fep/c16b/fep-c16b.md
    #[serde(rename = "quoteUrl", skip_serializing_if = "Option::is_none")]
    pub quote_url: Option<Url>,

    // Also support quoteUri for compatibility with some implementations
    #[serde(rename = "quoteUri", skip_serializing_if = "Option::is_none")]
    pub quote_uri: Option<Url>,

    // Misskey extensions
    #[serde(rename = "_misskey_quote", skip_serializing_if = "Option::is_none")]
    pub misskey_quote: Option<Url>,

    #[serde(rename = "_misskey_content", skip_serializing_if = "Option::is_none")]
    pub misskey_content: Option<String>,

    #[serde(rename = "_misskey_summary", skip_serializing_if = "Option::is_none")]
    pub misskey_summary: Option<String>,

    #[serde(rename = "_misskey_reaction", skip_serializing_if = "Option::is_none")]
    pub misskey_reaction: Option<String>,
}

/// `ActivityPub` poll option (for Question objects).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApPollOption {
    #[serde(rename = "type")]
    pub kind: String, // "Note"
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<ApPollReplies>,
}

/// Poll option vote count.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApPollReplies {
    #[serde(rename = "type")]
    pub kind: String, // "Collection"
    pub total_items: u32,
}

impl ApPollOption {
    /// Create a new poll option.
    #[must_use] 
    pub fn new(name: String, votes: u32) -> Self {
        Self {
            kind: "Note".to_string(),
            name,
            replies: Some(ApPollReplies {
                kind: "Collection".to_string(),
                total_items: votes,
            }),
        }
    }
}

/// `ActivityPub` tag (mention or hashtag).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApTag {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `ActivityPub` attachment (file).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApAttachment {
    #[serde(rename = "type")]
    pub kind: String,
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurhash: Option<String>,
}

impl ApNote {
    /// Create a new Note object.
    #[must_use] 
    pub const fn new(id: Url, attributed_to: Url, content: String, published: DateTime<Utc>) -> Self {
        Self {
            kind: ApObjectType::Note,
            id,
            attributed_to,
            content,
            published,
            to: None,
            cc: None,
            in_reply_to: None,
            summary: None,
            sensitive: None,
            tag: None,
            attachment: None,
            one_of: None,
            any_of: None,
            end_time: None,
            closed: None,
            voters_count: None,
            quote_url: None,
            quote_uri: None,
            misskey_quote: None,
            misskey_content: None,
            misskey_summary: None,
            misskey_reaction: None,
        }
    }

    /// Create a new Question (poll) object.
    #[must_use] 
    pub fn new_question(
        id: Url,
        attributed_to: Url,
        content: String,
        published: DateTime<Utc>,
        options: Vec<String>,
        multiple_choice: bool,
        end_time: Option<DateTime<Utc>>,
    ) -> Self {
        let poll_options: Vec<ApPollOption> = options
            .into_iter()
            .map(|name| ApPollOption::new(name, 0))
            .collect();

        Self {
            kind: ApObjectType::Question,
            id,
            attributed_to,
            content,
            published,
            to: None,
            cc: None,
            in_reply_to: None,
            summary: None,
            sensitive: None,
            tag: None,
            attachment: None,
            one_of: if multiple_choice { None } else { Some(poll_options.clone()) },
            any_of: if multiple_choice { Some(poll_options) } else { None },
            end_time,
            closed: None,
            voters_count: Some(0),
            quote_url: None,
            quote_uri: None,
            misskey_quote: None,
            misskey_content: None,
            misskey_summary: None,
            misskey_reaction: None,
        }
    }

    /// Set the quoted post URL (FEP-c16b compliant).
    /// Sets both standard `quoteUrl` and Misskey's `_misskey_quote` for compatibility.
    #[must_use] 
    pub fn with_quote(mut self, quote_url: Url) -> Self {
        self.quote_url = Some(quote_url.clone());
        self.misskey_quote = Some(quote_url);
        self
    }

    /// Get the quote URL, checking multiple fields for compatibility.
    #[must_use] 
    pub fn get_quote_url(&self) -> Option<&Url> {
        self.quote_url
            .as_ref()
            .or(self.quote_uri.as_ref())
            .or(self.misskey_quote.as_ref())
    }

    /// Check if this is a Question (poll).
    #[must_use] 
    pub fn is_question(&self) -> bool {
        self.kind == ApObjectType::Question
    }

    /// Set public addressing.
    #[must_use] 
    pub fn public(mut self) -> Self {
        self.to = Some(vec!["https://www.w3.org/ns/activitystreams#Public"
            .parse()
            .unwrap()]);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_url(path: &str) -> Url {
        Url::parse(&format!("https://example.com{path}")).unwrap()
    }

    #[test]
    fn test_note_serialization() {
        let note = ApNote::new(
            test_url("/notes/123"),
            test_url("/users/alice"),
            "Hello, world!".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_string(&note).unwrap();
        assert!(json.contains("\"type\":\"Note\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));
        assert!(json.contains("\"attributedTo\":\"https://example.com/users/alice\""));
    }

    #[test]
    fn test_note_deserialization() {
        let json = r#"{
            "type": "Note",
            "id": "https://example.com/notes/123",
            "attributedTo": "https://example.com/users/alice",
            "content": "Hello, world!",
            "published": "2025-01-01T00:00:00Z"
        }"#;

        let note: ApNote = serde_json::from_str(json).unwrap();
        assert_eq!(note.kind, ApObjectType::Note);
        assert_eq!(note.content, "Hello, world!");
    }

    #[test]
    fn test_fep_c16b_quote_url() {
        let note = ApNote::new(
            test_url("/notes/123"),
            test_url("/users/alice"),
            "Check this out!".to_string(),
            Utc::now(),
        )
        .with_quote(test_url("/notes/original"));

        let json = serde_json::to_string(&note).unwrap();
        // FEP-c16b: should have quoteUrl
        assert!(json.contains("\"quoteUrl\":\"https://example.com/notes/original\""));
        // Misskey compatibility: should also have _misskey_quote
        assert!(json.contains("\"_misskey_quote\":\"https://example.com/notes/original\""));
    }

    #[test]
    fn test_fep_c16b_quote_url_parsing() {
        // Test parsing quoteUrl (standard FEP-c16b)
        let json = r#"{
            "type": "Note",
            "id": "https://mastodon.social/notes/456",
            "attributedTo": "https://mastodon.social/users/bob",
            "content": "Nice quote!",
            "published": "2025-01-01T00:00:00Z",
            "quoteUrl": "https://example.com/notes/original"
        }"#;

        let note: ApNote = serde_json::from_str(json).unwrap();
        assert!(note.quote_url.is_some());
        assert_eq!(
            note.get_quote_url().unwrap().as_str(),
            "https://example.com/notes/original"
        );
    }

    #[test]
    fn test_misskey_quote_parsing() {
        // Test parsing _misskey_quote (Misskey compatibility)
        let json = r#"{
            "type": "Note",
            "id": "https://misskey.io/notes/456",
            "attributedTo": "https://misskey.io/users/bob",
            "content": "Nice quote!",
            "published": "2025-01-01T00:00:00Z",
            "_misskey_quote": "https://example.com/notes/original"
        }"#;

        let note: ApNote = serde_json::from_str(json).unwrap();
        assert!(note.misskey_quote.is_some());
        assert_eq!(
            note.get_quote_url().unwrap().as_str(),
            "https://example.com/notes/original"
        );
    }

    #[test]
    fn test_question_serialization() {
        let question = ApNote::new_question(
            test_url("/notes/poll123"),
            test_url("/users/alice"),
            "What's your favorite color?".to_string(),
            Utc::now(),
            vec!["Red".to_string(), "Blue".to_string(), "Green".to_string()],
            false, // single choice
            None,
        );

        let json = serde_json::to_string(&question).unwrap();
        assert!(json.contains("\"type\":\"Question\""));
        assert!(json.contains("\"oneOf\""));
        assert!(json.contains("\"Red\""));
        assert!(json.contains("\"Blue\""));
        assert!(json.contains("\"Green\""));
    }

    #[test]
    fn test_question_multiple_choice() {
        let question = ApNote::new_question(
            test_url("/notes/poll456"),
            test_url("/users/alice"),
            "Select all that apply:".to_string(),
            Utc::now(),
            vec!["A".to_string(), "B".to_string()],
            true, // multiple choice
            None,
        );

        let json = serde_json::to_string(&question).unwrap();
        assert!(json.contains("\"anyOf\""));
        assert!(!json.contains("\"oneOf\""));
    }

    #[test]
    fn test_tag_serialization() {
        let tag = ApTag {
            kind: "Hashtag".to_string(),
            href: Some(test_url("/tags/rust")),
            name: Some("#rust".to_string()),
        };

        let json = serde_json::to_string(&tag).unwrap();
        assert!(json.contains("\"type\":\"Hashtag\""));
        assert!(json.contains("\"name\":\"#rust\""));
    }

    #[test]
    fn test_mention_tag() {
        let mention = ApTag {
            kind: "Mention".to_string(),
            href: Some(test_url("/users/alice")),
            name: Some("@alice".to_string()),
        };

        let json = serde_json::to_string(&mention).unwrap();
        assert!(json.contains("\"type\":\"Mention\""));
        assert!(json.contains("\"href\":\"https://example.com/users/alice\""));
    }

    #[test]
    fn test_attachment_serialization() {
        let attachment = ApAttachment {
            kind: "Document".to_string(),
            url: test_url("/files/image.png"),
            media_type: Some("image/png".to_string()),
            name: Some("My image".to_string()),
            width: Some(800),
            height: Some(600),
            blurhash: Some("LEHV6nWB2yk8pyo0adR*.7kCMdnj".to_string()),
        };

        let json = serde_json::to_string(&attachment).unwrap();
        assert!(json.contains("\"type\":\"Document\""));
        assert!(json.contains("\"mediaType\":\"image/png\""));
        assert!(json.contains("\"width\":800"));
        assert!(json.contains("\"blurhash\""));
    }

    #[test]
    fn test_note_with_reply() {
        let mut note = ApNote::new(
            test_url("/notes/reply123"),
            test_url("/users/alice"),
            "This is a reply".to_string(),
            Utc::now(),
        );
        note.in_reply_to = Some(test_url("/notes/original"));

        let json = serde_json::to_string(&note).unwrap();
        assert!(json.contains("\"inReplyTo\":\"https://example.com/notes/original\""));
    }

    #[test]
    fn test_note_with_cw() {
        let mut note = ApNote::new(
            test_url("/notes/cw123"),
            test_url("/users/alice"),
            "Spoiler content here".to_string(),
            Utc::now(),
        );
        note.summary = Some("Spoiler warning!".to_string());
        note.sensitive = Some(true);

        let json = serde_json::to_string(&note).unwrap();
        assert!(json.contains("\"summary\":\"Spoiler warning!\""));
        assert!(json.contains("\"sensitive\":true"));
    }

    #[test]
    fn test_public_addressing() {
        let note = ApNote::new(
            test_url("/notes/public123"),
            test_url("/users/alice"),
            "Public post".to_string(),
            Utc::now(),
        )
        .public();

        let json = serde_json::to_string(&note).unwrap();
        assert!(json.contains("\"to\":[\"https://www.w3.org/ns/activitystreams#Public\"]"));
    }
}
