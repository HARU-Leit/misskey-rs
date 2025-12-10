//! Note <-> `ApNote` conversion.

use chrono::Utc;
use misskey_db::entities::{drive_file, note};
use url::Url;

use crate::objects::{ApAttachment, ApNote, ApObjectType, ApTag};

use super::user::UrlConfig;

/// Extension trait for converting Note to `ApNote`.
pub trait NoteToApNote {
    /// Convert to `ApNote`.
    fn to_ap_note(
        &self,
        config: &UrlConfig,
        author_username: &str,
        files: &[drive_file::Model],
    ) -> ApNote;
}

impl NoteToApNote for note::Model {
    fn to_ap_note(
        &self,
        config: &UrlConfig,
        author_username: &str,
        files: &[drive_file::Model],
    ) -> ApNote {
        let id = if let Some(ref uri) = self.uri {
            Url::parse(uri).unwrap_or_else(|_| note_url(config, &self.id))
        } else {
            note_url(config, &self.id)
        };

        let attributed_to = config.user_url(author_username);
        let content = self.text.clone().unwrap_or_default();
        let published = self.created_at.with_timezone(&Utc);

        // Build tags from mentions
        let mentions: Vec<String> = serde_json::from_value(self.mentions.clone()).unwrap_or_default();
        let tags: Vec<ApTag> = mentions
            .iter()
            .filter_map(|user_id| {
                // In a real implementation, we'd look up the user's URI
                // For now, just create a placeholder
                Some(ApTag {
                    kind: "Mention".to_string(),
                    href: None,
                    name: Some(format!("@{user_id}")),
                })
            })
            .collect();

        // Build hashtags
        let hashtags: Vec<String> = serde_json::from_value(self.tags.clone()).unwrap_or_default();
        let hashtag_tags: Vec<ApTag> = hashtags
            .iter()
            .map(|tag| ApTag {
                kind: "Hashtag".to_string(),
                href: None,
                name: Some(format!("#{tag}")),
            })
            .collect();

        let all_tags: Vec<ApTag> = tags.into_iter().chain(hashtag_tags).collect();

        // Build attachments from files
        let attachments: Vec<ApAttachment> = files
            .iter()
            .map(|f| ApAttachment {
                kind: "Document".to_string(),
                url: Url::parse(&f.url).expect("valid file URL"),
                media_type: Some(f.content_type.clone()),
                name: f.comment.clone(),
                width: f.width.map(|w| w as u32),
                height: f.height.map(|h| h as u32),
                blurhash: f.blurhash.clone(),
            })
            .collect();

        // Determine addressing based on visibility
        let (to, cc) = visibility_to_addressing(&self.visibility, config, author_username);

        // Reply handling
        let in_reply_to = self.reply_id.as_ref().map(|reply_id| note_url(config, reply_id));

        // Quote (renote) handling
        let misskey_quote = self.renote_id.as_ref().and_then(|renote_id| {
            // Only set if this is a quote (has text) not a pure renote
            if self.text.is_some() {
                Some(note_url(config, renote_id))
            } else {
                None
            }
        });

        ApNote {
            kind: ApObjectType::Note,
            id,
            attributed_to,
            content,
            published,
            to,
            cc,
            in_reply_to,
            summary: self.cw.clone(),
            sensitive: self.cw.as_ref().map(|_| true),
            tag: if all_tags.is_empty() {
                None
            } else {
                Some(all_tags)
            },
            attachment: if attachments.is_empty() {
                None
            } else {
                Some(attachments)
            },
            one_of: None,
            any_of: None,
            end_time: None,
            closed: None,
            voters_count: None,
            // FEP-c16b: Set both quoteUrl and _misskey_quote for compatibility
            quote_url: misskey_quote.clone(),
            quote_uri: None,
            misskey_quote,
            misskey_content: self.text.clone(),
            misskey_summary: self.cw.clone(),
            misskey_reaction: None,
        }
    }
}

/// Generate note URL.
fn note_url(config: &UrlConfig, note_id: &str) -> Url {
    config
        .base_url
        .join(&format!("/notes/{note_id}"))
        .expect("valid URL")
}

/// Convert visibility to AP addressing.
fn visibility_to_addressing(
    visibility: &note::Visibility,
    config: &UrlConfig,
    author_username: &str,
) -> (Option<Vec<Url>>, Option<Vec<Url>>) {
    let public = Url::parse("https://www.w3.org/ns/activitystreams#Public").unwrap();
    let followers = config.followers_url(author_username);

    match visibility {
        note::Visibility::Public => (Some(vec![public]), Some(vec![followers])),
        note::Visibility::Home => (Some(vec![followers]), Some(vec![public])),
        note::Visibility::Followers => (Some(vec![followers]), None),
        note::Visibility::Specified => {
            // For specified visibility, we'd need to include the specific user URIs
            // This would require looking up the visible_user_ids
            (None, None)
        }
    }
}

/// Extension trait for `ApNote`.
pub trait ApNoteExt {
    /// Check if this note is public.
    fn is_public(&self) -> bool;

    /// Extract the host from the note ID.
    fn extract_host(&self) -> Option<String>;
}

impl ApNoteExt for ApNote {
    fn is_public(&self) -> bool {
        let public = "https://www.w3.org/ns/activitystreams#Public";
        self.to
            .as_ref()
            .is_some_and(|to| to.iter().any(|u| u.as_str() == public))
            || self
                .cc
                .as_ref()
                .is_some_and(|cc| cc.iter().any(|u| u.as_str() == public))
    }

    fn extract_host(&self) -> Option<String> {
        self.id.host_str().map(std::string::ToString::to_string)
    }
}
