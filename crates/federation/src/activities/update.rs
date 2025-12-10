//! Update activity.

use activitypub_federation::kinds::activity::UpdateType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::actors::ApPerson;

/// `ActivityPub` Update activity.
/// Used to update an actor or object.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActivity {
    #[serde(rename = "type")]
    pub kind: UpdateType,
    pub id: Url,
    pub actor: Url,
    /// The updated object (typically an actor).
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
    Person(ApPerson),
    ObjectUrl(Url),
}

impl UpdateActivity {
    /// Create a new Update activity for a Person.
    #[must_use] 
    pub const fn new_person(id: Url, actor: Url, person: ApPerson, published: DateTime<Utc>) -> Self {
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

    /// Set the public audience.
    #[must_use]
    pub fn public(mut self) -> Self {
        self.to = Some(vec![Url::parse("https://www.w3.org/ns/activitystreams#Public").unwrap()]);
        self
    }
}
