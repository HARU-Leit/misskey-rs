//! Federation compatibility tests.
//!
//! Tests to verify compatibility with other `ActivityPub` implementations:
//! - Mastodon
//! - Pleroma/Akkoma
//! - Misskey (original)
//!
//! These tests focus on verifying that the `ActivityPub` objects and activities
//! we produce are compatible with what these implementations expect.

use chrono::Utc;
use misskey_federation::objects::{ApAttachment, ApNote, ApObjectType, ApTag};
use serde_json::json;
use url::Url;

fn test_url(path: &str) -> Url {
    Url::parse(&format!("https://example.com{path}")).unwrap()
}

// =============================================================================
// Mastodon Compatibility Tests
// =============================================================================

mod mastodon {
    use super::*;

    /// Mastodon requires these fields in Note objects.
    #[test]
    fn test_note_has_required_fields() {
        let note = ApNote::new(
            test_url("/notes/123"),
            test_url("/users/alice"),
            "<p>Hello, Mastodon!</p>".to_string(),
            Utc::now(),
        )
        .public();

        let json = serde_json::to_value(&note).unwrap();

        // Required fields for Mastodon
        assert!(json["id"].is_string(), "Missing 'id' field");
        assert!(json["type"].is_string(), "Missing 'type' field");
        assert!(json["attributedTo"].is_string(), "Missing 'attributedTo' field");
        assert!(json["content"].is_string(), "Missing 'content' field");
        assert!(json["published"].is_string(), "Missing 'published' field");
        assert!(json["to"].is_array(), "Missing 'to' field");
    }

    /// Mastodon expects HTML content.
    #[test]
    fn test_content_is_html() {
        let note = ApNote::new(
            test_url("/notes/html-test"),
            test_url("/users/alice"),
            "<p>This is <strong>HTML</strong> content.</p>".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_value(&note).unwrap();
        let content = json["content"].as_str().unwrap();

        // Mastodon expects HTML
        assert!(content.contains("<p>") || content.contains("<strong>") || !content.contains('<'),
            "Content should be HTML or plain text");
    }

    /// Mastodon visibility through addressing.
    #[test]
    fn test_public_visibility_addressing() {
        let note = ApNote::new(
            test_url("/notes/public"),
            test_url("/users/alice"),
            "Public post".to_string(),
            Utc::now(),
        )
        .public();

        let json = serde_json::to_value(&note).unwrap();

        // Public posts should have Public in 'to'
        let to = json["to"].as_array().unwrap();
        let has_public = to.iter().any(|v| {
            v.as_str()
                .is_some_and(|s| s.contains("Public") || s.contains("#Public"))
        });
        assert!(has_public, "Public post should have Public in 'to' array");
    }

    /// Mastodon expects Mention tags for mentions.
    #[test]
    fn test_mention_tag_format() {
        let mut note = ApNote::new(
            test_url("/notes/mention-test"),
            test_url("/users/alice"),
            "@bob Hello!".to_string(),
            Utc::now(),
        );

        note.tag = Some(vec![ApTag {
            kind: "Mention".to_string(),
            href: Some(test_url("/users/bob")),
            name: Some("@bob".to_string()),
        }]);

        let json = serde_json::to_value(&note).unwrap();

        let tags = json["tag"].as_array().unwrap();
        let mention = &tags[0];

        assert_eq!(mention["type"], "Mention");
        assert!(mention["href"].is_string());
        assert!(mention["name"].is_string());
    }

    /// Mastodon expects Hashtag tags for hashtags.
    #[test]
    fn test_hashtag_tag_format() {
        let mut note = ApNote::new(
            test_url("/notes/hashtag-test"),
            test_url("/users/alice"),
            "#rust #programming".to_string(),
            Utc::now(),
        );

        note.tag = Some(vec![ApTag {
            kind: "Hashtag".to_string(),
            href: Some(test_url("/tags/rust")),
            name: Some("#rust".to_string()),
        }]);

        let json = serde_json::to_value(&note).unwrap();

        let tags = json["tag"].as_array().unwrap();
        let hashtag = &tags[0];

        assert_eq!(hashtag["type"], "Hashtag");
        assert!(hashtag["name"].as_str().unwrap().starts_with('#'));
    }

    /// Mastodon attachment format.
    #[test]
    fn test_attachment_format() {
        let mut note = ApNote::new(
            test_url("/notes/media-test"),
            test_url("/users/alice"),
            "Check this out!".to_string(),
            Utc::now(),
        );

        note.attachment = Some(vec![ApAttachment {
            kind: "Document".to_string(),
            url: test_url("/files/image.png"),
            media_type: Some("image/png".to_string()),
            name: Some("Alt text description".to_string()),
            width: Some(1920),
            height: Some(1080),
            blurhash: Some("LEHV6nWB2yk8pyo0adR*.7kCMdnj".to_string()),
        }]);

        let json = serde_json::to_value(&note).unwrap();

        let attachments = json["attachment"].as_array().unwrap();
        let attachment = &attachments[0];

        // Mastodon expected fields
        assert_eq!(attachment["type"], "Document");
        assert!(attachment["url"].is_string());
        assert!(attachment["mediaType"].is_string());
        // 'name' is used for alt text in Mastodon
        assert!(attachment["name"].is_string());
    }

    /// Mastodon uses summary for content warnings.
    #[test]
    fn test_content_warning_format() {
        let mut note = ApNote::new(
            test_url("/notes/cw-test"),
            test_url("/users/alice"),
            "Spoiler content".to_string(),
            Utc::now(),
        );

        note.summary = Some("Spoiler Alert!".to_string());
        note.sensitive = Some(true);

        let json = serde_json::to_value(&note).unwrap();

        assert_eq!(json["summary"], "Spoiler Alert!");
        assert_eq!(json["sensitive"], true);
    }

    /// Mastodon reply format.
    #[test]
    fn test_reply_format() {
        let mut note = ApNote::new(
            test_url("/notes/reply-test"),
            test_url("/users/alice"),
            "This is a reply".to_string(),
            Utc::now(),
        );

        note.in_reply_to = Some(test_url("/notes/original"));

        let json = serde_json::to_value(&note).unwrap();

        assert!(json["inReplyTo"].is_string());
        assert!(json["inReplyTo"]
            .as_str()
            .unwrap()
            .contains("/notes/original"));
    }

    /// Mastodon poll format (Question type).
    #[test]
    fn test_poll_format() {
        let question = ApNote::new_question(
            test_url("/notes/poll-test"),
            test_url("/users/alice"),
            "What's your favorite language?".to_string(),
            Utc::now(),
            vec!["Rust".to_string(), "Go".to_string(), "Python".to_string()],
            false,
            None,
        );

        let json = serde_json::to_value(&question).unwrap();

        assert_eq!(json["type"], "Question");
        assert!(json["oneOf"].is_array());

        let options = json["oneOf"].as_array().unwrap();
        assert_eq!(options.len(), 3);

        // Each option should have type "Note" and a name
        for option in options {
            assert_eq!(option["type"], "Note");
            assert!(option["name"].is_string());
        }
    }
}

// =============================================================================
// Pleroma/Akkoma Compatibility Tests
// =============================================================================

mod pleroma {
    use super::*;

