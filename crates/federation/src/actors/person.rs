//! `ActivityPub` Person actor.

use activitypub_federation::kinds::actor::PersonType;
use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Person actor.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApPerson {
    #[serde(rename = "type")]
    pub kind: PersonType,
    pub id: Url,
    pub preferred_username: String,
    pub inbox: Url,
    pub outbox: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_inbox: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ApImage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ApImage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<ApPublicKey>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub manually_approves_followers: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,

    /// URI of the account this user has moved to (for account migration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moved_to: Option<Url>,

    /// List of alternative account URIs (for account migration verification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub also_known_as: Option<Vec<Url>>,

    // Misskey extensions
    #[serde(rename = "_misskey_summary", skip_serializing_if = "Option::is_none")]
    pub misskey_summary: Option<String>,

    #[serde(rename = "isCat", skip_serializing_if = "Option::is_none")]
    pub is_cat: Option<bool>,
}

/// `ActivityPub` Image object.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApImage {
    #[serde(rename = "type")]
    pub kind: String,
    pub url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

/// `ActivityPub` public key.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApPublicKey {
    pub id: String,
    pub owner: Url,
    pub public_key_pem: String,
}

impl ApPerson {
    /// Create a new Person actor.
    #[must_use]
    pub const fn new(id: Url, username: String, inbox: Url, outbox: Url) -> Self {
        Self {
            kind: PersonType::Person,
            id,
            preferred_username: username,
            inbox,
            outbox,
            shared_inbox: None,
            name: None,
            summary: None,
            icon: None,
            image: None,
            public_key: None,
            followers: None,
            following: None,
            manually_approves_followers: None,
            discoverable: None,
            moved_to: None,
            also_known_as: None,
            misskey_summary: None,
            is_cat: None,
        }
    }

    /// Check if this actor has moved to another account.
    #[must_use]
    pub const fn is_moved(&self) -> bool {
        self.moved_to.is_some()
    }
}
