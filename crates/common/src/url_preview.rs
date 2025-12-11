//! URL preview (Summaly-like) functionality.
//!
//! Fetches metadata from URLs to generate rich link previews.

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use url::Url;

/// URL preview metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlPreview {
    /// Page title.
    pub title: Option<String>,
    /// Page description.
    pub description: Option<String>,
    /// Preview image URL.
    pub image: Option<String>,
    /// Site name.
    pub site_name: Option<String>,
    /// Icon/favicon URL.
    pub icon: Option<String>,
    /// Original URL.
    pub url: String,
    /// Whether the URL points to sensitive content.
    pub sensitive: bool,
}

/// URL preview fetcher configuration.
#[derive(Debug, Clone)]
pub struct UrlPreviewConfig {
    /// User agent string.
    pub user_agent: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum response size in bytes.
    pub max_size: usize,
}

impl Default for UrlPreviewConfig {
    fn default() -> Self {
        Self {
            user_agent: "Misskey/1.0 (compatible; URLPreview)".to_string(),
            timeout_secs: 10,
            max_size: 1024 * 1024, // 1MB
        }
    }
}

// Regex patterns for extracting metadata
static TITLE_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"<title[^>]*>([^<]*)</title>").unwrap());

static OG_TITLE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+property=["']og:title["'][^>]+content=["']([^"']*)["']"#).unwrap()
});

static OG_TITLE_RE2: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+content=["']([^"']*)["'][^>]+property=["']og:title["']"#).unwrap()
});

static OG_DESC_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+property=["']og:description["'][^>]+content=["']([^"']*)["']"#).unwrap()
});

static OG_DESC_RE2: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+content=["']([^"']*)["'][^>]+property=["']og:description["']"#).unwrap()
});

static META_DESC_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+name=["']description["'][^>]+content=["']([^"']*)["']"#).unwrap()
});

static META_DESC_RE2: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+content=["']([^"']*)["'][^>]+name=["']description["']"#).unwrap()
});

static OG_IMAGE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+property=["']og:image["'][^>]+content=["']([^"']*)["']"#).unwrap()
});

static OG_IMAGE_RE2: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+content=["']([^"']*)["'][^>]+property=["']og:image["']"#).unwrap()
});

static OG_SITE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+property=["']og:site_name["'][^>]+content=["']([^"']*)["']"#).unwrap()
});

static OG_SITE_RE2: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<meta[^>]+content=["']([^"']*)["'][^>]+property=["']og:site_name["']"#).unwrap()
});

static ICON_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<link[^>]+rel=["'](?:shortcut )?icon["'][^>]+href=["']([^"']*)["']"#).unwrap()
});

static ICON_RE2: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#"<link[^>]+href=["']([^"']*)["'][^>]+rel=["'](?:shortcut )?icon["']"#).unwrap()
});

/// Fetch URL preview metadata.
pub async fn fetch_preview(url: &str, config: &UrlPreviewConfig) -> Option<UrlPreview> {
    // Validate URL
    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            warn!("Invalid URL: {} - {}", url, e);
            return None;
        }
    };

    // Only allow HTTP(S)
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        debug!("Skipping non-HTTP URL: {}", url);
        return None;
    }

    // Create HTTP client
    let client = Client::builder()
        .user_agent(&config.user_agent)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()
        .ok()?;

    // Fetch the page
    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to fetch URL: {} - {}", url, e);
            return None;
        }
    };

    // Check status
    if !response.status().is_success() {
        debug!(
            "URL returned non-success status: {} - {}",
            url,
            response.status()
        );
        return None;
    }

    // Check content type
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.contains("text/html") && !content_type.contains("application/xhtml") {
        debug!("URL is not HTML: {} - {}", url, content_type);
        return None;
    }

    // Get response body (limited size)
    let body = match response.text().await {
        Ok(b) => {
            if b.len() > config.max_size {
                b[..config.max_size].to_string()
            } else {
                b
            }
        }
        Err(e) => {
            warn!("Failed to read response body: {} - {}", url, e);
            return None;
        }
    };

    // Extract metadata
    let title = extract_title(&body);
    let description = extract_description(&body);
    let image = extract_image(&body, &parsed_url);
    let site_name = extract_site_name(&body);
    let icon = extract_icon(&body, &parsed_url);

    Some(UrlPreview {
        title,
        description,
        image,
        site_name,
        icon,
        url: url.to_string(),
        sensitive: false,
    })
}

/// Extract page title.
fn extract_title(html: &str) -> Option<String> {
    // Try og:title first
    if let Some(cap) = OG_TITLE_RE
        .captures(html)
        .or_else(|| OG_TITLE_RE2.captures(html))
    {
        return Some(decode_html_entities(cap.get(1)?.as_str()));
    }

    // Fall back to <title>
    if let Some(cap) = TITLE_RE.captures(html) {
        return Some(decode_html_entities(cap.get(1)?.as_str()));
    }

    None
}

