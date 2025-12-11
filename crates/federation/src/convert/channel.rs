//! Channel <-> `ApGroup` conversion.

#![allow(clippy::expect_used)] // URL joins with known-valid paths cannot fail

use misskey_db::entities::channel;
use url::Url;

use crate::actors::{ApGroup, ApImage, ApPublicKey};

use super::UrlConfig;

/// Extended URL configuration for channels.
impl UrlConfig {
    /// Generate channel URL.
    #[must_use]
    pub fn channel_url(&self, channel_id: &str) -> Url {
        self.base_url
            .join(&format!("/channels/{channel_id}"))
            .expect("valid URL")
    }

    /// Generate channel inbox URL.
    #[must_use]
    pub fn channel_inbox_url(&self, channel_id: &str) -> Url {
        self.base_url
            .join(&format!("/channels/{channel_id}/inbox"))
            .expect("valid URL")
    }

    /// Generate channel outbox URL.
    #[must_use]
    pub fn channel_outbox_url(&self, channel_id: &str) -> Url {
        self.base_url
            .join(&format!("/channels/{channel_id}/outbox"))
            .expect("valid URL")
    }

    /// Generate channel followers URL.
    #[must_use]
    pub fn channel_followers_url(&self, channel_id: &str) -> Url {
        self.base_url
            .join(&format!("/channels/{channel_id}/followers"))
            .expect("valid URL")
    }

    /// Generate channel public key URL.
    #[must_use]
    pub fn channel_public_key_url(&self, channel_id: &str) -> String {
        format!("{}#main-key", self.channel_url(channel_id))
    }
}

/// Extension trait for converting Channel to `ApGroup`.
pub trait ChannelToApGroup {
    /// Convert to `ApGroup`.
    fn to_ap_group(&self, config: &UrlConfig, banner_url: Option<&str>) -> ApGroup;
}

impl ChannelToApGroup for channel::Model {
    fn to_ap_group(&self, config: &UrlConfig, banner_url: Option<&str>) -> ApGroup {
        // Use stored URI for remote channels, generate for local
        let id = if let Some(ref uri) = self.uri {
            Url::parse(uri).unwrap_or_else(|_| config.channel_url(&self.id))
        } else {
            config.channel_url(&self.id)
        };

        // Use stored inbox or generate for local
        let inbox = if let Some(ref inbox) = self.inbox {
            Url::parse(inbox).unwrap_or_else(|_| config.channel_inbox_url(&self.id))
        } else {
            config.channel_inbox_url(&self.id)
        };

        let outbox = config.channel_outbox_url(&self.id);

        // Shared inbox
        let shared_inbox = self
            .shared_inbox
            .as_ref()
            .and_then(|s| Url::parse(s).ok())
            .or_else(|| Some(config.shared_inbox_url()));

        // Banner image
        let image = banner_url.and_then(|url| {
            Url::parse(url).ok().map(|u| ApImage {
                kind: "Image".to_string(),
                url: u,
                media_type: None,
            })
        });

        // Public key for local channels
        let public_key = self.public_key_pem.as_ref().map(|pem| ApPublicKey {
            id: config.channel_public_key_url(&self.id),
            owner: id.clone(),
            public_key_pem: pem.clone(),
        });

        // Published timestamp
        let published = Some(self.created_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());

        let mut group = ApGroup::new(id.clone(), slug_from_name(&self.name), inbox, outbox);

        group.shared_inbox = shared_inbox;
        group.name = Some(self.name.clone());
        group.summary = self.description.clone();
        group.image = image;
        group.public_key = public_key;
        group.followers = Some(config.channel_followers_url(&self.id));
        group.url = Some(id);
        group.manually_approves_followers = Some(false); // Channels are open for following
        group.discoverable = Some(self.is_searchable);
        group.published = published;

        // Misskey extensions
        group.misskey_allow_anyone_to_post = Some(self.allow_anyone_to_post);
        group.misskey_is_archived = Some(self.is_archived);
        group.misskey_color = self.color.clone();

        group
    }
}

/// Generate a URL-friendly slug from channel name.
fn slug_from_name(name: &str) -> String {
    name.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c.is_whitespace() || c == '-' || c == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Extension trait for `ApGroup`.
pub trait ApGroupExt {
    /// Check if this is a local channel.
    fn is_local(&self, local_domain: &str) -> bool;

    /// Extract the host from the group ID.
    fn extract_host(&self) -> Option<String>;
}

impl ApGroupExt for ApGroup {
    fn is_local(&self, local_domain: &str) -> bool {
        self.id.host_str() == Some(local_domain)
    }

    fn extract_host(&self) -> Option<String> {
        self.id.host_str().map(std::string::ToString::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_from_name() {
        assert_eq!(slug_from_name("My Channel"), "my_channel");
        assert_eq!(slug_from_name("Hello World!"), "hello_world");
        assert_eq!(slug_from_name("Test 123"), "test_123");
        assert_eq!(slug_from_name("日本語"), "");
        assert_eq!(slug_from_name("mix日本語abc"), "mixabc");
    }
}
