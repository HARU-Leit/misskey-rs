//! Search service with optional Meilisearch integration.
//!
//! This module provides a unified search interface that can use either:
//! - `PostgreSQL` full-text search (default, always available)
//! - Meilisearch (optional, for improved performance and relevance)
//!
//! When Meilisearch is configured, it's used for search queries while `PostgreSQL`
//! remains the source of truth for data storage.

#[cfg(feature = "meilisearch")]
use misskey_common::AppError;
use misskey_common::AppResult;
use misskey_db::repositories::NoteRepository;
use serde::{Deserialize, Serialize};
#[cfg(feature = "meilisearch")]
use tracing::{debug, info, warn};

#[cfg(feature = "meilisearch")]
use meilisearch_sdk::client::Client as MeilisearchClient;

/// Configuration for the search service.
#[derive(Clone, Debug)]
pub struct SearchConfig {
    /// Meilisearch host URL (e.g., "<http://localhost:7700>")
    pub meilisearch_url: Option<String>,
    /// Meilisearch API key (optional, for authenticated access)
    pub meilisearch_api_key: Option<String>,
    /// Whether to use Meilisearch when available (can be disabled for fallback)
    pub use_meilisearch: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            meilisearch_url: None,
            meilisearch_api_key: None,
            use_meilisearch: true,
        }
    }
}

impl SearchConfig {
    /// Create a new search config with Meilisearch enabled.
    #[must_use]
    pub const fn with_meilisearch(url: String, api_key: Option<String>) -> Self {
        Self {
            meilisearch_url: Some(url),
            meilisearch_api_key: api_key,
            use_meilisearch: true,
        }
    }

    /// Check if Meilisearch is configured.
    #[must_use]
    pub const fn has_meilisearch(&self) -> bool {
        self.meilisearch_url.is_some() && self.use_meilisearch
    }
}

/// Document representing a note in the search index.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDocument {
    /// Note ID (primary key)
    pub id: String,
    /// Note text content
    pub text: String,
    /// Content warning (optional)
    pub cw: Option<String>,
    /// User ID of the author
    pub user_id: String,
    /// Username of the author
    pub username: String,
    /// Host of the author (null for local users)
    pub host: Option<String>,
    /// Hashtags extracted from the note
    pub tags: Vec<String>,
    /// Visibility level
    pub visibility: String,
    /// Creation timestamp (Unix epoch seconds)
    pub created_at: i64,
    /// Reaction count for ranking
    pub reaction_count: i64,
    /// Renote count for ranking
    pub renote_count: i64,
    /// Reply count for ranking
    pub reply_count: i64,
}

/// Document representing a user in the search index.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDocument {
    /// User ID (primary key)
    pub id: String,
    /// Username (unique per host)
    pub username: String,
    /// Display name
    pub name: Option<String>,
    /// User bio/description
    pub description: Option<String>,
    /// Host (null for local users)
    pub host: Option<String>,
    /// Follower count for ranking
    pub followers_count: i32,
    /// Notes count for ranking
    pub notes_count: i32,
    /// Whether this is a bot account
    pub is_bot: bool,
    /// Whether the user is suspended
    pub is_suspended: bool,
}

/// Search result with relevance score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit<T> {
    /// The matched document
    pub document: T,
    /// Relevance score (higher is better)
    pub score: Option<f64>,
}

/// Search service providing unified search across `PostgreSQL` and Meilisearch.
#[derive(Clone)]
pub struct SearchService {
    note_repo: NoteRepository,
    config: SearchConfig,
    #[cfg(feature = "meilisearch")]
    meilisearch: Option<MeilisearchClient>,
}

impl SearchService {
    /// Index name for notes in Meilisearch.
    pub const NOTES_INDEX: &'static str = "notes";
    /// Index name for users in Meilisearch.
    pub const USERS_INDEX: &'static str = "users";

    /// Create a new search service with `PostgreSQL` only.
    #[must_use]
    pub fn new(note_repo: NoteRepository) -> Self {
        Self {
            note_repo,
            config: SearchConfig::default(),
            #[cfg(feature = "meilisearch")]
            meilisearch: None,
        }
    }

    /// Create a new search service with Meilisearch integration.
    #[cfg(feature = "meilisearch")]
    #[must_use]
    #[allow(clippy::expect_used)] // URL existence is checked by has_meilisearch() above
    pub fn with_meilisearch(note_repo: NoteRepository, config: SearchConfig) -> Self {
        let meilisearch = if config.has_meilisearch() {
            let url = config.meilisearch_url.as_ref().expect("URL checked above");
            let client = if let Some(ref api_key) = config.meilisearch_api_key {
                MeilisearchClient::new(url, Some(api_key.clone()))
            } else {
                MeilisearchClient::new(url, None::<String>)
            };
            match client {
                Ok(c) => {
                    info!(url = %url, "Meilisearch client initialized");
                    Some(c)
                }
                Err(e) => {
                    warn!(error = %e, "Failed to initialize Meilisearch client, falling back to PostgreSQL");
                    None
                }
            }
        } else {
            None
        };

        Self {
            note_repo,
            config,
            meilisearch,
        }
    }