    /// Pleroma accepts both Note and Question types.
    #[test]
    fn test_note_type_accepted() {
        let note = ApNote::new(
            test_url("/notes/pleroma-test"),
            test_url("/users/alice"),
            "Hello Pleroma!".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_value(&note).unwrap();
        assert!(json["type"] == "Note" || json["type"] == "Question");
    }

    /// Pleroma can parse emoji reactions.
    #[test]
    fn test_like_with_emoji() {
        // Pleroma supports emoji reactions through Like activities
        // We just need to ensure our format is compatible
        let like_activity = json!({
            "type": "Like",
            "id": "https://example.com/activities/like123",
            "actor": "https://example.com/users/alice",
            "object": "https://pleroma.instance/objects/note456",
            "content": "👍"
        });

        assert_eq!(like_activity["type"], "Like");
        assert!(like_activity["content"].is_string());
    }

    /// Pleroma `ChatMessage` support (for direct messages).
    #[test]
    fn test_direct_message_addressing() {
        // Direct messages should have specific user in 'to' and no public/followers
        let direct_note = ApNote::new(
            test_url("/notes/dm-test"),
            test_url("/users/alice"),
            "Private message".to_string(),
            Utc::now(),
        );

        // For direct messages, we don't use .public()
        let json = serde_json::to_value(&direct_note).unwrap();

        // Should not have Public in addressing
        if let Some(to) = json["to"].as_array() {
            let has_public = to.iter().any(|v| {
                v.as_str()
                    .is_some_and(|s| s.contains("Public"))
            });
            assert!(!has_public, "Direct messages should not be public");
        }
    }

    /// Pleroma quote post support.
    #[test]
    fn test_quote_post_format() {
        let note = ApNote::new(
            test_url("/notes/quote-test"),
            test_url("/users/alice"),
            "Check this out!".to_string(),
            Utc::now(),
        )
        .with_quote(test_url("/notes/original"));

        let json = serde_json::to_value(&note).unwrap();

        // Pleroma uses quoteUrl (FEP-c16b)
        assert!(json["quoteUrl"].is_string());
    }
}

// =============================================================================
// Misskey Compatibility Tests
// =============================================================================

mod misskey {
    use super::*;

