//! `ActivityPub` delivery service.
//!
//! Handles creating and queueing activities for delivery to remote servers.

#![allow(missing_docs)]

use misskey_common::{AppResult, IdGenerator};
use misskey_db::{
    entities::{note, user},
    repositories::{FollowingRepository, UserRepository},
};
use serde_json::{json, Value};
use tracing::info;
use url::Url;


/// Delivery service for queueing `ActivityPub` activities.
#[derive(Clone)]
pub struct DeliveryService {
    user_repo: UserRepository,
    following_repo: FollowingRepository,
    id_gen: IdGenerator,
    base_url: Url,
}

/// Activity to be delivered.
pub struct DeliveryTarget {
    pub user_id: String,
    pub inbox_url: String,
    pub activity: Value,
}

impl DeliveryService {
    /// Create a new delivery service.
    #[must_use] 
    pub const fn new(user_repo: UserRepository, following_repo: FollowingRepository, base_url: Url) -> Self {
        Self {
            user_repo,
            following_repo,
            id_gen: IdGenerator::new(),
            base_url,
        }
    }

    /// Build a Create activity for a note.
    #[must_use] 
    pub fn build_create_activity(&self, note: &note::Model, author: &user::Model) -> Value {
        let actor_url = format!("{}/users/{}", self.base_url, author.id);
        let note_url = format!("{}/notes/{}", self.base_url, note.id);
        let activity_id = format!("{note_url}/activity");

        // Build addressing
        let (to, cc) = self.build_addressing(note);

        // Convert to ActivityPub Note
        let ap_note = json!({
            "@context": [
                "https://www.w3.org/ns/activitystreams",
                {
                    "misskey": "https://misskey-hub.net/ns#",
                    "_misskey_content": "misskey:_misskey_content",
                    "_misskey_quote": "misskey:_misskey_quote"
                }
            ],
            "id": note_url,
            "type": "Note",
            "attributedTo": actor_url,
            "content": note.text.as_deref().unwrap_or(""),
            "published": note.created_at.to_rfc3339(),
            "to": to,
            "cc": cc,
        });

        json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": activity_id,
            "type": "Create",
            "actor": actor_url,
            "object": ap_note,
            "to": to,
            "cc": cc,
        })
    }

    /// Build a Delete activity for a note.
    #[must_use] 
    pub fn build_delete_activity(&self, note_id: &str, author: &user::Model) -> Value {
        let actor_url = format!("{}/users/{}", self.base_url, author.id);
        let note_url = format!("{}/notes/{}", self.base_url, note_id);
        let activity_id = format!("{}/delete/{}", actor_url, self.id_gen.generate());

        json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": activity_id,
            "type": "Delete",
            "actor": actor_url,
            "object": {
                "id": note_url,
                "type": "Tombstone"
            }
        })
    }

    /// Build a Follow activity.
    #[must_use] 
    pub fn build_follow_activity(&self, follower: &user::Model, followee: &user::Model) -> Value {
        let actor_url = format!("{}/users/{}", self.base_url, follower.id);
        let target_url = followee
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/users/{}", self.base_url, followee.id));
        let activity_id = format!("{}/follow/{}", actor_url, self.id_gen.generate());

        json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": activity_id,
            "type": "Follow",
            "actor": actor_url,
            "object": target_url
        })
    }

    /// Build an Undo Follow activity.
    #[must_use] 
    pub fn build_unfollow_activity(
        &self,
        follower: &user::Model,
        followee: &user::Model,
        original_follow_id: &str,
    ) -> Value {
        let actor_url = format!("{}/users/{}", self.base_url, follower.id);
        let target_url = followee
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/users/{}", self.base_url, followee.id));
        let activity_id = format!("{}/undo/{}", actor_url, self.id_gen.generate());

        json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": activity_id,
            "type": "Undo",
            "actor": actor_url,
            "object": {
                "id": original_follow_id,
                "type": "Follow",
                "actor": actor_url,
                "object": target_url
            }
        })
    }

    /// Build a Like activity (reaction).
    #[must_use] 
    pub fn build_like_activity(
        &self,
        user: &user::Model,
        note: &note::Model,
        reaction: &str,
    ) -> Value {
        let actor_url = format!("{}/users/{}", self.base_url, user.id);
        let note_url = note
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/notes/{}", self.base_url, note.id));
        let activity_id = format!("{}/like/{}", actor_url, self.id_gen.generate());

        // Misskey-style reaction in _misskey_reaction
        json!({
            "@context": [
                "https://www.w3.org/ns/activitystreams",
                {
                    "misskey": "https://misskey-hub.net/ns#",
                    "_misskey_reaction": "misskey:_misskey_reaction"
                }
            ],
            "id": activity_id,
            "type": "Like",
            "actor": actor_url,
            "object": note_url,
            "_misskey_reaction": reaction
        })
    }

    /// Build an Announce activity (renote/boost).
    #[must_use] 
    pub fn build_announce_activity(&self, user: &user::Model, note: &note::Model) -> Value {
        let actor_url = format!("{}/users/{}", self.base_url, user.id);
        let note_url = note
            .uri
            .clone()
            .unwrap_or_else(|| format!("{}/notes/{}", self.base_url, note.id));
        let activity_id = format!("{}/announce/{}", actor_url, self.id_gen.generate());

        json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": activity_id,
            "type": "Announce",
            "actor": actor_url,
            "object": note_url,
            "to": ["https://www.w3.org/ns/activitystreams#Public"],
            "cc": [format!("{}/followers", actor_url)]
        })
    }

    /// Get inboxes to deliver to for a note.
    pub async fn get_delivery_inboxes(
        &self,
        author: &user::Model,
        _note: &note::Model,
    ) -> AppResult<Vec<String>> {
        let mut inboxes = Vec::new();

        // Get followers of the author
        let followers = self
            .following_repo
            .find_followers(&author.id, 10000, None)
            .await?;

        for following in followers {
            // Get follower user
            if let Ok(follower) = self.user_repo.get_by_id(&following.follower_id).await {
                // Only deliver to remote users
                if follower.host.is_some() {
                    // Prefer shared inbox if available
                    if let Some(ref shared_inbox) = follower.shared_inbox {
                        if !inboxes.contains(shared_inbox) {
                            inboxes.push(shared_inbox.clone());
                        }
                    } else if let Some(ref inbox) = follower.inbox
                        && !inboxes.contains(inbox) {
                            inboxes.push(inbox.clone());
                        }
                }
            }
        }

        info!(
            author = %author.id,
            inbox_count = inboxes.len(),
            "Collected delivery inboxes"
        );

        Ok(inboxes)
    }

    /// Build addressing (to/cc) for a note based on visibility.
    fn build_addressing(&self, note: &note::Model) -> (Vec<String>, Vec<String>) {
        let public = "https://www.w3.org/ns/activitystreams#Public".to_string();
        let followers = format!("{}/users/{}/followers", self.base_url, note.user_id);

        match note.visibility {
            note::Visibility::Public => {
                (vec![public], vec![followers])
            }
            note::Visibility::Home => {
                (vec![followers], vec![public])
            }
            note::Visibility::Followers => {
                (vec![followers], vec![])
            }
            note::Visibility::Specified => {
                // TODO: Extract visible_user_ids and convert to actor URLs
                (vec![], vec![])
            }
        }
    }
}