    /// Check if Meilisearch is available and connected.
    #[must_use]
    pub const fn is_meilisearch_available(&self) -> bool {
        #[cfg(feature = "meilisearch")]
        {
            self.meilisearch.is_some()
        }
        #[cfg(not(feature = "meilisearch"))]
        {
            false
        }
    }

    /// Get the search configuration.
    #[must_use]
    pub const fn config(&self) -> &SearchConfig {
        &self.config
    }

    /// Initialize Meilisearch indexes with proper settings.
    ///
    /// This should be called once during application startup.
    #[cfg(feature = "meilisearch")]
    pub async fn initialize_indexes(&self) -> AppResult<()> {
        let Some(ref client) = self.meilisearch else {
            debug!("Meilisearch not configured, skipping index initialization");
            return Ok(());
        };

        info!("Initializing Meilisearch indexes");

        // Create notes index with settings
        let notes_index = client.index(Self::NOTES_INDEX);
        notes_index
            .set_searchable_attributes(["text", "cw", "username", "tags"])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set searchable attributes: {e}")))?;

        notes_index
            .set_filterable_attributes(["userId", "host", "visibility", "tags", "createdAt"])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set filterable attributes: {e}")))?;

        notes_index
            .set_sortable_attributes(["createdAt", "reactionCount", "renoteCount"])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set sortable attributes: {e}")))?;

        notes_index
            .set_ranking_rules([
                "words",
                "typo",
                "proximity",
                "attribute",
                "sort",
                "exactness",
                "reactionCount:desc",
            ])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set ranking rules: {e}")))?;

        // Create users index with settings
        let users_index = client.index(Self::USERS_INDEX);
        users_index
            .set_searchable_attributes(["username", "name", "description"])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set searchable attributes: {e}")))?;

        users_index
            .set_filterable_attributes(["host", "isBot", "isSuspended"])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set filterable attributes: {e}")))?;

        users_index
            .set_sortable_attributes(["followersCount", "notesCount"])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set sortable attributes: {e}")))?;

        users_index
            .set_ranking_rules([
                "words",
                "typo",
                "proximity",
                "attribute",
                "sort",
                "exactness",
                "followersCount:desc",
            ])
            .await
            .map_err(|e| AppError::Internal(format!("Failed to set ranking rules: {e}")))?;

        info!("Meilisearch indexes initialized successfully");
        Ok(())
    }

    /// Index a note document in Meilisearch.
    #[cfg(feature = "meilisearch")]
    pub async fn index_note(&self, doc: NoteDocument) -> AppResult<()> {
        let Some(ref client) = self.meilisearch else {
            return Ok(());
        };

        // Only index public notes
        if doc.visibility != "Public" {
            return Ok(());
        }

        let index = client.index(Self::NOTES_INDEX);
        index
            .add_documents(&[doc], Some("id"))
            .await
            .map_err(|e| AppError::Internal(format!("Failed to index note: {e}")))?;

        Ok(())
    }

