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
}
