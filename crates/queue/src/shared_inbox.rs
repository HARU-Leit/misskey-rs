//! Shared inbox optimization for `ActivityPub` delivery.
//!
//! When delivering to multiple users on the same server, we can use
//! the server's shared inbox to send the activity only once.

#![allow(missing_docs)]

use std::collections::HashMap;
use url::Url;

/// Group recipients by their server's shared inbox.
///
/// This optimization reduces the number of HTTP requests needed when
/// delivering to multiple users on the same server.
#[must_use]
pub fn group_by_shared_inbox(
    recipients: Vec<RecipientInfo>,
) -> HashMap<String, Vec<RecipientInfo>> {
    let mut groups: HashMap<String, Vec<RecipientInfo>> = HashMap::new();

    for recipient in recipients {
        let key = recipient
            .shared_inbox
            .as_ref()
            .unwrap_or(&recipient.inbox)
            .clone();
        groups.entry(key).or_default().push(recipient);
    }

    groups
}

/// Information about a delivery recipient.
#[derive(Debug, Clone)]
pub struct RecipientInfo {
    /// The recipient's actor URI.
    pub actor_uri: Url,
    /// The recipient's personal inbox URL.
    pub inbox: String,
    /// The recipient's server's shared inbox URL (if available).
    pub shared_inbox: Option<String>,
}

impl RecipientInfo {
    /// Create a new recipient info.
    #[must_use]
    pub const fn new(actor_uri: Url, inbox: String, shared_inbox: Option<String>) -> Self {
        Self {
            actor_uri,
            inbox,
            shared_inbox,
        }
    }

    /// Get the best inbox to deliver to.
    ///
    /// Prefers shared inbox if available.
    #[must_use]
    pub fn delivery_inbox(&self) -> &str {
        self.shared_inbox.as_ref().unwrap_or(&self.inbox)
    }
}

/// Batch delivery job created from grouped recipients.
#[derive(Debug, Clone)]
pub struct BatchDeliveryTarget {
    /// The inbox URL to deliver to (shared inbox if available).
    pub inbox: String,
    /// List of actor URIs being targeted by this delivery.
    pub target_actors: Vec<Url>,
    /// Whether this is a shared inbox delivery.
    pub is_shared: bool,
}

impl BatchDeliveryTarget {
    /// Create batch delivery targets from recipient info.
    #[must_use]
    pub fn from_recipients(recipients: Vec<RecipientInfo>) -> Vec<Self> {
        let groups = group_by_shared_inbox(recipients);

        groups
            .into_iter()
            .map(|(inbox, group)| {
                let is_shared = group.first().is_some_and(|r| r.shared_inbox.is_some());
                Self {
                    inbox,
                    target_actors: group.into_iter().map(|r| r.actor_uri).collect(),
                    is_shared,
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_shared_inbox() {
        let recipients = vec![
            RecipientInfo::new(
                Url::parse("https://example.com/users/alice").unwrap(),
                "https://example.com/users/alice/inbox".to_string(),
                Some("https://example.com/inbox".to_string()),
            ),
            RecipientInfo::new(
                Url::parse("https://example.com/users/bob").unwrap(),
                "https://example.com/users/bob/inbox".to_string(),
                Some("https://example.com/inbox".to_string()),
            ),
            RecipientInfo::new(
                Url::parse("https://other.com/users/charlie").unwrap(),
                "https://other.com/users/charlie/inbox".to_string(),
                None,
            ),
        ];

        let groups = group_by_shared_inbox(recipients);

        // Should have 2 groups: one for shared inbox, one for individual inbox
        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("https://example.com/inbox").unwrap().len(), 2);
        assert_eq!(
            groups
                .get("https://other.com/users/charlie/inbox")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn test_batch_delivery_target() {
        let recipients = vec![
            RecipientInfo::new(
                Url::parse("https://example.com/users/alice").unwrap(),
                "https://example.com/users/alice/inbox".to_string(),
                Some("https://example.com/inbox".to_string()),
            ),
            RecipientInfo::new(
                Url::parse("https://example.com/users/bob").unwrap(),
                "https://example.com/users/bob/inbox".to_string(),
                Some("https://example.com/inbox".to_string()),
            ),
        ];

        let targets = BatchDeliveryTarget::from_recipients(recipients);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].inbox, "https://example.com/inbox");
        assert_eq!(targets[0].target_actors.len(), 2);
        assert!(targets[0].is_shared);
    }
}
