//! MFM AST nodes.

use serde::{Deserialize, Serialize};

/// MFM node in the AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfmNode {
    /// The type of node.
    #[serde(flatten)]
    pub node_type: MfmNodeType,
    /// Start position in the source text.
    pub start: usize,
    /// End position in the source text.
    pub end: usize,
}

impl MfmNode {
    /// Create a new MFM node.
    #[must_use]
    pub fn new(node_type: MfmNodeType, start: usize, end: usize) -> Self {
        Self {
            node_type,
            start,
            end,
        }
    }
}

/// Types of MFM nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "props", rename_all = "camelCase")]
pub enum MfmNodeType {
    /// Plain text.
    Text {
        /// The text content.
        text: String,
    },

    /// Bold text (**text** or __text__).
    Bold {
        /// Child nodes within the bold formatting.
        children: Vec<MfmNode>,
    },

    /// Italic text (*text* or _text_).
    Italic {
        /// Child nodes within the italic formatting.
        children: Vec<MfmNode>,
    },

    /// Strikethrough text (~~text~~).
    Strike {
        /// Child nodes within the strikethrough formatting.
        children: Vec<MfmNode>,
    },

    /// Inline code (`code`).
    InlineCode {
        /// The code content.
        code: String,
    },

    /// Code block (```code```).
    CodeBlock {
        /// The code content.
        code: String,
        /// The programming language (if specified).
        lang: Option<String>,
    },

    /// Quote (> text).
    Quote {
        /// Child nodes within the quote.
        children: Vec<MfmNode>,
    },

    /// Mention (@user or @user@host).
    Mention {
        /// The username being mentioned.
        username: String,
        /// The host of the mentioned user (for federated mentions).
        host: Option<String>,
        /// The full account string (e.g., "@user@host").
        acct: String,
    },

    /// Hashtag (#tag).
    Hashtag {
        /// The tag name (without the # prefix).
        tag: String,
    },

    /// URL.
    Url {
        /// The URL string.
        url: String,
        /// Whether the URL is wrapped in angle brackets.
        bracket: bool,
    },

    /// Link [text](url).
    Link {
        /// The URL the link points to.
        url: String,
        /// Child nodes (the link text).
        children: Vec<MfmNode>,
        /// Whether the link should be silent (no preview).
        silent: bool,
    },

    /// Custom emoji (:emoji:).
    Emoji {
        /// The emoji name (without colons).
        name: String,
    },

    /// Unicode emoji.
    UnicodeEmoji {
        /// The Unicode emoji character(s).
        emoji: String,
    },

    /// Misskey function $[fn.name args content].
    Fn {
        /// The function name.
        name: String,
        /// The function arguments.
        args: std::collections::HashMap<String, String>,
        /// Child nodes within the function.
        children: Vec<MfmNode>,
    },

    /// Plain block (no MFM processing).
    Plain {
        /// The plain text content.
        text: String,
    },

    /// Center block (`<center>`).
    Center {
        /// Child nodes within the center block.
        children: Vec<MfmNode>,
    },

    /// Small text (`<small>`).
    Small {
        /// Child nodes within the small text block.
        children: Vec<MfmNode>,
    },

    /// Search block (search keyword).
    Search {
        /// The search query.
        query: String,
        /// The full content of the search line.
        content: String,
    },

    /// Line break.
    LineBreak,
}

impl MfmNodeType {
    /// Get the type name as a string.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::Bold { .. } => "bold",
            Self::Italic { .. } => "italic",
            Self::Strike { .. } => "strike",
            Self::InlineCode { .. } => "inlineCode",
            Self::CodeBlock { .. } => "codeBlock",
            Self::Quote { .. } => "quote",
            Self::Mention { .. } => "mention",
            Self::Hashtag { .. } => "hashtag",
            Self::Url { .. } => "url",
            Self::Link { .. } => "link",
            Self::Emoji { .. } => "emoji",
            Self::UnicodeEmoji { .. } => "unicodeEmoji",
            Self::Fn { .. } => "fn",
            Self::Plain { .. } => "plain",
            Self::Center { .. } => "center",
            Self::Small { .. } => "small",
            Self::Search { .. } => "search",
            Self::LineBreak => "lineBreak",
        }
    }
}
