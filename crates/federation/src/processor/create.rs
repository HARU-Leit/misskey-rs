//! Create activity processor.

use misskey_common::{AppResult, IdGenerator};
use misskey_db::{
    entities::{note, user},
    repositories::{NoteRepository, UserRepository},
};
use sea_orm::Set;
use serde_json::json;
use tracing::info;

use super::ActorFetcher;
use crate::{CreateActivity, client::ApClient, objects::ApNote};

/// Processor for Create activities (notes).
#[derive(Clone)]
pub struct CreateProcessor {
    note_repo: NoteRepository,
    actor_fetcher: ActorFetcher,
    id_gen: IdGenerator,
}

impl CreateProcessor {
    /// Create a new create processor.
    #[must_use]
    pub const fn new(
        note_repo: NoteRepository,
        user_repo: UserRepository,
        ap_client: ApClient,
    ) -> Self {
        Self {
            note_repo,
            actor_fetcher: ActorFetcher::new(user_repo, ap_client),
            id_gen: IdGenerator::new(),
        }
    }

    /// Process an incoming Create activity (Note).
    pub async fn process(&self, activity: &CreateActivity) -> AppResult<note::Model> {
        info!(
            actor = %activity.actor,
            note_id = %activity.object.id,
            "Processing Create activity"
        );

        // Check if we already have this note
        if let Some(existing) = self
            .note_repo
            .find_by_uri(activity.object.id.as_str())
            .await?
        {
            info!(note_id = %existing.id, "Note already exists");
            return Ok(existing);
        }

        // Find or fetch the author
        let author = self.find_or_fetch_author(&activity.actor).await?;

        // Convert ActivityPub Note to local note
        let note = self.create_note_from_ap(&activity.object, &author).await?;

        info!(
            note_id = %note.id,
            author = %author.id,
            "Created note from remote"
        );

        Ok(note)
    }

    /// Find an existing author or fetch from remote.
    async fn find_or_fetch_author(&self, actor_url: &url::Url) -> AppResult<user::Model> {
        self.actor_fetcher.find_or_fetch(actor_url).await
    }

    /// Create a note from an `ActivityPub` Note object.
    async fn create_note_from_ap(
        &self,
        ap_note: &ApNote,
        author: &user::Model,
    ) -> AppResult<note::Model> {
        // Parse reply target
        let reply_id = if let Some(ref reply_url) = ap_note.in_reply_to {
            // Try to find the reply target in our database
            if let Some(reply_note) = self.note_repo.find_by_uri(reply_url.as_str()).await? {
                Some(reply_note.id)
            } else {
                // Reply to unknown note - we might want to fetch it
                // For now, we'll just skip the reply chain
                None
            }
        } else {
            None
        };

        // Determine visibility
        let visibility = self.determine_visibility(ap_note);

        // Extract mentions and tags from content
        let content = &ap_note.content;
        let mentions = self.extract_mentions_from_tags(ap_note);
        let tags = self.extract_hashtags_from_tags(ap_note);

        let note_id = self.id_gen.generate();

        let model = note::ActiveModel {
            id: Set(note_id),
            user_id: Set(author.id.clone()),
            user_host: Set(author.host.clone()),
            text: Set(Some(strip_html_basic(content))),
            cw: Set(ap_note.summary.clone()),
            visibility: Set(visibility),
            reply_id: Set(reply_id.clone()),
            renote_id: Set(None),
            thread_id: Set(reply_id),
            mentions: Set(json!(mentions)),
            visible_user_ids: Set(json!([])),
            file_ids: Set(json!([])), // TODO: Handle attachments
            tags: Set(json!(tags)),
            reactions: Set(json!({})),
            is_local: Set(false),
            uri: Set(Some(ap_note.id.to_string())),
            url: Set(None), // Note URL not available in current schema
            ..Default::default()
        };

        self.note_repo.create(model).await
    }

    /// Determine visibility from `ActivityPub` addressing.
    fn determine_visibility(&self, ap_note: &ApNote) -> note::Visibility {
        let public = "https://www.w3.org/ns/activitystreams#Public";

        let to_urls = ap_note.to.as_deref().unwrap_or(&[]);
        let cc_urls = ap_note.cc.as_deref().unwrap_or(&[]);

        // Check if addressed to public
        let is_public = to_urls.iter().any(|u| u.as_str() == public)
            || cc_urls.iter().any(|u| u.as_str() == public);

        if is_public {
            // Check if public is in 'to' (public) or 'cc' (unlisted/home)
            if to_urls.iter().any(|u| u.as_str() == public) {
                note::Visibility::Public
            } else {
                note::Visibility::Home
            }
        } else if to_urls.len() == 1 && cc_urls.is_empty() {
            // Direct message (single recipient, no CC)
            note::Visibility::Specified
        } else {
            // Followers only
            note::Visibility::Followers
        }
    }

    /// Extract mention usernames from `ActivityPub` tags.
    fn extract_mentions_from_tags(&self, ap_note: &ApNote) -> Vec<String> {
        ap_note
            .tag
            .as_ref()
            .map(|tags| {
                tags.iter()
                    .filter(|tag| tag.kind == "Mention")
                    .filter_map(|tag| tag.name.clone())
                    .map(|name| name.trim_start_matches('@').to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract hashtags from `ActivityPub` tags.
    fn extract_hashtags_from_tags(&self, ap_note: &ApNote) -> Vec<String> {
        ap_note
            .tag
            .as_ref()
            .map(|tags| {
                tags.iter()
                    .filter(|tag| tag.kind == "Hashtag")
                    .filter_map(|tag| tag.name.clone())
                    .map(|name| name.trim_start_matches('#').to_lowercase())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Basic HTML stripping (for converting `ActivityPub` content to plain text).
/// A more robust solution would use an HTML parser.
fn strip_html_basic(html: &str) -> String {
    // First, replace <br> tags with newlines before stripping
    let html = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p><p>", "\n\n")
        .replace("</p>", "\n")
        .replace("<p>", "");

    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    // Decode common HTML entities
    result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic() {
        assert_eq!(strip_html_basic("<p>Hello</p>"), "Hello");
        assert_eq!(strip_html_basic("<a href='x'>Link</a>"), "Link");
        assert_eq!(strip_html_basic("a &amp; b"), "a & b");
        assert_eq!(strip_html_basic("line1<br>line2"), "line1\nline2");
    }
}
