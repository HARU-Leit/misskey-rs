//! Word filter service.

use chrono::{Duration, Utc};
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::word_filter::{self, FilterAction, FilterContext};
use misskey_db::repositories::WordFilterRepository;
use regex::Regex;
use sea_orm::Set;
use serde::Deserialize;
use validator::Validate;

/// Maximum number of filters per user.
const MAX_FILTERS_PER_USER: u64 = 100;

/// Maximum phrase length.
const MAX_PHRASE_LENGTH: usize = 512;

/// Input for creating a word filter.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateFilterInput {
    #[validate(length(min = 1, max = 512))]
    pub phrase: String,
    #[serde(default)]
    pub is_regex: bool,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default = "default_whole_word")]
    pub whole_word: bool,
    #[serde(default = "default_action")]
    pub action: FilterAction,
    #[serde(default = "default_context")]
    pub context: FilterContext,
    /// Duration in seconds until expiration (None = permanent).
    pub expires_in: Option<i64>,
}

fn default_whole_word() -> bool {
    true
}

fn default_action() -> FilterAction {
    FilterAction::Hide
}

fn default_context() -> FilterContext {
    FilterContext::All
}

/// Input for updating a word filter.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFilterInput {
    pub filter_id: String,
    #[validate(length(min = 1, max = 512))]
    pub phrase: Option<String>,
    pub is_regex: Option<bool>,
    pub case_sensitive: Option<bool>,
    pub whole_word: Option<bool>,
    pub action: Option<FilterAction>,
    pub context: Option<FilterContext>,
    /// Duration in seconds until expiration (None = permanent, Some(0) = remove expiration).
    pub expires_in: Option<Option<i64>>,
}

/// Result of applying filters to content.
#[derive(Debug, Clone)]
pub struct FilterResult {
    /// Whether any filter matched.
    pub matched: bool,
    /// The matching filter IDs.
    pub matched_filter_ids: Vec<String>,
    /// The most severe action to take.
    pub action: Option<FilterAction>,
    /// Matched phrases for display.
    pub matched_phrases: Vec<String>,
}

/// Service for managing word filters.
#[derive(Clone)]
pub struct WordFilterService {
    filter_repo: WordFilterRepository,
    id_gen: IdGenerator,
}

