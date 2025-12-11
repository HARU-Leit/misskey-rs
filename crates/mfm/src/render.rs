//! MFM rendering to HTML and plain text.

use crate::nodes::{MfmNode, MfmNodeType};

/// Render MFM nodes to HTML.
#[must_use]
pub fn to_html(text: &str) -> String {
    let nodes = crate::parse(text);
    nodes_to_html(&nodes)
}

/// Render MFM nodes to plain text.
#[must_use]
pub fn to_plain_text(text: &str) -> String {
    let nodes = crate::parse(text);
    nodes_to_plain_text(&nodes)
}

/// Convert nodes to HTML.
fn nodes_to_html(nodes: &[MfmNode]) -> String {
    nodes.iter().map(node_to_html).collect()
}

/// Convert a single node to HTML.
fn node_to_html(node: &MfmNode) -> String {
    match &node.node_type {
        MfmNodeType::Text { text } => html_escape(text),
        MfmNodeType::Bold { children } => {
            format!("<b>{}</b>", nodes_to_html(children))
        }
        MfmNodeType::Italic { children } => {
            format!("<i>{}</i>", nodes_to_html(children))
        }
        MfmNodeType::Strike { children } => {
            format!("<del>{}</del>", nodes_to_html(children))
        }
        MfmNodeType::InlineCode { code } => {
            format!("<code>{}</code>", html_escape(code))
        }
        MfmNodeType::CodeBlock { code, lang } => {
            if let Some(l) = lang {
                format!(
                    "<pre><code class=\"language-{}\">{}</code></pre>",
                    html_escape(l),
                    html_escape(code)
                )
            } else {
                format!("<pre><code>{}</code></pre>", html_escape(code))
            }
        }
        MfmNodeType::Quote { children } => {
            format!("<blockquote>{}</blockquote>", nodes_to_html(children))
        }
        MfmNodeType::Mention {
            username,
            host,
            acct,
        } => {
            let href = if let Some(h) = host {
                format!("https://{h}/@{username}")
            } else {
                format!("/@{username}")
            };
            format!(
                "<a href=\"{}\" class=\"mention\">{}</a>",
                html_escape(&href),
                html_escape(acct)
            )
        }
        MfmNodeType::Hashtag { tag } => {
            format!(
                "<a href=\"/tags/{}\" class=\"hashtag\">#{}</a>",
                html_escape(tag),
                html_escape(tag)
            )
        }
        MfmNodeType::Url { url, .. } => {
            format!(
                "<a href=\"{}\" rel=\"nofollow noopener\" target=\"_blank\">{}</a>",
                html_escape(url),
                html_escape(url)
            )
        }
        MfmNodeType::Link { url, children, .. } => {
            format!(
                "<a href=\"{}\" rel=\"nofollow noopener\" target=\"_blank\">{}</a>",
                html_escape(url),
                nodes_to_html(children)
            )
        }
        MfmNodeType::Emoji { name } => {
            format!(
                "<span class=\"emoji\" data-emoji=\"{}\">:{name}:</span>",
                html_escape(name)
            )
        }
        MfmNodeType::UnicodeEmoji { emoji } => emoji.clone(),
        MfmNodeType::Fn {
            name,
            args: _,
            children,
        } => {
            // Simple handling of MFM functions
            match name.as_str() {
                "flip" => format!(
                    "<span style=\"display: inline-block; transform: scaleX(-1);\">{}</span>",
                    nodes_to_html(children)
                ),
                "rotate" => format!(
                    "<span style=\"display: inline-block; transform: rotate(90deg);\">{}</span>",
                    nodes_to_html(children)
                ),
                "x2" => format!(
                    "<span style=\"font-size: 200%;\">{}</span>",
                    nodes_to_html(children)
                ),
                "x3" => format!(
                    "<span style=\"font-size: 300%;\">{}</span>",
                    nodes_to_html(children)
                ),
                "x4" => format!(
                    "<span style=\"font-size: 400%;\">{}</span>",
                    nodes_to_html(children)
                ),
                "blur" => format!(
                    "<span style=\"filter: blur(6px);\">{}</span>",
                    nodes_to_html(children)
                ),
                "rainbow" => format!(
                    "<span class=\"mfm-rainbow\">{}</span>",
                    nodes_to_html(children)
                ),
                "sparkle" => format!(
                    "<span class=\"mfm-sparkle\">{}</span>",
                    nodes_to_html(children)
                ),
                _ => nodes_to_html(children),
            }
        }
        MfmNodeType::Plain { text } => html_escape(text),
        MfmNodeType::Center { children } => {
            format!(
                "<div style=\"text-align: center;\">{}</div>",
                nodes_to_html(children)
            )
        }
        MfmNodeType::Small { children } => {
            format!(
                "<small style=\"opacity: 0.7;\">{}</small>",
                nodes_to_html(children)
            )
        }
        MfmNodeType::Search { query, content } => {
            format!(
                "<div class=\"mfm-search\"><input type=\"text\" value=\"{}\" readonly /><a href=\"https://www.google.com/search?q={}\" target=\"_blank\">{}</a></div>",
                html_escape(query),
                html_escape(query),
                html_escape(content)
            )
        }
        MfmNodeType::LineBreak => "<br />".to_string(),
    }
}

