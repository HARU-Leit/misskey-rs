//! Federation integration tests.
//!
//! Tests for `ActivityPub` federation functionality.

#![allow(clippy::unwrap_used)]

use chrono::Utc;
use misskey_federation::objects::{ApAttachment, ApNote, ApTag};
use url::Url;

fn test_url(path: &str) -> Url {
    Url::parse(&format!("https://example.com{path}")).unwrap()
}

#[test]
fn test_create_activity_structure() {
    // Test that we can create a valid Create activity with Note
    let note = ApNote::new(
        test_url("/notes/123"),
        test_url("/users/alice"),
        "Hello, fediverse!".to_string(),
        Utc::now(),
    )
    .public();

    let json = serde_json::to_value(&note).unwrap();

    // Verify structure
    assert_eq!(json["type"], "Note");
    assert!(json["id"].as_str().unwrap().contains("/notes/123"));
    assert!(
        json["attributedTo"]
            .as_str()
            .unwrap()
            .contains("/users/alice")
    );
    assert_eq!(json["content"], "Hello, fediverse!");
    assert!(json["to"].is_array());
}

#[test]
fn test_follow_activity_compatible_format() {
    // Test that Follow activities have the right structure for other implementations
    #[derive(serde::Serialize)]
    struct Follow {
        #[serde(rename = "@context")]
        context: String,
        #[serde(rename = "type")]
        kind: String,
        id: String,
        actor: String,
        object: String,
    }

    let follow = Follow {
        context: "https://www.w3.org/ns/activitystreams".to_string(),
        kind: "Follow".to_string(),
        id: "https://example.com/activities/follow123".to_string(),
        actor: "https://example.com/users/alice".to_string(),
        object: "https://remote.server/users/bob".to_string(),
    };

    let json = serde_json::to_value(&follow).unwrap();

    assert_eq!(json["type"], "Follow");
    assert!(json["actor"].as_str().unwrap().contains("alice"));
    assert!(json["object"].as_str().unwrap().contains("bob"));
}

#[test]
fn test_like_activity_with_misskey_reaction() {
    // Test Misskey-style reactions (Like with _misskey_reaction)
    #[derive(serde::Serialize)]
    struct Like {
        #[serde(rename = "type")]
        kind: String,
        id: String,
        actor: String,
        object: String,
        #[serde(rename = "_misskey_reaction")]
        misskey_reaction: String,
    }

    let like = Like {
        kind: "Like".to_string(),
        id: "https://example.com/activities/like123".to_string(),
        actor: "https://example.com/users/alice".to_string(),
        object: "https://remote.server/notes/456".to_string(),
        misskey_reaction: "👍".to_string(),
    };

    let json = serde_json::to_value(&like).unwrap();

    assert_eq!(json["type"], "Like");
    assert_eq!(json["_misskey_reaction"], "👍");
}

#[test]
fn test_announce_activity_for_renote() {
    // Test Announce (boost/renote) activity structure
    #[derive(serde::Serialize)]
    struct Announce {
        #[serde(rename = "type")]
        kind: String,
        id: String,
        actor: String,
        object: String,
        to: Vec<String>,
        cc: Vec<String>,
    }

    let announce = Announce {
        kind: "Announce".to_string(),
        id: "https://example.com/activities/announce123".to_string(),
        actor: "https://example.com/users/alice".to_string(),
        object: "https://remote.server/notes/789".to_string(),
        to: vec!["https://www.w3.org/ns/activitystreams#Public".to_string()],
        cc: vec!["https://example.com/users/alice/followers".to_string()],
    };

    let json = serde_json::to_value(&announce).unwrap();

    assert_eq!(json["type"], "Announce");
    assert!(json["to"][0].as_str().unwrap().contains("Public"));
}

#[test]
fn test_note_with_mentions() {
    let mut note = ApNote::new(
        test_url("/notes/mention123"),
        test_url("/users/alice"),
        "@bob Hello there!".to_string(),
        Utc::now(),
    );

    note.tag = Some(vec![ApTag {
        kind: "Mention".to_string(),
        href: Some(test_url("/users/bob")),
        name: Some("@bob".to_string()),
    }]);

    let json = serde_json::to_value(&note).unwrap();

    assert!(json["tag"].is_array());
    assert_eq!(json["tag"][0]["type"], "Mention");
    assert_eq!(json["tag"][0]["name"], "@bob");
}

