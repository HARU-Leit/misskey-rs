//! MFM parser.

use regex::Regex;

use crate::nodes::{MfmNode, MfmNodeType};

/// Mention information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    /// Username.
    pub username: String,
    /// Host (if remote).
    pub host: Option<String>,
    /// Full acct string (@user or @user@host).
    pub acct: String,
}

// Regex patterns - these are valid static patterns that cannot fail
#[allow(clippy::unwrap_used)]
static MENTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"@([a-zA-Z0-9_]+)(?:@([a-zA-Z0-9][a-zA-Z0-9.-]*[a-zA-Z0-9]))?").unwrap()
});

#[allow(clippy::unwrap_used)]
static HASHTAG_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"#([a-zA-Z0-9_\u3040-\u309F\u30A0-\u30FF\u4E00-\u9FAF]+)").unwrap()
});

#[allow(clippy::unwrap_used)]
static URL_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"https?://[^\s<>\[\]()]+").unwrap());

#[allow(clippy::unwrap_used)]
static EMOJI_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r":([a-zA-Z0-9_+-]+):").unwrap());

#[allow(clippy::unwrap_used)]
static BOLD_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"\*\*(.+?)\*\*").unwrap());

#[allow(clippy::unwrap_used)]
static ITALIC_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"\*([^*]+)\*").unwrap());

#[allow(clippy::unwrap_used)]
static STRIKE_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"~~(.+?)~~").unwrap());

#[allow(clippy::unwrap_used)]
static INLINE_CODE_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"`([^`]+)`").unwrap());

/// Parse MFM text into an AST.
#[must_use]
#[allow(clippy::unwrap_used)] // Regex capture groups are guaranteed to exist
pub fn parse(text: &str) -> Vec<MfmNode> {
    let mut nodes = Vec::new();
    let mut pos = 0;

    // Collect all matches with their positions
    let mut matches: Vec<(usize, usize, MfmNodeType)> = Vec::new();

    // Find mentions
    for cap in MENTION_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let username = cap.get(1).unwrap().as_str().to_string();
        let host = cap.get(2).map(|h| h.as_str().to_string());
        let acct = if host.is_some() {
            format!("@{}@{}", username, host.as_ref().unwrap())
        } else {
            format!("@{username}")
        };
        matches.push((
            m.start(),
            m.end(),
            MfmNodeType::Mention {
                username,
                host,
                acct,
            },
        ));
    }

    // Find hashtags
    for cap in HASHTAG_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let tag = cap.get(1).unwrap().as_str().to_string();
        matches.push((m.start(), m.end(), MfmNodeType::Hashtag { tag }));
    }

    // Find URLs
    for cap in URL_RE.find_iter(text) {
        matches.push((
            cap.start(),
            cap.end(),
            MfmNodeType::Url {
                url: cap.as_str().to_string(),
                bracket: false,
            },
        ));
    }

    // Find emojis
    for cap in EMOJI_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let name = cap.get(1).unwrap().as_str().to_string();
        matches.push((m.start(), m.end(), MfmNodeType::Emoji { name }));
    }

    // Find bold
    for cap in BOLD_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let inner = cap.get(1).unwrap().as_str();
        let children = vec![MfmNode::new(
            MfmNodeType::Text {
                text: inner.to_string(),
            },
            m.start() + 2,
            m.end() - 2,
        )];
        matches.push((m.start(), m.end(), MfmNodeType::Bold { children }));
    }

    // Find italic (but not bold)
    for cap in ITALIC_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        // Skip if this is actually bold
        if m.start() > 0 && text[..m.start()].ends_with('*') {
            continue;
        }
        if m.end() < text.len() && text[m.end()..].starts_with('*') {
            continue;
        }
        let inner = cap.get(1).unwrap().as_str();
        let children = vec![MfmNode::new(
            MfmNodeType::Text {
                text: inner.to_string(),
            },
            m.start() + 1,
            m.end() - 1,
        )];
        matches.push((m.start(), m.end(), MfmNodeType::Italic { children }));
    }

    // Find strikethrough
    for cap in STRIKE_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let inner = cap.get(1).unwrap().as_str();
        let children = vec![MfmNode::new(
            MfmNodeType::Text {
                text: inner.to_string(),
            },
            m.start() + 2,
            m.end() - 2,
        )];
        matches.push((m.start(), m.end(), MfmNodeType::Strike { children }));
    }

    // Find inline code
    for cap in INLINE_CODE_RE.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let code = cap.get(1).unwrap().as_str().to_string();
        matches.push((m.start(), m.end(), MfmNodeType::InlineCode { code }));
    }

    // Sort matches by position
    matches.sort_by_key(|(start, _, _)| *start);

    // Remove overlapping matches (keep first)
    let mut filtered_matches: Vec<(usize, usize, MfmNodeType)> = Vec::new();
    for m in matches {
        if filtered_matches.is_empty() || m.0 >= filtered_matches.last().unwrap().1 {
            filtered_matches.push(m);
        }
    }

    // Build nodes from matches
    for (start, end, node_type) in filtered_matches {
        // Add text before this match
        if start > pos {
            nodes.push(MfmNode::new(
                MfmNodeType::Text {
                    text: text[pos..start].to_string(),
                },
                pos,
                start,
            ));
        }

        // Add the matched node
        nodes.push(MfmNode::new(node_type, start, end));
        pos = end;
    }

    // Add remaining text
    if pos < text.len() {
        nodes.push(MfmNode::new(
            MfmNodeType::Text {
                text: text[pos..].to_string(),
            },
            pos,
            text.len(),
        ));
    }

    // Handle empty input
    if nodes.is_empty() && !text.is_empty() {
        nodes.push(MfmNode::new(
            MfmNodeType::Text {
                text: text.to_string(),
            },
            0,
            text.len(),
        ));
    }

    nodes
}

/// Extract all mentions from text.
#[must_use]
#[allow(clippy::unwrap_used)] // Regex capture groups are guaranteed to exist
pub fn extract_mentions(text: &str) -> Vec<Mention> {
    MENTION_RE
        .captures_iter(text)
        .map(|cap| {
            let username = cap.get(1).unwrap().as_str().to_string();
            let host = cap.get(2).map(|h| h.as_str().to_string());
            let acct = host.as_ref().map_or_else(
                || format!("@{username}"),
                |h| format!("@{username}@{h}"),
            );
            Mention {
                username,
                host,
                acct,
            }
        })
        .collect()
}

/// Extract all hashtags from text.
#[must_use]
#[allow(clippy::unwrap_used)] // Regex capture groups are guaranteed to exist
pub fn extract_hashtags(text: &str) -> Vec<String> {
    HASHTAG_RE
        .captures_iter(text)
        .map(|cap| cap.get(1).unwrap().as_str().to_string())
        .collect()
}