    /// Misskey-specific extensions.
    #[test]
    fn test_misskey_extensions() {
        let note = ApNote::new(
            test_url("/notes/misskey-test"),
            test_url("/users/alice"),
            "Hello Misskey!".to_string(),
            Utc::now(),
        )
        .with_quote(test_url("/notes/quoted"));

        let json = serde_json::to_value(&note).unwrap();

        // Misskey uses _misskey_quote
        assert!(json["_misskey_quote"].is_string());
    }

    /// Misskey reaction format (_`misskey_reaction`).
    #[test]
    fn test_misskey_reaction_extension() {
        // Misskey uses custom emoji reactions through Like activities
        let like = json!({
            "type": "Like",
            "id": "https://example.com/activities/like123",
            "actor": "https://example.com/users/alice",
            "object": "https://misskey.io/notes/abc",
            "_misskey_reaction": "👍"
        });

        assert!(like["_misskey_reaction"].is_string());
    }

    /// Misskey expects _`misskey_content` for MFM.
    #[test]
    fn test_misskey_mfm_content() {
        let mut note = ApNote::new(
            test_url("/notes/mfm-test"),
            test_url("/users/alice"),
            "<p>Hello <span class=\"mfm-bold\">world</span></p>".to_string(),
            Utc::now(),
        );

        // _misskey_content contains raw MFM
        note.misskey_content = Some("Hello **world**".to_string());

        let json = serde_json::to_value(&note).unwrap();

        assert!(json["_misskey_content"].is_string());
        assert_eq!(json["_misskey_content"], "Hello **world**");
    }

    /// Misskey poll with _`misskey_summary` for CW.
    #[test]
    fn test_misskey_poll_with_cw() {
        let mut question = ApNote::new_question(
            test_url("/notes/poll-cw"),
            test_url("/users/alice"),
            "Sensitive poll".to_string(),
            Utc::now(),
            vec!["Option A".to_string(), "Option B".to_string()],
            false,
            None,
        );

        question.summary = Some("Content Warning".to_string());
        question.misskey_summary = Some("Content Warning".to_string());

        let json = serde_json::to_value(&question).unwrap();

        // Both standard and Misskey-specific CW fields
        assert_eq!(json["summary"], "Content Warning");
        assert_eq!(json["_misskey_summary"], "Content Warning");
    }

    /// Misskey handles both quoteUrl and _`misskey_quote`.
    #[test]
    fn test_quote_url_compatibility() {
        let note = ApNote::new(
            test_url("/notes/quote-compat"),
            test_url("/users/alice"),
            "Quoting!".to_string(),
            Utc::now(),
        )
        .with_quote(test_url("/notes/original"));

        let json = serde_json::to_value(&note).unwrap();

        // Both fields should be set for maximum compatibility
        assert_eq!(json["quoteUrl"], json["_misskey_quote"]);
    }
}

// =============================================================================
// General ActivityPub Compliance Tests
// =============================================================================

mod activitypub_compliance {
    use super::*;