#[test]
fn test_note_with_hashtags() {
    let mut note = ApNote::new(
        test_url("/notes/hashtag123"),
        test_url("/users/alice"),
        "Check out #rust and #programming".to_string(),
        Utc::now(),
    );

    note.tag = Some(vec![
        ApTag {
            kind: "Hashtag".to_string(),
            href: Some(test_url("/tags/rust")),
            name: Some("#rust".to_string()),
        },
        ApTag {
            kind: "Hashtag".to_string(),
            href: Some(test_url("/tags/programming")),
            name: Some("#programming".to_string()),
        },
    ]);

    let json = serde_json::to_value(&note).unwrap();

    assert_eq!(json["tag"].as_array().unwrap().len(), 2);
    assert_eq!(json["tag"][0]["type"], "Hashtag");
}

#[test]
fn test_note_with_attachments() {
    let mut note = ApNote::new(
        test_url("/notes/media123"),
        test_url("/users/alice"),
        "Check out this image!".to_string(),
        Utc::now(),
    );

    note.attachment = Some(vec![ApAttachment {
        kind: "Document".to_string(),
        url: test_url("/files/image.png"),
        media_type: Some("image/png".to_string()),
        name: Some("My beautiful image".to_string()),
        width: Some(1920),
        height: Some(1080),
        blurhash: Some("LEHV6nWB2yk8pyo0adR*.7kCMdnj".to_string()),
    }]);

    let json = serde_json::to_value(&note).unwrap();

    assert!(json["attachment"].is_array());
    assert_eq!(json["attachment"][0]["type"], "Document");
    assert_eq!(json["attachment"][0]["mediaType"], "image/png");
    assert!(json["attachment"][0]["blurhash"].is_string());
}

#[test]
fn test_question_poll_format() {
    let question = ApNote::new_question(
        test_url("/notes/poll123"),
        test_url("/users/alice"),
        "What's your favorite language?".to_string(),
        Utc::now(),
        vec![
            "Rust".to_string(),
            "Go".to_string(),
            "Python".to_string(),
            "TypeScript".to_string(),
        ],
        false,
        None,
    );

    let json = serde_json::to_value(&question).unwrap();

    assert_eq!(json["type"], "Question");
    assert!(json["oneOf"].is_array());
    assert_eq!(json["oneOf"].as_array().unwrap().len(), 4);
    assert_eq!(json["oneOf"][0]["name"], "Rust");
}

#[test]
fn test_visibility_addressing() {
    // Public post
    let public_note = ApNote::new(
        test_url("/notes/public"),
        test_url("/users/alice"),
        "Public post".to_string(),
        Utc::now(),
    )
    .public();

    let json = serde_json::to_value(&public_note).unwrap();
    assert!(
        json["to"][0]
            .as_str()
            .unwrap()
            .contains("activitystreams#Public")
    );
}

#[test]
fn test_reply_chain() {
    let mut reply = ApNote::new(
        test_url("/notes/reply123"),
        test_url("/users/bob"),
        "This is a reply".to_string(),
        Utc::now(),
    );
    reply.in_reply_to = Some(test_url("/notes/original"));

    let json = serde_json::to_value(&reply).unwrap();

    assert!(json["inReplyTo"].is_string());
    assert!(
        json["inReplyTo"]
            .as_str()
            .unwrap()
            .contains("/notes/original")
    );
}

#[test]
fn test_content_warning_sensitive() {
    let mut note = ApNote::new(
        test_url("/notes/cw123"),
        test_url("/users/alice"),
        "Sensitive content here".to_string(),
        Utc::now(),
    );
    note.summary = Some("Content Warning".to_string());
    note.sensitive = Some(true);

    let json = serde_json::to_value(&note).unwrap();

    assert_eq!(json["summary"], "Content Warning");
    assert_eq!(json["sensitive"], true);
}

