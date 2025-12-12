//! MFM (Misskey Flavored Markdown) parser.
//!
//! This crate provides parsing and rendering for MFM, the custom markdown-like
//! syntax used in Misskey.
//!
//! # Features
//!
//! - **Parsing**: Convert MFM text to an AST via [`parse`]
//! - **Rendering**: Convert AST to HTML via [`to_html`] or plain text via [`to_plain_text`]
//! - **Extraction**: Extract mentions via [`extract_mentions`] and hashtags via [`extract_hashtags`]
//! - **HTML conversion**: Convert HTML back to MFM via [`from_html`]
//!
//! # Example
//!
//! ```
//! use misskey_mfm::{parse, to_html, extract_mentions, extract_hashtags};
//!
//! let text = "Hello **world** @user #rust";
//! let nodes = parse(text);
//! let html = to_html(text);
//! let mentions = extract_mentions(text);
//! let hashtags = extract_hashtags(text);
//! ```

#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::too_many_lines)]

mod nodes;
mod parser;
mod render;

pub use nodes::{MfmNode, MfmNodeType};
pub use parser::{Mention, extract_hashtags, extract_mentions, parse};
pub use render::{from_html, to_html, to_plain_text};

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let nodes = parse("Hello, world!");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(nodes[0].node_type, MfmNodeType::Text { .. }));
    }

    #[test]
    fn test_parse_mention() {
        let nodes = parse("Hello @user");
        assert_eq!(nodes.len(), 2);
        assert!(matches!(nodes[1].node_type, MfmNodeType::Mention { .. }));
    }

    #[test]
    fn test_parse_mention_with_host() {
        let nodes = parse("Hello @user@example.com");
        assert_eq!(nodes.len(), 2);
        if let MfmNodeType::Mention { username, host, .. } = &nodes[1].node_type {
            assert_eq!(username, "user");
            assert_eq!(host.as_deref(), Some("example.com"));
        } else {
            panic!("Expected mention node");
        }
    }

    #[test]
    fn test_parse_hashtag() {
        let nodes = parse("Check out #rust");
        assert_eq!(nodes.len(), 2);
        if let MfmNodeType::Hashtag { tag } = &nodes[1].node_type {
            assert_eq!(tag, "rust");
        } else {
            panic!("Expected hashtag node");
        }
    }

    #[test]
    fn test_parse_url() {
        let nodes = parse("Visit https://example.com for more");
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[1].node_type, MfmNodeType::Url { .. }));
    }

    #[test]
    fn test_parse_emoji() {
        let nodes = parse("Hello :smile: world");
        assert_eq!(nodes.len(), 3);
        if let MfmNodeType::Emoji { name } = &nodes[1].node_type {
            assert_eq!(name, "smile");
        } else {
            panic!("Expected emoji node");
        }
    }

    #[test]
    fn test_parse_bold() {
        let nodes = parse("Hello **bold** world");
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[1].node_type, MfmNodeType::Bold { .. }));
    }

    #[test]
    fn test_parse_italic() {
        let nodes = parse("Hello *italic* world");
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[1].node_type, MfmNodeType::Italic { .. }));
    }

    #[test]
    fn test_extract_mentions() {
        let mentions = extract_mentions("Hello @user1 and @user2@remote.com");
        assert_eq!(mentions.len(), 2);
        assert_eq!(mentions[0].username, "user1");
        assert!(mentions[0].host.is_none());
        assert_eq!(mentions[1].username, "user2");
        assert_eq!(mentions[1].host.as_deref(), Some("remote.com"));
    }

    #[test]
    fn test_extract_hashtags() {
        let hashtags = extract_hashtags("Check out #rust and #programming");
        assert_eq!(hashtags.len(), 2);
        assert!(hashtags.contains(&"rust".to_string()));
        assert!(hashtags.contains(&"programming".to_string()));
    }

    #[test]
    fn test_to_plain_text() {
        let text = to_plain_text("Hello **bold** and @user");
        assert_eq!(text, "Hello bold and @user");
    }

    #[test]
    fn test_to_html() {
        let html = to_html("Hello **bold**");
        assert!(html.contains("<b>bold</b>"));
    }

    // Edge case tests
    #[test]
    fn test_parse_empty_string() {
        let nodes = parse("");
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_parse_only_whitespace() {
        let nodes = parse("   \n\t  ");
        assert_eq!(nodes.len(), 1);
        assert!(matches!(nodes[0].node_type, MfmNodeType::Text { .. }));
    }

    #[test]
    fn test_parse_consecutive_mentions() {
        let nodes = parse("@user1@user2@user3");
        // Should parse as a single mention with host containing at signs
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_mention_at_start() {
        let nodes = parse("@user hello");
        assert_eq!(nodes.len(), 2);
        assert!(matches!(nodes[0].node_type, MfmNodeType::Mention { .. }));
    }

    #[test]
    fn test_parse_hashtag_with_numbers() {
        let nodes = parse("#rust2025");
        assert_eq!(nodes.len(), 1);
        if let MfmNodeType::Hashtag { tag } = &nodes[0].node_type {
            assert_eq!(tag, "rust2025");
        } else {
            panic!("Expected hashtag node");
        }
    }

    #[test]
    fn test_parse_hashtag_only_numbers() {
        let nodes = parse("#12345");
        // Numbers-only might or might not be valid depending on implementation
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_url_with_query_params() {
        let nodes = parse("https://example.com/path?foo=bar&baz=qux");
        assert_eq!(nodes.len(), 1);
        if let MfmNodeType::Url { url, .. } = &nodes[0].node_type {
            assert!(url.contains("?foo=bar"));
        } else {
            panic!("Expected URL node");
        }
    }

    #[test]
    fn test_parse_url_with_fragment() {
        let nodes = parse("https://example.com/page#section");
        assert_eq!(nodes.len(), 1);
        if let MfmNodeType::Url { url, .. } = &nodes[0].node_type {
            assert!(url.contains("#section"));
        } else {
            panic!("Expected URL node");
        }
    }

    #[test]
    fn test_parse_nested_formatting() {
        let nodes = parse("Hello ***bold and italic*** world");
        // Should handle nested formatting
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_unclosed_bold() {
        let nodes = parse("Hello **unclosed");
        // Should handle gracefully
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_unclosed_italic() {
        let nodes = parse("Hello *unclosed");
        // Should handle gracefully
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_emoji_with_underscores() {
        let nodes = parse(":thumbs_up:");
        assert_eq!(nodes.len(), 1);
        if let MfmNodeType::Emoji { name } = &nodes[0].node_type {
            assert_eq!(name, "thumbs_up");
        } else {
            panic!("Expected emoji node");
        }
    }

    #[test]
    fn test_parse_emoji_unclosed() {
        let nodes = parse(":unclosed");
        // Should not parse as emoji
        assert!(!nodes.is_empty());
        assert!(!matches!(nodes[0].node_type, MfmNodeType::Emoji { .. }));
    }

    #[test]
    fn test_extract_mentions_empty() {
        let mentions = extract_mentions("");
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_extract_mentions_no_mentions() {
        let mentions = extract_mentions("Hello world!");
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_extract_hashtags_empty() {
        let hashtags = extract_hashtags("");
        assert!(hashtags.is_empty());
    }

    #[test]
    fn test_extract_hashtags_no_hashtags() {
        let hashtags = extract_hashtags("Hello world!");
        assert!(hashtags.is_empty());
    }

    #[test]
    fn test_to_plain_text_empty() {
        let text = to_plain_text("");
        assert_eq!(text, "");
    }

    #[test]
    fn test_to_html_empty() {
        let html = to_html("");
        assert_eq!(html, "");
    }

    #[test]
    fn test_parse_special_characters() {
        let nodes = parse("Hello <>&\"' world");
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_unicode() {
        let nodes = parse("こんにちは 🌸 @ユーザー #タグ");
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_very_long_text() {
        let long_text = "a".repeat(10000);
        let nodes = parse(&long_text);
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_parse_multiple_urls() {
        let nodes = parse("Check https://a.com and https://b.com");
        let url_count = nodes
            .iter()
            .filter(|n| matches!(n.node_type, MfmNodeType::Url { .. }))
            .count();
        assert_eq!(url_count, 2);
    }

    #[test]
    fn test_from_html_basic() {
        let mfm = from_html("<p>Hello <b>world</b></p>");
        assert!(mfm.contains("Hello"));
        assert!(mfm.contains("world"));
    }

    #[test]
    fn test_from_html_empty() {
        let mfm = from_html("");
        assert_eq!(mfm, "");
    }

    #[test]
    fn test_from_html_only_text() {
        let mfm = from_html("plain text");
        assert_eq!(mfm, "plain text");
    }
}