/// Extract page description.
fn extract_description(html: &str) -> Option<String> {
    // Try og:description first
    if let Some(cap) = OG_DESC_RE
        .captures(html)
        .or_else(|| OG_DESC_RE2.captures(html))
    {
        return Some(decode_html_entities(cap.get(1)?.as_str()));
    }

    // Fall back to meta description
    if let Some(cap) = META_DESC_RE
        .captures(html)
        .or_else(|| META_DESC_RE2.captures(html))
    {
        return Some(decode_html_entities(cap.get(1)?.as_str()));
    }

    None
}

/// Extract preview image URL.
fn extract_image(html: &str, base_url: &Url) -> Option<String> {
    let cap = OG_IMAGE_RE
        .captures(html)
        .or_else(|| OG_IMAGE_RE2.captures(html))?;
    let image_url = cap.get(1)?.as_str();
    resolve_url(image_url, base_url)
}

/// Extract site name.
fn extract_site_name(html: &str) -> Option<String> {
    let cap = OG_SITE_RE
        .captures(html)
        .or_else(|| OG_SITE_RE2.captures(html))?;
    Some(decode_html_entities(cap.get(1)?.as_str()))
}

/// Extract favicon URL.
fn extract_icon(html: &str, base_url: &Url) -> Option<String> {
    if let Some(cap) = ICON_RE.captures(html).or_else(|| ICON_RE2.captures(html)) {
        let icon_url = cap.get(1)?.as_str();
        return resolve_url(icon_url, base_url);
    }

    // Default favicon location
    let favicon_url = format!(
        "{}://{}/favicon.ico",
        base_url.scheme(),
        base_url.host_str()?
    );
    Some(favicon_url)
}

/// Resolve a potentially relative URL against a base URL.
fn resolve_url(url: &str, base: &Url) -> Option<String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Some(url.to_string())
    } else if url.starts_with("//") {
        Some(format!("{}:{}", base.scheme(), url))
    } else if url.starts_with('/') {
        Some(format!("{}://{}{}", base.scheme(), base.host_str()?, url))
    } else {
        // Relative path
        base.join(url).ok().map(|u| u.to_string())
    }
}

/// Decode common HTML entities.
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&#x2F;", "/")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let html = r"<html><head><title>Test Page</title></head></html>";
        assert_eq!(extract_title(html), Some("Test Page".to_string()));
    }

    #[test]
    fn test_extract_og_title() {
        let html = r#"<html><head><meta property="og:title" content="OG Title"></head></html>"#;
        assert_eq!(extract_title(html), Some("OG Title".to_string()));
    }

    #[test]
    fn test_extract_description() {
        let html = r#"<html><head><meta name="description" content="A test page"></head></html>"#;
        assert_eq!(extract_description(html), Some("A test page".to_string()));
    }

    #[test]
    fn test_extract_og_description() {
        let html = r#"<html><head><meta property="og:description" content="OG Description"></head></html>"#;
        assert_eq!(
            extract_description(html),
            Some("OG Description".to_string())
        );
    }

    #[test]
    fn test_extract_image() {
        let html = r#"<html><head><meta property="og:image" content="https://example.com/image.png"></head></html>"#;
        let base = Url::parse("https://example.com/").unwrap();
        assert_eq!(
            extract_image(html, &base),
            Some("https://example.com/image.png".to_string())
        );
    }

    #[test]
    fn test_extract_image_relative() {
        let html =
            r#"<html><head><meta property="og:image" content="/images/preview.png"></head></html>"#;
        let base = Url::parse("https://example.com/page").unwrap();
        assert_eq!(
            extract_image(html, &base),
            Some("https://example.com/images/preview.png".to_string())
        );
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("Hello &amp; World"), "Hello & World");
        assert_eq!(decode_html_entities("&lt;script&gt;"), "<script>");
        assert_eq!(
            decode_html_entities("She said &quot;Hi&quot;"),
            "She said \"Hi\""
        );
    }

    #[test]
    fn test_resolve_url_absolute() {
        let base = Url::parse("https://example.com/page").unwrap();
        assert_eq!(
            resolve_url("https://other.com/img.png", &base),
            Some("https://other.com/img.png".to_string())
        );
    }

    #[test]
    fn test_resolve_url_relative() {
        let base = Url::parse("https://example.com/page/").unwrap();
        assert_eq!(
            resolve_url("img.png", &base),
            Some("https://example.com/page/img.png".to_string())
        );
    }

    #[test]
    fn test_resolve_url_protocol_relative() {
        let base = Url::parse("https://example.com/").unwrap();
        assert_eq!(
            resolve_url("//cdn.example.com/img.png", &base),
            Some("https://cdn.example.com/img.png".to_string())
        );
    }
}