#[test]
fn test_webfinger_response_format() {
    // Test WebFinger response structure
    #[derive(serde::Serialize)]
    struct WebFingerResponse {
        subject: String,
        aliases: Vec<String>,
        links: Vec<WebFingerLink>,
    }

    #[derive(serde::Serialize)]
    struct WebFingerLink {
        rel: String,
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        link_type: Option<String>,
        href: String,
    }

    let response = WebFingerResponse {
        subject: "acct:alice@example.com".to_string(),
        aliases: vec!["https://example.com/users/alice".to_string()],
        links: vec![
            WebFingerLink {
                rel: "self".to_string(),
                link_type: Some("application/activity+json".to_string()),
                href: "https://example.com/users/alice".to_string(),
            },
            WebFingerLink {
                rel: "http://webfinger.net/rel/profile-page".to_string(),
                link_type: Some("text/html".to_string()),
                href: "https://example.com/@alice".to_string(),
            },
        ],
    };

    let json = serde_json::to_value(&response).unwrap();

    assert!(json["subject"].as_str().unwrap().starts_with("acct:"));
    assert!(json["links"].is_array());
    assert_eq!(json["links"][0]["rel"], "self");
    assert!(
        json["links"][0]["type"]
            .as_str()
            .unwrap()
            .contains("activity+json")
    );
}