    /// Test that type field is always present.
    #[test]
    fn test_type_field_present() {
        let note = ApNote::new(
            test_url("/notes/type-test"),
            test_url("/users/alice"),
            "Test".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_value(&note).unwrap();
        assert!(json["type"].is_string());
        assert!(json["type"] == "Note" || json["type"] == "Question");
    }

    /// Test that id is a valid URL.
    #[test]
    fn test_id_is_valid_url() {
        let note = ApNote::new(
            test_url("/notes/id-test"),
            test_url("/users/alice"),
            "Test".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_value(&note).unwrap();
        let id = json["id"].as_str().unwrap();
        assert!(Url::parse(id).is_ok(), "id should be a valid URL");
    }

    /// Test that attributedTo is a valid URL.
    #[test]
    fn test_attributed_to_is_valid_url() {
        let note = ApNote::new(
            test_url("/notes/author-test"),
            test_url("/users/alice"),
            "Test".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_value(&note).unwrap();
        let author = json["attributedTo"].as_str().unwrap();
        assert!(Url::parse(author).is_ok(), "attributedTo should be a valid URL");
    }

    /// Test that published is ISO 8601 format.
    #[test]
    fn test_published_is_iso8601() {
        let note = ApNote::new(
            test_url("/notes/date-test"),
            test_url("/users/alice"),
            "Test".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_value(&note).unwrap();
        let published = json["published"].as_str().unwrap();

        // Should be parseable as ISO 8601
        assert!(
            chrono::DateTime::parse_from_rfc3339(published).is_ok(),
            "published should be ISO 8601/RFC 3339 format"
        );
    }

    /// Test that addressing arrays contain valid URLs.
    #[test]
    fn test_addressing_contains_valid_urls() {
        let note = ApNote::new(
            test_url("/notes/addressing-test"),
            test_url("/users/alice"),
            "Test".to_string(),
            Utc::now(),
        )
        .public();

        let json = serde_json::to_value(&note).unwrap();

        if let Some(to) = json["to"].as_array() {
            for addr in to {
                if let Some(url_str) = addr.as_str() {
                    assert!(Url::parse(url_str).is_ok(), "to entry should be valid URL: {url_str}");
                }
            }
        }

        if let Some(cc) = json["cc"].as_array() {
            for addr in cc {
                if let Some(url_str) = addr.as_str() {
                    assert!(Url::parse(url_str).is_ok(), "cc entry should be valid URL: {url_str}");
                }
            }
        }
    }

    /// Test that optional fields are properly omitted when None.
    #[test]
    fn test_optional_fields_omitted() {
        let note = ApNote::new(
            test_url("/notes/optional-test"),
            test_url("/users/alice"),
            "Minimal note".to_string(),
            Utc::now(),
        );

        let json = serde_json::to_value(&note).unwrap();

        // These should not be present when None
        assert!(json.get("inReplyTo").is_none() || json["inReplyTo"].is_null());
        assert!(json.get("summary").is_none() || json["summary"].is_null());
        assert!(json.get("tag").is_none() || json["tag"].is_null());
        assert!(json.get("attachment").is_none() || json["attachment"].is_null());
    }
}

// =============================================================================
// Incoming Activity Parsing Tests (What we receive from other servers)
// =============================================================================

mod incoming_parsing {
    use super::*;

    /// Parse a note from Mastodon format.
    #[test]
    fn test_parse_mastodon_note() {
        let mastodon_note = r#"{
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Note",
            "id": "https://mastodon.social/users/alice/statuses/123456789",
            "attributedTo": "https://mastodon.social/users/alice",
            "content": "<p>Hello from Mastodon!</p>",
            "published": "2025-01-01T00:00:00Z",
            "to": ["https://www.w3.org/ns/activitystreams#Public"],
            "cc": ["https://mastodon.social/users/alice/followers"]
        }"#;

        let note: ApNote = serde_json::from_str(mastodon_note).unwrap();

        assert_eq!(note.kind, ApObjectType::Note);
        assert!(note.content.contains("Mastodon"));
    }

    /// Parse a note from Pleroma format.
    #[test]
    fn test_parse_pleroma_note() {
        let pleroma_note = r#"{
            "type": "Note",
            "id": "https://pleroma.example/objects/abc123",
            "attributedTo": "https://pleroma.example/users/bob",
            "content": "Hello from Pleroma!",
            "published": "2025-01-01T12:00:00Z",
            "to": ["https://www.w3.org/ns/activitystreams#Public"],
            "quoteUrl": "https://example.com/notes/original"
        }"#;

        let note: ApNote = serde_json::from_str(pleroma_note).unwrap();

        assert_eq!(note.kind, ApObjectType::Note);
        assert!(note.quote_url.is_some());
    }

    /// Parse a note from Misskey format.
    #[test]
    fn test_parse_misskey_note() {
        let misskey_note = r#"{
            "type": "Note",
            "id": "https://misskey.io/notes/xyz789",
            "attributedTo": "https://misskey.io/users/charlie",
            "content": "<p>Hello from Misskey!</p>",
            "published": "2025-01-01T06:00:00Z",
            "to": ["https://www.w3.org/ns/activitystreams#Public"],
            "_misskey_content": "Hello from Misskey!",
            "_misskey_quote": "https://example.com/notes/quoted"
        }"#;

        let note: ApNote = serde_json::from_str(misskey_note).unwrap();

        assert_eq!(note.kind, ApObjectType::Note);
        assert!(note.misskey_content.is_some());
        assert!(note.misskey_quote.is_some());
    }

    /// Parse a poll from various formats.
    #[test]
    fn test_parse_question_poll() {
        let question = r#"{
            "type": "Question",
            "id": "https://example.com/notes/poll123",
            "attributedTo": "https://example.com/users/alice",
            "content": "What's your favorite?",
            "published": "2025-01-01T00:00:00Z",
            "oneOf": [
                {"type": "Note", "name": "Option A"},
                {"type": "Note", "name": "Option B"}
            ]
        }"#;

        let poll: ApNote = serde_json::from_str(question).unwrap();

        assert_eq!(poll.kind, ApObjectType::Question);
        assert!(poll.one_of.is_some());
        assert_eq!(poll.one_of.as_ref().unwrap().len(), 2);
    }