impl WordFilterService {
    /// Create a new word filter service.
    #[must_use]
    pub const fn new(filter_repo: WordFilterRepository) -> Self {
        Self {
            filter_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Get a filter by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<word_filter::Model>> {
        self.filter_repo.find_by_id(id).await
    }

    /// Get a filter by ID with ownership check.
    pub async fn get_by_id_for_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<word_filter::Model> {
        let filter = self.filter_repo.get_by_id(id).await?;

        if filter.user_id != user_id {
            return Err(AppError::Forbidden("Not the filter owner".to_string()));
        }

        Ok(filter)
    }

    /// List filters for a user.
    pub async fn list_filters(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<word_filter::Model>> {
        self.filter_repo.find_by_user(user_id, limit, offset).await
    }

    /// List active (non-expired) filters for a user.
    pub async fn list_active_filters(&self, user_id: &str) -> AppResult<Vec<word_filter::Model>> {
        self.filter_repo.find_active_by_user(user_id).await
    }

    /// Count filters for a user.
    pub async fn count_filters(&self, user_id: &str) -> AppResult<u64> {
        self.filter_repo.count_by_user(user_id).await
    }

    /// Create a new filter.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateFilterInput,
    ) -> AppResult<word_filter::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check filter limit
        let count = self.filter_repo.count_by_user(user_id).await?;
        if count >= MAX_FILTERS_PER_USER {
            return Err(AppError::Validation(format!(
                "Maximum of {} filters allowed per user",
                MAX_FILTERS_PER_USER
            )));
        }

        // Validate phrase length
        if input.phrase.len() > MAX_PHRASE_LENGTH {
            return Err(AppError::Validation(format!(
                "Phrase must be at most {} characters",
                MAX_PHRASE_LENGTH
            )));
        }

        // Validate regex if is_regex is true
        if input.is_regex {
            Regex::new(&input.phrase)
                .map_err(|e| AppError::Validation(format!("Invalid regex pattern: {}", e)))?;
        }

        // Calculate expiration
        let expires_at = input.expires_in.map(|secs| {
            let duration = Duration::seconds(secs);
            Utc::now() + duration
        });

        let id = self.id_gen.generate();
        let now = Utc::now();

        let model = word_filter::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            phrase: Set(input.phrase),
            is_regex: Set(input.is_regex),
            case_sensitive: Set(input.case_sensitive),
            whole_word: Set(input.whole_word),
            action: Set(input.action),
            context: Set(input.context),
            expires_at: Set(expires_at.map(|dt| dt.into())),
            match_count: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.filter_repo.create(model).await
    }

    /// Update a filter.
    pub async fn update(
        &self,
        user_id: &str,
        input: UpdateFilterInput,
    ) -> AppResult<word_filter::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Get filter and verify ownership
        let filter = self.get_by_id_for_user(&input.filter_id, user_id).await?;

        // Validate phrase if provided
        if let Some(ref phrase) = input.phrase {
            if phrase.len() > MAX_PHRASE_LENGTH {
                return Err(AppError::Validation(format!(
                    "Phrase must be at most {} characters",
                    MAX_PHRASE_LENGTH
                )));
            }

            // Check if regex is valid
            let is_regex = input.is_regex.unwrap_or(filter.is_regex);
            if is_regex {
                Regex::new(phrase)
                    .map_err(|e| AppError::Validation(format!("Invalid regex pattern: {}", e)))?;
            }
        } else if let Some(true) = input.is_regex {
            // Validating existing phrase as regex
            Regex::new(&filter.phrase).map_err(|e| {
                AppError::Validation(format!("Existing phrase is not valid regex: {}", e))
            })?;
        }

        let now = Utc::now();

        let mut active: word_filter::ActiveModel = filter.into();

        if let Some(phrase) = input.phrase {
            active.phrase = Set(phrase);
        }
        if let Some(is_regex) = input.is_regex {
            active.is_regex = Set(is_regex);
        }
        if let Some(case_sensitive) = input.case_sensitive {
            active.case_sensitive = Set(case_sensitive);
        }
        if let Some(whole_word) = input.whole_word {
            active.whole_word = Set(whole_word);
        }
        if let Some(action) = input.action {
            active.action = Set(action);
        }
        if let Some(context) = input.context {
            active.context = Set(context);
        }
        if let Some(expires_in) = input.expires_in {
            let expires_at = expires_in.map(|secs| {
                let duration = Duration::seconds(secs);
                Utc::now() + duration
            });
            active.expires_at = Set(expires_at.map(|dt| dt.into()));
        }

        active.updated_at = Set(Some(now.into()));

        self.filter_repo.update(active).await
    }

    /// Delete a filter.
    pub async fn delete(&self, filter_id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        self.get_by_id_for_user(filter_id, user_id).await?;
        self.filter_repo.delete(filter_id).await
    }

    /// Delete all filters for a user.
    pub async fn delete_all_for_user(&self, user_id: &str) -> AppResult<u64> {
        let filters = self.filter_repo.find_by_user(user_id, 1000, 0).await?;
        let count = filters.len() as u64;

        for filter in filters {
            self.filter_repo.delete(&filter.id).await?;
        }

        Ok(count)
    }

    /// Apply filters to content and return the result.
    pub async fn apply_filters(
        &self,
        user_id: &str,
        content: &str,
        context: FilterContext,
    ) -> AppResult<FilterResult> {
        let filters = self.filter_repo.find_active_by_user(user_id).await?;

        let mut matched_filter_ids = Vec::new();
        let mut matched_phrases = Vec::new();
        let mut most_severe_action: Option<FilterAction> = None;

        for filter in filters {
            // Check if filter applies to this context
            if filter.context != FilterContext::All && filter.context != context {
                continue;
            }

            let matches = self.check_filter_match(&filter, content)?;

            if matches {
                matched_filter_ids.push(filter.id.clone());
                matched_phrases.push(filter.phrase.clone());

                // Update action to most severe
                most_severe_action = Some(match (&most_severe_action, &filter.action) {
                    (None, action) => action.clone(),
                    (Some(FilterAction::Hide), _) => FilterAction::Hide,
                    (_, FilterAction::Hide) => FilterAction::Hide,
                    (Some(FilterAction::ContentWarning), _) => FilterAction::ContentWarning,
                    (_, FilterAction::ContentWarning) => FilterAction::ContentWarning,
                    (Some(FilterAction::Warn), _) => FilterAction::Warn,
                });

                // Increment match count in background (non-blocking)
                let _ = self.filter_repo.increment_match_count(&filter.id).await;
            }
        }

        Ok(FilterResult {
            matched: !matched_filter_ids.is_empty(),
            matched_filter_ids,
            action: most_severe_action,
            matched_phrases,
        })
    }

