//! Move activity for account migration.

use serde::{Deserialize, Serialize};
use url::Url;

/// `ActivityPub` Move type.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MoveType;

impl MoveType {
    /// The type name.
    pub const MOVE: &'static str = "Move";
}

/// `ActivityPub` Move activity for account migration.
///
/// A Move activity is used to signal that an actor has moved to a new account.
/// The actor (origin) sends this activity to notify followers that they should
/// now follow the target account instead.
///
/// Per `ActivityPub` spec and FEP-7628:
/// - `actor`: The old account URI
/// - `object`: The old account URI (same as actor)
/// - `target`: The new account URI to follow
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveActivity {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: Url,
    /// The actor performing the move (the old account)
    pub actor: Url,
    /// The object being moved (typically same as actor)
    pub object: Url,
    /// The target account to move to
    pub target: Url,
}

impl MoveActivity {
    /// Create a new Move activity.
    #[must_use]
    pub fn new(id: Url, actor: Url, target: Url) -> Self {
        Self {
            kind: MoveType::MOVE.to_string(),
            id,
            actor: actor.clone(),
            object: actor,
            target,
        }
    }

    /// Get the source account URI (the account being moved from).
    #[must_use]
    pub const fn source(&self) -> &Url {
        &self.actor
    }

    /// Get the destination account URI (the account being moved to).
    #[must_use]
    pub const fn destination(&self) -> &Url {
        &self.target
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_move_activity_serialization() {
        let activity = MoveActivity::new(
            Url::parse("https://example.com/activities/move/123").unwrap(),
            Url::parse("https://example.com/users/alice").unwrap(),
            Url::parse("https://newinstance.com/users/alice").unwrap(),
        );

        let json = serde_json::to_value(&activity).unwrap();
        assert_eq!(json["type"], "Move");
        assert_eq!(json["actor"], "https://example.com/users/alice");
        assert_eq!(json["object"], "https://example.com/users/alice");
        assert_eq!(json["target"], "https://newinstance.com/users/alice");
    }

    #[test]
    fn test_move_activity_deserialization() {
        let json = r#"{
            "type": "Move",
            "id": "https://example.com/activities/move/123",
            "actor": "https://example.com/users/alice",
            "object": "https://example.com/users/alice",
            "target": "https://newinstance.com/users/alice"
        }"#;

        let activity: MoveActivity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.kind, "Move");
        assert_eq!(activity.actor.as_str(), "https://example.com/users/alice");
        assert_eq!(
            activity.target.as_str(),
            "https://newinstance.com/users/alice"
        );
    }
}
