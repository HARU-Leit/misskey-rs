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
    Text { text: String },

    /// Bold text (**text** or __text__).
    Bold { children: Vec<MfmNode> },

    /// Italic text (*text* or _text_).
    Italic { children: Vec<MfmNode> },

    /// Strikethrough text (~~text~~).
    Strike { children: Vec<MfmNode> },

    /// Inline code (`code`).
    InlineCode { code: String },

    /// Code block (```code```).
    CodeBlock { code: String, lang: Option<String> },

    /// Quote (> text).
    Quote { children: Vec<MfmNode> },

    /// Mention (@user or @user@host).
    Mention {
        username: String,
        host: Option<String>,
        acct: String,
    },

    /// Hashtag (#tag).
    Hashtag { tag: String },

    /// URL.
    Url { url: String, bracket: bool },

    /// Link [text](url).
    Link {
        url: String,
        children: Vec<MfmNode>,
        silent: bool,
    },

    /// Custom emoji (:emoji:).
    Emoji { name: String },

    /// Unicode emoji.
    UnicodeEmoji { emoji: String },

    /// Misskey function $[fn.name args content].
    Fn {
        name: String,
        args: std::collections::HashMap<String, String>,
        children: Vec<MfmNode>,
    },

    /// Plain block (no MFM processing).
    Plain { text: String },

    /// Center block (<center>).
    Center { children: Vec<MfmNode> },

    /// Small text (<small>).
    Small { children: Vec<MfmNode> },

    /// Search block (search keyword).
    Search { query: String, content: String },

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