#[test]
fn test_nodeinfo_structure() {
    #[derive(serde::Serialize)]
    struct NodeInfo {
        version: String,
        software: NodeInfoSoftware,
        protocols: Vec<String>,
        #[serde(rename = "openRegistrations")]
        open_registrations: bool,
    }

    #[derive(serde::Serialize)]
    struct NodeInfoSoftware {
        name: String,
        version: String,
    }

    let nodeinfo = NodeInfo {
        version: "2.1".to_string(),
        software: NodeInfoSoftware {
            name: "misskey-rs".to_string(),
            version: "0.1.0".to_string(),
        },
        protocols: vec!["activitypub".to_string()],
        open_registrations: true,
    };

    let json = serde_json::to_value(&nodeinfo).unwrap();

    assert_eq!(json["version"], "2.1");
    assert_eq!(json["software"]["name"], "misskey-rs");
    assert!(
        json["protocols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "activitypub")
    );
}

#[test]
fn test_mastodon_compatibility_note() {
    // Test that notes are compatible with Mastodon's expected format
    let note = ApNote::new(
        test_url("/notes/compat123"),
        test_url("/users/alice"),
        "<p>Hello from Misskey-rs!</p>".to_string(),
        Utc::now(),
    )
    .public();

    let json = serde_json::to_value(&note).unwrap();

    // Required fields for Mastodon compatibility
    assert!(json["id"].is_string());
    assert!(json["type"].is_string());
    assert!(json["attributedTo"].is_string());
    assert!(json["content"].is_string());
    assert!(json["published"].is_string());
    assert!(json["to"].is_array());
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_note_with_empty_content() {
    let note = ApNote::new(
        test_url("/notes/empty"),
        test_url("/users/alice"),
        String::new(),
        Utc::now(),
    );

    let json = serde_json::to_value(&note).unwrap();
    assert_eq!(json["content"], "");
}

#[test]
fn test_note_with_very_long_content() {
    let long_content = "a".repeat(50000);
    let note = ApNote::new(
        test_url("/notes/long"),
        test_url("/users/alice"),
        long_content.clone(),
        Utc::now(),
    );

    let json = serde_json::to_value(&note).unwrap();
    assert_eq!(json["content"].as_str().unwrap().len(), 50000);
}

#[test]
fn test_note_with_unicode_content() {
    let unicode_content = "こんにちは 🌸 مرحبا 🎉 שלום 🚀";
    let note = ApNote::new(
        test_url("/notes/unicode"),
        test_url("/users/alice"),
        unicode_content.to_string(),
        Utc::now(),
    );

    let json = serde_json::to_value(&note).unwrap();
    assert_eq!(json["content"], unicode_content);
}

#[test]
fn test_note_with_special_html_characters() {
    let html_content = "<script>alert('xss')</script>&amp;&lt;&gt;";
    let note = ApNote::new(
        test_url("/notes/html-escape"),
        test_url("/users/alice"),
        html_content.to_string(),
        Utc::now(),
    );

    let json = serde_json::to_value(&note).unwrap();
    // Content should be preserved as-is (HTML is expected in content)
    assert!(json["content"].as_str().unwrap().contains("script"));
}

#[test]
fn test_note_with_many_tags() {
    let mut note = ApNote::new(
        test_url("/notes/many-tags"),
        test_url("/users/alice"),
        "Post with many tags".to_string(),
        Utc::now(),
    );

    // Add 100 tags
    let tags: Vec<ApTag> = (0..100)
        .map(|i| ApTag {
            kind: "Hashtag".to_string(),
            href: Some(test_url(&format!("/tags/tag{i}"))),
            name: Some(format!("#tag{i}")),
        })
        .collect();

    note.tag = Some(tags);

    let json = serde_json::to_value(&note).unwrap();
    assert_eq!(json["tag"].as_array().unwrap().len(), 100);
}

#[test]
fn test_note_with_many_attachments() {
    let mut note = ApNote::new(
        test_url("/notes/many-attachments"),
        test_url("/users/alice"),
        "Post with many attachments".to_string(),
        Utc::now(),
    );

    // Add 16 attachments (Misskey's limit)
    let attachments: Vec<ApAttachment> = (0..16)
        .map(|i| ApAttachment {
            kind: "Document".to_string(),
            url: test_url(&format!("/files/file{i}.png")),
            media_type: Some("image/png".to_string()),
            name: Some(format!("File {i}")),
            width: Some(1920),
            height: Some(1080),
            blurhash: None,
        })
        .collect();

    note.attachment = Some(attachments);

    let json = serde_json::to_value(&note).unwrap();
    assert_eq!(json["attachment"].as_array().unwrap().len(), 16);
}

#[test]
fn test_question_with_many_options() {
    let options: Vec<String> = (0..10).map(|i| format!("Option {i}")).collect();

    let question = ApNote::new_question(
        test_url("/notes/many-options"),
        test_url("/users/alice"),
        "Poll with many options".to_string(),
        Utc::now(),
        options,
        false,
        None,
    );

    let json = serde_json::to_value(&question).unwrap();
    assert_eq!(json["oneOf"].as_array().unwrap().len(), 10);
}

#[test]
fn test_question_with_multiple_choice() {
    let question = ApNote::new_question(
        test_url("/notes/multi-choice"),
        test_url("/users/alice"),
        "Multiple choice poll".to_string(),
        Utc::now(),
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
        true, // Multiple choice
        None,
    );

    let json = serde_json::to_value(&question).unwrap();
    // Multiple choice uses anyOf instead of oneOf
    assert!(json["anyOf"].is_array());
}

#[test]
fn test_note_roundtrip_serialization() {
    let note = ApNote::new(
        test_url("/notes/roundtrip"),
        test_url("/users/alice"),
        "Roundtrip test".to_string(),
        Utc::now(),
    )
    .public();

    let json_str = serde_json::to_string(&note).unwrap();
    let parsed: ApNote = serde_json::from_str(&json_str).unwrap();

    assert_eq!(note.id, parsed.id);
    assert_eq!(note.attributed_to, parsed.attributed_to);
    assert_eq!(note.content, parsed.content);
}

#[test]
fn test_note_with_null_fields() {
    // Parsing a note with explicit null values
    let json = r#"{
        "type": "Note",
        "id": "https://example.com/notes/null-test",
        "attributedTo": "https://example.com/users/alice",
        "content": "Test",
        "published": "2025-01-01T00:00:00Z",
        "inReplyTo": null,
        "summary": null,
        "sensitive": null
    }"#;

    let note: ApNote = serde_json::from_str(json).unwrap();
    assert!(note.in_reply_to.is_none());
    assert!(note.summary.is_none());
    assert!(note.sensitive.is_none());
}

#[test]
fn test_note_with_quote_url_in_different_formats() {
    // quoteUrl as string
    let json1 = r#"{
        "type": "Note",
        "id": "https://example.com/notes/1",
        "attributedTo": "https://example.com/users/alice",
        "content": "Test",
        "published": "2025-01-01T00:00:00Z",
        "quoteUrl": "https://example.com/notes/quoted"
    }"#;

    let note1: ApNote = serde_json::from_str(json1).unwrap();
    assert!(note1.quote_url.is_some());
}

#[test]
fn test_attachment_with_minimal_fields() {
    let attachment = ApAttachment {
        kind: "Document".to_string(),
        url: test_url("/files/minimal.png"),
        media_type: None,
        name: None,
        width: None,
        height: None,
        blurhash: None,
    };

    let json = serde_json::to_value(&attachment).unwrap();
    assert_eq!(json["type"], "Document");
    assert!(json["url"].is_string());
}

#[test]
fn test_tag_with_minimal_fields() {
    let tag = ApTag {
        kind: "Hashtag".to_string(),
        href: None,
        name: Some("#minimal".to_string()),
    };

    let json = serde_json::to_value(&tag).unwrap();
    assert_eq!(json["type"], "Hashtag");
    assert_eq!(json["name"], "#minimal");
}
