//! Update activity.

use activitypub_federation::kinds::activity::UpdateType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::actors::ApPerson;
use crate::objects::ApNote;

/// `ActivityPub` Update activity.
/// Used to update an actor or object.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActivity {
    #[serde(rename = "type")]
    pub kind: UpdateType,
    pub id: Url,
    pub actor: Url,
    /// The updated object (actor or note).
    pub object: UpdateObject,
    pub published: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<Url>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<Url>>,
}

/// Object that can be updated.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UpdateObject {
    /// A full Person object (for actor profile updates).
    Person(ApPerson),
    /// A full Note object (for note content updates).
    Note(ApNote),
    /// Just a URL reference to an object.
    ObjectUrl(Url),
}

impl UpdateActivity {
    /// Create a new Update activity for a Person.
    #[must_use]
    pub const fn new_person(
        id: Url,
        actor: Url,
        person: ApPerson,
        published: DateTime<Utc>,
    ) -> Self {
        Self {
            kind: UpdateType::Update,
            id,
            actor,
            object: UpdateObject::Person(person),
            published,
            to: None,
            cc: None,
        }
    }

    /// Create a new Update activity for a Note.
    #[must_use]
    pub const fn new_note(id: Url, actor: Url, note: ApNote, published: DateTime<Utc>) -> Self {
        Self {
            kind: UpdateType::Update,
            id,
            actor,
            object: UpdateObject::Note(note),
            published,
            to: None,
            cc: None,
        }
    }

    /// Set the public audience.
    #[must_use]
    #[allow(clippy::unwrap_used)] // Static URL is known to be valid
    pub fn public(mut self) -> Self {
        self.to = Some(vec![
            Url::parse("https://www.w3.org/ns/activitystreams#Public").unwrap(),
        ]);
        self
    }

    /// Set specific audiences (to and cc).
    #[must_use]
    pub fn with_audience(mut self, to: Vec<Url>, cc: Vec<Url>) -> Self {
        self.to = if to.is_empty() { None } else { Some(to) };
        self.cc = if cc.is_empty() { None } else { Some(cc) };
        self
    }
}