/// Convert nodes to plain text.
fn nodes_to_plain_text(nodes: &[MfmNode]) -> String {
    nodes.iter().map(node_to_plain_text).collect()
}

/// Convert a single node to plain text.
fn node_to_plain_text(node: &MfmNode) -> String {
    match &node.node_type {
        MfmNodeType::Text { text } => text.clone(),
        MfmNodeType::Bold { children }
        | MfmNodeType::Italic { children }
        | MfmNodeType::Strike { children }
        | MfmNodeType::Quote { children }
        | MfmNodeType::Center { children }
        | MfmNodeType::Small { children } => nodes_to_plain_text(children),
        MfmNodeType::InlineCode { code } | MfmNodeType::CodeBlock { code, .. } => code.clone(),
        MfmNodeType::Mention { acct, .. } => acct.clone(),
        MfmNodeType::Hashtag { tag } => format!("#{tag}"),
        MfmNodeType::Url { url, .. } => url.clone(),
        MfmNodeType::Link { children, .. } => nodes_to_plain_text(children),
        MfmNodeType::Emoji { name } => format!(":{name}:"),
        MfmNodeType::UnicodeEmoji { emoji } => emoji.clone(),
        MfmNodeType::Fn { children, .. } => nodes_to_plain_text(children),
        MfmNodeType::Plain { text } => text.clone(),
        MfmNodeType::Search { content, .. } => content.clone(),
        MfmNodeType::LineBreak => "\n".to_string(),
    }
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Convert HTML to MFM (basic conversion).
/// This is a simplified implementation that handles common HTML elements.
#[must_use]
pub fn from_html(html: &str) -> String {
    let mut result = html.to_string();

    // First, decode HTML entities
    result = html_unescape(&result);

    // Convert common HTML tags to MFM
    result = result
        // Bold
        .replace("<b>", "**")
        .replace("</b>", "**")
        .replace("<strong>", "**")
        .replace("</strong>", "**")
        // Italic
        .replace("<i>", "_")
        .replace("</i>", "_")
        .replace("<em>", "_")
        .replace("</em>", "_")
        // Strikethrough
        .replace("<del>", "~~")
        .replace("</del>", "~~")
        .replace("<s>", "~~")
        .replace("</s>", "~~")
        // Line breaks
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        // Paragraphs
        .replace("<p>", "")
        .replace("</p>", "\n\n");

    // Handle blockquotes
    result = convert_blockquotes(&result);

    // Handle links
    result = convert_links(&result);

    // Handle code blocks
    result = convert_code_blocks(&result);

    // Handle inline code
    result = convert_inline_code(&result);

    // Clean up remaining tags
    result = strip_remaining_tags(&result);

    // Clean up excessive newlines
    result = clean_newlines(&result);

    result.trim().to_string()
}

/// Unescape HTML entities.
fn html_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

/// Convert blockquotes to MFM format.
fn convert_blockquotes(html: &str) -> String {
    let mut result = html.to_string();

    // Simple blockquote conversion
    while let (Some(start), Some(end)) = (result.find("<blockquote>"), result.find("</blockquote>"))
    {
        if start < end {
            let content = &result[start + 12..end];
            let quoted = content
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            result = format!("{}{}{}", &result[..start], quoted, &result[end + 13..]);
        } else {
            break;
        }
    }

    result
}

/// Convert links to MFM format.
fn convert_links(html: &str) -> String {
    use regex::Regex;

    let link_re = Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>([^<]*)</a>"#).unwrap();
    link_re
        .replace_all(html, |caps: &regex::Captures| {
            let url = &caps[1];
            let text = &caps[2];
            if url == text || text.is_empty() {
                url.to_string()
            } else {
                format!("[{text}]({url})")
            }
        })
        .to_string()
}

/// Convert code blocks to MFM format.
fn convert_code_blocks(html: &str) -> String {
    use regex::Regex;

    let code_re =
        Regex::new(r#"<pre><code(?:\s+class="language-(\w+)")?>([^<]*)</code></pre>"#).unwrap();

    code_re
        .replace_all(html, |caps: &regex::Captures| {
            let lang = caps.get(1).map_or("", |m| m.as_str());
            let code = &caps[2];
            if lang.is_empty() {
                format!("```\n{code}\n```")
            } else {
                format!("```{lang}\n{code}\n```")
            }
        })
        .to_string()
}

/// Convert inline code to MFM format.
fn convert_inline_code(html: &str) -> String {
    use regex::Regex;

    let code_re = Regex::new(r"<code>([^<]*)</code>").unwrap();
    code_re.replace_all(html, "`$1`").to_string()
}

/// Strip remaining HTML tags.
fn strip_remaining_tags(html: &str) -> String {
    use regex::Regex;

    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    tag_re.replace_all(html, "").to_string()
}

/// Clean up excessive newlines.
fn clean_newlines(s: &str) -> String {
    use regex::Regex;

    let newline_re = Regex::new(r"\n{3,}").unwrap();
    newline_re.replace_all(s, "\n\n").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn test_render_bold_html() {
        let html = to_html("Hello **world**!");
        assert!(html.contains("<b>world</b>"));
    }

    #[test]
    fn test_render_mention_html() {
        let html = to_html("Hello @user@example.com!");
        assert!(html.contains("class=\"mention\""));
        assert!(html.contains("@user@example.com"));
    }

    #[test]
    fn test_render_plain_text() {
        let plain = to_plain_text("Hello **world** and @user!");
        assert_eq!(plain, "Hello world and @user!");
    }

    #[test]
    fn test_from_html_bold() {
        let mfm = from_html("<b>bold</b>");
        assert_eq!(mfm, "**bold**");
    }

    #[test]
    fn test_from_html_italic() {
        let mfm = from_html("<em>italic</em>");
        assert_eq!(mfm, "_italic_");
    }

    #[test]
    fn test_from_html_strikethrough() {
        let mfm = from_html("<del>deleted</del>");
        assert_eq!(mfm, "~~deleted~~");
    }

    #[test]
    fn test_from_html_link() {
        let mfm = from_html("<a href=\"https://example.com\">Example</a>");
        assert_eq!(mfm, "[Example](https://example.com)");
    }

    #[test]
    fn test_from_html_link_url_only() {
        let mfm = from_html("<a href=\"https://example.com\">https://example.com</a>");
        assert_eq!(mfm, "https://example.com");
    }

    #[test]
    fn test_from_html_code() {
        let mfm = from_html("<code>code</code>");
        assert_eq!(mfm, "`code`");
    }

    #[test]
    fn test_from_html_entities() {
        let mfm = from_html("hello &amp; world &quot;quotes&quot;");
        assert_eq!(mfm, "hello & world \"quotes\"");
    }
}