    /// Check if a single filter matches the content.
    fn check_filter_match(&self, filter: &word_filter::Model, content: &str) -> AppResult<bool> {
        let content_to_check = if filter.case_sensitive {
            content.to_string()
        } else {
            content.to_lowercase()
        };

        let phrase_to_check = if filter.case_sensitive {
            filter.phrase.clone()
        } else {
            filter.phrase.to_lowercase()
        };

        if filter.is_regex {
            let pattern = if filter.case_sensitive {
                filter.phrase.clone()
            } else {
                format!("(?i){}", filter.phrase)
            };

            let regex = Regex::new(&pattern)
                .map_err(|e| AppError::Internal(format!("Invalid regex in filter: {}", e)))?;

            Ok(regex.is_match(content))
        } else if filter.whole_word {
            // Word boundary matching
            let pattern = format!(r"\b{}\b", regex::escape(&phrase_to_check));
            let regex = Regex::new(&pattern).map_err(|e| {
                AppError::Internal(format!("Failed to create word boundary regex: {}", e))
            })?;

            Ok(regex.is_match(&content_to_check))
        } else {
            // Simple substring match
            Ok(content_to_check.contains(&phrase_to_check))
        }
    }

    /// Delete expired filters (maintenance task).
    pub async fn cleanup_expired(&self) -> AppResult<u64> {
        self.filter_repo.delete_expired().await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

    fn create_test_filter(
        id: &str,
        user_id: &str,
        phrase: &str,
        is_regex: bool,
        whole_word: bool,
    ) -> word_filter::Model {
        word_filter::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            phrase: phrase.to_string(),
            is_regex,
            case_sensitive: false,
            whole_word,
            action: FilterAction::Hide,
            context: FilterContext::All,
            expires_at: None,
            match_count: 0,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let filter = create_test_filter("filter1", "user1", "test", false, true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[filter.clone()]])
                .into_connection(),
        );

        let repo = WordFilterRepository::new(db);
        let service = WordFilterService::new(repo);

        let result = service.get_by_id("filter1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().phrase, "test");
    }

    #[tokio::test]
    async fn test_check_filter_match_simple() {
        let filter = create_test_filter("filter1", "user1", "bad word", false, false);

        let service = WordFilterService::new(WordFilterRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // Should match substring
        assert!(
            service
                .check_filter_match(&filter, "This contains bad word in it")
                .unwrap()
        );
        assert!(
            !service
                .check_filter_match(&filter, "This is clean")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_check_filter_match_whole_word() {
        let filter = create_test_filter("filter1", "user1", "bad", false, true);

        let service = WordFilterService::new(WordFilterRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // Should only match whole word
        assert!(
            service
                .check_filter_match(&filter, "This is bad content")
                .unwrap()
        );
        assert!(
            !service
                .check_filter_match(&filter, "This is badger content")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_check_filter_match_regex() {
        let filter = create_test_filter("filter1", "user1", r"b[a4]d\s*w[o0]rd", true, false);

        let service = WordFilterService::new(WordFilterRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // Should match regex variations
        assert!(
            service
                .check_filter_match(&filter, "This is bad word")
                .unwrap()
        );
        assert!(
            service
                .check_filter_match(&filter, "This is b4d w0rd")
                .unwrap()
        );
        assert!(
            service
                .check_filter_match(&filter, "This is badword")
                .unwrap()
        );
        assert!(
            !service
                .check_filter_match(&filter, "This is clean")
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_check_filter_match_case_insensitive() {
        let filter = create_test_filter("filter1", "user1", "BadWord", false, false);

        let service = WordFilterService::new(WordFilterRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // Case insensitive by default
        assert!(
            service
                .check_filter_match(&filter, "This is BADWORD")
                .unwrap()
        );
        assert!(
            service
                .check_filter_match(&filter, "This is badword")
                .unwrap()
        );
    }
}