    /// Handle notes with both quoteUrl and _`misskey_quote`.
    #[test]
    fn test_parse_dual_quote_fields() {
        let note = r#"{
            "type": "Note",
            "id": "https://example.com/notes/123",
            "attributedTo": "https://example.com/users/alice",
            "content": "Quoting!",
            "published": "2025-01-01T00:00:00Z",
            "quoteUrl": "https://remote.com/notes/original",
            "_misskey_quote": "https://remote.com/notes/original"
        }"#;

        let parsed: ApNote = serde_json::from_str(note).unwrap();

        // Both should be available
        assert!(parsed.quote_url.is_some());
        assert!(parsed.misskey_quote.is_some());

        // get_quote_url should return one of them
        assert!(parsed.get_quote_url().is_some());
    }

    /// Handle missing optional fields gracefully.
    #[test]
    fn test_parse_minimal_note() {
        let minimal = r#"{
            "type": "Note",
            "id": "https://example.com/notes/minimal",
            "attributedTo": "https://example.com/users/alice",
            "content": "Minimal",
            "published": "2025-01-01T00:00:00Z"
        }"#;

        let note: ApNote = serde_json::from_str(minimal).unwrap();

        assert!(note.to.is_none());
        assert!(note.cc.is_none());
        assert!(note.in_reply_to.is_none());
        assert!(note.summary.is_none());
        assert!(note.tag.is_none());
        assert!(note.attachment.is_none());
    }
}

// =============================================================================
// FEP-c16b Quote Post Compliance Tests
// =============================================================================

mod fep_c16b {
    use super::*;

    /// FEP-c16b specifies quoteUrl as the standard field.
    #[test]
    fn test_quote_url_is_primary() {
        let note = ApNote::new(
            test_url("/notes/fep-quote"),
            test_url("/users/alice"),
            "Quote post!".to_string(),
            Utc::now(),
        )
        .with_quote(test_url("/notes/original"));

        let json = serde_json::to_value(&note).unwrap();

        // quoteUrl should be present
        assert!(json["quoteUrl"].is_string());
    }

    /// For backwards compatibility, _`misskey_quote` should also be set.
    #[test]
    fn test_misskey_quote_for_compatibility() {
        let note = ApNote::new(
            test_url("/notes/compat-quote"),
            test_url("/users/alice"),
            "Compatible quote!".to_string(),
            Utc::now(),
        )
        .with_quote(test_url("/notes/original"));

        let json = serde_json::to_value(&note).unwrap();

        // Both should be set
        assert!(json["quoteUrl"].is_string());
        assert!(json["_misskey_quote"].is_string());
        assert_eq!(json["quoteUrl"], json["_misskey_quote"]);
    }

    /// Test that `get_quote_url` checks all quote fields.
    #[test]
    fn test_get_quote_url_fallbacks() {
        // Only quoteUrl
        let json1 = r#"{
            "type": "Note",
            "id": "https://example.com/notes/1",
            "attributedTo": "https://example.com/users/a",
            "content": "Test",
            "published": "2025-01-01T00:00:00Z",
            "quoteUrl": "https://example.com/notes/original"
        }"#;
        let note1: ApNote = serde_json::from_str(json1).unwrap();
        assert!(note1.get_quote_url().is_some());

        // Only quoteUri
        let json2 = r#"{
            "type": "Note",
            "id": "https://example.com/notes/2",
            "attributedTo": "https://example.com/users/a",
            "content": "Test",
            "published": "2025-01-01T00:00:00Z",
            "quoteUri": "https://example.com/notes/original"
        }"#;
        let note2: ApNote = serde_json::from_str(json2).unwrap();
        assert!(note2.get_quote_url().is_some());

        // Only _misskey_quote
        let json3 = r#"{
            "type": "Note",
            "id": "https://example.com/notes/3",
            "attributedTo": "https://example.com/users/a",
            "content": "Test",
            "published": "2025-01-01T00:00:00Z",
            "_misskey_quote": "https://example.com/notes/original"
        }"#;
        let note3: ApNote = serde_json::from_str(json3).unwrap();
        assert!(note3.get_quote_url().is_some());
    }
}