    /// Remove a note from the search index.
    #[cfg(feature = "meilisearch")]
    pub async fn remove_note(&self, note_id: &str) -> AppResult<()> {
        let Some(ref client) = self.meilisearch else {
            return Ok(());
        };

        let index = client.index(Self::NOTES_INDEX);
        index
            .delete_document(note_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to remove note from index: {e}")))?;

        Ok(())
    }

    /// Index a user document in Meilisearch.
    #[cfg(feature = "meilisearch")]
    pub async fn index_user(&self, doc: UserDocument) -> AppResult<()> {
        let Some(ref client) = self.meilisearch else {
            return Ok(());
        };

        // Don't index suspended users
        if doc.is_suspended {
            return Ok(());
        }

        let index = client.index(Self::USERS_INDEX);
        index
            .add_documents(&[doc], Some("id"))
            .await
            .map_err(|e| AppError::Internal(format!("Failed to index user: {e}")))?;

        Ok(())
    }

    /// Remove a user from the search index.
    #[cfg(feature = "meilisearch")]
    pub async fn remove_user(&self, user_id: &str) -> AppResult<()> {
        let Some(ref client) = self.meilisearch else {
            return Ok(());
        };

        let index = client.index(Self::USERS_INDEX);
        index
            .delete_document(user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to remove user from index: {e}")))?;

        Ok(())
    }

    /// Search notes using Meilisearch (with `PostgreSQL` fallback).
    #[cfg(feature = "meilisearch")]
    pub async fn search_notes_meilisearch(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
        user_id: Option<&str>,
        host: Option<&str>,
    ) -> AppResult<Vec<SearchHit<NoteDocument>>> {
        let Some(ref client) = self.meilisearch else {
            // Fall back to PostgreSQL
            return self.search_notes(query, limit, offset, user_id, host).await;
        };

        let index = client.index(Self::NOTES_INDEX);

        // Build filter
        let mut filters = vec!["visibility = 'Public'".to_string()];
        if let Some(uid) = user_id {
            filters.push(format!("userId = '{uid}'"));
        }
        if let Some(h) = host {
            if h.is_empty() {
                filters.push("host IS NULL".to_string());
            } else {
                filters.push(format!("host = '{h}'"));
            }
        }
        let filter = filters.join(" AND ");

        let results = index
            .search()
            .with_query(query)
            .with_limit(limit)
            .with_offset(offset)
            .with_filter(&filter)
            .with_show_ranking_score(true)
            .execute::<NoteDocument>()
            .await
            .map_err(|e| {
                warn!(error = %e, "Meilisearch search failed, falling back to PostgreSQL");
                AppError::Internal(format!("Meilisearch search failed: {e}"))
            })?;

        Ok(results
            .hits
            .into_iter()
            .map(|hit| SearchHit {
                document: hit.result,
                score: hit.ranking_score,
            })
            .collect())
    }

    /// Search notes using `PostgreSQL` full-text search.
    ///
    /// This is the default search method when Meilisearch is not available.
    pub async fn search_notes(
        &self,
        query: &str,
        limit: usize,
        _offset: usize,
        user_id: Option<&str>,
        host: Option<&str>,
    ) -> AppResult<Vec<SearchHit<NoteDocument>>> {
        let notes = self
            .note_repo
            .search_fulltext(query, limit as u64, None, user_id, host)
            .await?;

        Ok(notes
            .into_iter()
            .map(|note| {
                let tags: Vec<String> = note
                    .tags
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                SearchHit {
                    document: NoteDocument {
                        id: note.id,
                        text: note.text.unwrap_or_default(),
                        cw: note.cw,
                        user_id: note.user_id,
                        username: String::new(), // Not available from note model directly
                        host: None,              // Would need to join with user table
                        tags,
                        visibility: format!("{:?}", note.visibility),
                        created_at: note.created_at.timestamp(),
                        reaction_count: i64::from(note.reaction_count),
                        renote_count: i64::from(note.renote_count),
                        reply_count: i64::from(note.replies_count),
                    },
                    score: None, // PostgreSQL doesn't return scores in this format
                }
            })
            .collect())
    }

    /// Get Meilisearch health status.
    #[cfg(feature = "meilisearch")]
    pub async fn health_check(&self) -> AppResult<bool> {
        let Some(ref client) = self.meilisearch else {
            return Ok(false);
        };

        match client.health().await {
            Ok(health) => Ok(health.status == "available"),
            Err(e) => {
                warn!(error = %e, "Meilisearch health check failed");
                Ok(false)
            }
        }
    }

    /// Get index statistics.
    #[cfg(feature = "meilisearch")]
    pub async fn get_stats(&self) -> AppResult<SearchStats> {
        let Some(ref client) = self.meilisearch else {
            return Ok(SearchStats::default());
        };

        let notes_stats = client
            .index(Self::NOTES_INDEX)
            .get_stats()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get notes index stats: {e}")))?;

        let users_stats = client
            .index(Self::USERS_INDEX)
            .get_stats()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get users index stats: {e}")))?;

        Ok(SearchStats {
            notes_count: notes_stats.number_of_documents as u64,
            users_count: users_stats.number_of_documents as u64,
            is_indexing: notes_stats.is_indexing || users_stats.is_indexing,
        })
    }
}

/// Search index statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchStats {
    /// Number of notes in the index
    pub notes_count: u64,
    /// Number of users in the index
    pub users_count: u64,
    /// Whether indexing is in progress
    pub is_indexing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert!(!config.has_meilisearch());
    }

    #[test]
    fn test_search_config_with_meilisearch() {
        let config = SearchConfig::with_meilisearch(
            "http://localhost:7700".to_string(),
            Some("master_key".to_string()),
        );
        assert!(config.has_meilisearch());
    }

    #[test]
    fn test_note_document_serialization() {
        let doc = NoteDocument {
            id: "note1".to_string(),
            text: "Hello world".to_string(),
            cw: None,
            user_id: "user1".to_string(),
            username: "alice".to_string(),
            host: None,
            tags: vec!["rust".to_string()],
            visibility: "Public".to_string(),
            created_at: 1234567890,
            reaction_count: 5,
            renote_count: 2,
            reply_count: 1,
        };

        let json = serde_json::to_string(&doc).expect("serialization should succeed");
        assert!(json.contains("Hello world"));
    }
}
