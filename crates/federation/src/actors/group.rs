//! `ActivityPub` Group actor for channels.

use activitypub_federation::kinds::actor::GroupType;
use serde::{Deserialize, Serialize};
use url::Url;

use super::person::{ApImage, ApPublicKey};

/// `ActivityPub` Group actor representing a channel.
///
/// Used for federating channel content across ActivityPub instances.
/// Follows the W3C ActivityPub specification for Group actors.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApGroup {
    #[serde(rename = "type")]
    pub kind: GroupType,

    /// Unique identifier (URI) of this Group actor.
    pub id: Url,

    /// Unique username/handle for this group (channel name slug).
    pub preferred_username: String,

    /// Inbox URL for receiving activities.
    pub inbox: Url,

    /// Outbox URL for published activities.
    pub outbox: Url,

    /// Shared inbox URL for efficient delivery (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_inbox: Option<Url>,

    /// Display name of the group (channel name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Description/summary of the group (channel description).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Icon/avatar image for the group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ApImage>,

    /// Banner/header image for the group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ApImage>,

    /// Public key for verifying HTTP signatures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<ApPublicKey>,

    /// URL to the followers collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<Url>,

    /// URL to the human-readable page for this group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,

    /// Whether following this group requires approval.
    /// For channels: typically false (open following).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manually_approves_followers: Option<bool>,

    /// Whether this group is discoverable/indexable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,

    /// Timestamp when this group was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,

    /// Owner/attributed to (for channels, the creator).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributed_to: Option<Url>,

    // Misskey extension: channel-specific metadata
    /// Whether anyone can post to this channel.
    #[serde(
        rename = "_misskey_allowAnyoneToPost",
        skip_serializing_if = "Option::is_none"
    )]
    pub misskey_allow_anyone_to_post: Option<bool>,

    /// Whether the channel is archived (read-only).
    #[serde(
        rename = "_misskey_isArchived",
        skip_serializing_if = "Option::is_none"
    )]
    pub misskey_is_archived: Option<bool>,

    /// Channel color (hex).
    #[serde(rename = "_misskey_color", skip_serializing_if = "Option::is_none")]
    pub misskey_color: Option<String>,
}

impl ApGroup {
    /// Create a new Group actor with minimal required fields.
    #[must_use]
    pub fn new(id: Url, preferred_username: String, inbox: Url, outbox: Url) -> Self {
        Self {
            kind: GroupType::Group,
            id,
            preferred_username,
            inbox,
            outbox,
            shared_inbox: None,
            name: None,
            summary: None,
            icon: None,
            image: None,
            public_key: None,
            followers: None,
            url: None,
            manually_approves_followers: None,
            discoverable: None,
            published: None,
            attributed_to: None,
            misskey_allow_anyone_to_post: None,
            misskey_is_archived: None,
            misskey_color: None,
        }
    }

    /// Set the display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description/summary.
    #[must_use]
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Set the public key for signature verification.
    #[must_use]
    pub fn with_public_key(mut self, public_key: ApPublicKey) -> Self {
        self.public_key = Some(public_key);
        self
    }

    /// Set the banner image.
    #[must_use]
    pub fn with_image(mut self, image: ApImage) -> Self {
        self.image = Some(image);
        self
    }

    /// Set the followers collection URL.
    #[must_use]
    pub fn with_followers(mut self, followers: Url) -> Self {
        self.followers = Some(followers);
        self
    }

    /// Set the human-readable URL.
    #[must_use]
    pub fn with_url(mut self, url: Url) -> Self {
        self.url = Some(url);
        self
    }

    /// Set whether following requires approval.
    #[must_use]
    pub const fn with_manually_approves_followers(mut self, value: bool) -> Self {
        self.manually_approves_followers = Some(value);
        self
    }

    /// Set whether this group is discoverable.
    #[must_use]
    pub const fn with_discoverable(mut self, value: bool) -> Self {
        self.discoverable = Some(value);
        self
    }

    /// Set the published timestamp.
    #[must_use]
    pub fn with_published(mut self, published: impl Into<String>) -> Self {
        self.published = Some(published.into());
        self
    }

    /// Set the owner/creator.
    #[must_use]
    pub fn with_attributed_to(mut self, attributed_to: Url) -> Self {
        self.attributed_to = Some(attributed_to);
        self
    }

    /// Set Misskey channel-specific options.
    #[must_use]
    pub fn with_misskey_channel_options(
        mut self,
        allow_anyone_to_post: bool,
        is_archived: bool,
        color: Option<String>,
    ) -> Self {
        self.misskey_allow_anyone_to_post = Some(allow_anyone_to_post);
        self.misskey_is_archived = Some(is_archived);
        self.misskey_color = color;
        self
    }
}
