//! User <-> `ApPerson` conversion.

use misskey_db::entities::user;
use url::Url;

use crate::actors::{ApImage, ApPerson, ApPublicKey};

/// Configuration for generating user URLs.
#[derive(Clone)]
pub struct UrlConfig {
    pub base_url: Url,
}

impl UrlConfig {
    /// Create a new URL config.
    #[must_use]
    pub const fn new(base_url: Url) -> Self {
        Self { base_url }
    }

    /// Generate user URL.
    #[must_use]
    pub fn user_url(&self, username: &str) -> Url {
        self.base_url
            .join(&format!("/users/{username}"))
            .expect("valid URL")
    }

    /// Generate inbox URL.
    #[must_use]
    pub fn inbox_url(&self, username: &str) -> Url {
        self.base_url
            .join(&format!("/users/{username}/inbox"))
            .expect("valid URL")
    }

    /// Generate outbox URL.
    #[must_use]
    pub fn outbox_url(&self, username: &str) -> Url {
        self.base_url
            .join(&format!("/users/{username}/outbox"))
            .expect("valid URL")
    }

    /// Generate shared inbox URL.
    #[must_use]
    pub fn shared_inbox_url(&self) -> Url {
        self.base_url.join("/inbox").expect("valid URL")
    }

    /// Generate followers URL.
    #[must_use]
    pub fn followers_url(&self, username: &str) -> Url {
        self.base_url
            .join(&format!("/users/{username}/followers"))
            .expect("valid URL")
    }

    /// Generate following URL.
    #[must_use]
    pub fn following_url(&self, username: &str) -> Url {
        self.base_url
            .join(&format!("/users/{username}/following"))
            .expect("valid URL")
    }

    /// Generate public key URL.
    #[must_use]
    pub fn public_key_url(&self, username: &str) -> String {
        format!("{}#main-key", self.user_url(username))
    }
}

/// Extension trait for converting User to `ApPerson`.
pub trait UserToApPerson {
    /// Convert to `ApPerson`.
    fn to_ap_person(&self, config: &UrlConfig, public_key_pem: Option<&str>) -> ApPerson;
}

impl UserToApPerson for user::Model {
    fn to_ap_person(&self, config: &UrlConfig, public_key_pem: Option<&str>) -> ApPerson {
        let id = if let Some(ref uri) = self.uri {
            Url::parse(uri).unwrap_or_else(|_| config.user_url(&self.username))
        } else {
            config.user_url(&self.username)
        };

        let inbox = if let Some(ref inbox) = self.inbox {
            Url::parse(inbox).unwrap_or_else(|_| config.inbox_url(&self.username))
        } else {
            config.inbox_url(&self.username)
        };

        let outbox = config.outbox_url(&self.username);

        let shared_inbox = self
            .shared_inbox
            .as_ref()
            .and_then(|s| Url::parse(s).ok())
            .or_else(|| Some(config.shared_inbox_url()));

        let icon = self.avatar_url.as_ref().and_then(|url| {
            Url::parse(url).ok().map(|u| ApImage {
                kind: "Image".to_string(),
                url: u,
                media_type: None,
            })
        });

        let image = self.banner_url.as_ref().and_then(|url| {
            Url::parse(url).ok().map(|u| ApImage {
                kind: "Image".to_string(),
                url: u,
                media_type: None,
            })
        });

        let public_key = public_key_pem.map(|pem| ApPublicKey {
            id: config.public_key_url(&self.username),
            owner: id.clone(),
            public_key_pem: pem.to_string(),
        });

        ApPerson {
            kind: activitypub_federation::kinds::actor::PersonType::Person,
            id,
            preferred_username: self.username.clone(),
            inbox,
            outbox,
            shared_inbox,
            name: self.name.clone(),
            summary: self.description.clone(),
            icon,
            image,
            public_key,
            followers: Some(config.followers_url(&self.username)),
            following: Some(config.following_url(&self.username)),
            manually_approves_followers: Some(self.is_locked),
            discoverable: Some(true),
            misskey_summary: self.description.clone(),
            is_cat: Some(self.is_cat),
        }
    }
}

/// Extension trait for `ApPerson`.
pub trait ApPersonExt {
    /// Check if this is a local user.
    fn is_local(&self, local_domain: &str) -> bool;

    /// Extract the username from the actor ID.
    fn extract_username(&self) -> Option<String>;

    /// Extract the host from the actor ID.
    fn extract_host(&self) -> Option<String>;
}

impl ApPersonExt for ApPerson {
    fn is_local(&self, local_domain: &str) -> bool {
        self.id.host_str() == Some(local_domain)
    }

    fn extract_username(&self) -> Option<String> {
        Some(self.preferred_username.clone())
    }

    fn extract_host(&self) -> Option<String> {
        self.id.host_str().map(std::string::ToString::to_string)
    }
}
