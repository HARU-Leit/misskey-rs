//! Antenna service.

use aho_corasick::AhoCorasick;
use chrono::Utc;
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::antenna::{self, AntennaSource};
use misskey_db::entities::antenna_note;
use misskey_db::repositories::AntennaRepository;
use sea_orm::Set;
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

/// Maximum number of antennas per user.
const MAX_ANTENNAS_PER_USER: u64 = 5;

/// Maximum number of keywords.
const MAX_KEYWORDS: usize = 32;

/// Maximum keyword length.
const MAX_KEYWORD_LENGTH: usize = 128;

/// Input for creating an antenna.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateAntennaInput {
    #[validate(length(min = 1, max = 128))]
    pub name: String,
    #[serde(default = "default_source")]
    pub src: AntennaSource,
    pub user_list_id: Option<String>,
    /// Keywords in AND/OR format: [["foo", "bar"], ["baz"]] = (foo AND bar) OR baz
    #[serde(default)]
    pub keywords: Vec<Vec<String>>,
    #[serde(default)]
    pub exclude_keywords: Vec<Vec<String>>,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub instances: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub with_replies: bool,
    #[serde(default)]
    pub with_file: bool,
    #[serde(default)]
    pub notify: bool,
    #[serde(default)]
    pub local_only: bool,
}

const fn default_source() -> AntennaSource {
    AntennaSource::All
}

/// Input for updating an antenna.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAntennaInput {
    pub antenna_id: String,
    #[validate(length(min = 1, max = 128))]
    pub name: Option<String>,
    pub src: Option<AntennaSource>,
    pub user_list_id: Option<Option<String>>,
    pub keywords: Option<Vec<Vec<String>>>,
    pub exclude_keywords: Option<Vec<Vec<String>>>,
    pub users: Option<Vec<String>>,
    pub instances: Option<Vec<String>>,
    pub case_sensitive: Option<bool>,
    pub with_replies: Option<bool>,
    pub with_file: Option<bool>,
    pub notify: Option<bool>,
    pub local_only: Option<bool>,
    pub is_active: Option<bool>,
}

/// Note matching context.
pub struct NoteMatchContext {
    pub text: String,
    pub user_id: String,
    pub user_host: Option<String>,
    pub is_reply: bool,
    pub has_files: bool,
    /// User list memberships (`user_id` -> `list_ids`)
    pub list_memberships: Vec<String>,
}

/// Service for managing antennas.
#[derive(Clone)]
pub struct AntennaService {
    antenna_repo: AntennaRepository,
    id_gen: IdGenerator,
}

impl AntennaService {
    /// Create a new antenna service.
    #[must_use]
    pub const fn new(antenna_repo: AntennaRepository) -> Self {
        Self {
            antenna_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Get an antenna by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<antenna::Model>> {
        self.antenna_repo.find_by_id(id).await
    }

    /// Get an antenna by ID with ownership check.
    pub async fn get_by_id_for_user(&self, id: &str, user_id: &str) -> AppResult<antenna::Model> {
        let antenna = self.antenna_repo.get_by_id(id).await?;

        if antenna.user_id != user_id {
            return Err(AppError::Forbidden("Not the antenna owner".to_string()));
        }

        Ok(antenna)
    }

    /// List antennas for a user.
    pub async fn list_antennas(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<antenna::Model>> {
        self.antenna_repo.find_by_user(user_id, limit, offset).await
    }

    /// Count antennas for a user.
    pub async fn count_antennas(&self, user_id: &str) -> AppResult<u64> {
        self.antenna_repo.count_by_user(user_id).await
    }

    /// Create a new antenna.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateAntennaInput,
    ) -> AppResult<antenna::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check antenna limit
        let count = self.antenna_repo.count_by_user(user_id).await?;
        if count >= MAX_ANTENNAS_PER_USER {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_ANTENNAS_PER_USER} antennas allowed per user"
            )));
        }

        // Validate keywords
        self.validate_keywords(&input.keywords)?;
        self.validate_keywords(&input.exclude_keywords)?;

        // Validate source-specific requirements
        match input.src {
            AntennaSource::List => {
                if input.user_list_id.is_none() {
                    return Err(AppError::Validation(
                        "User list ID is required when source is 'list'".to_string(),
                    ));
                }
            }
            AntennaSource::Users => {
                if input.users.is_empty() {
                    return Err(AppError::Validation(
                        "At least one user is required when source is 'users'".to_string(),
                    ));
                }
            }
            AntennaSource::Instances => {
                if input.instances.is_empty() {
                    return Err(AppError::Validation(
                        "At least one instance is required when source is 'instances'".to_string(),
                    ));
                }
            }
            _ => {}
        }

        let id = self.id_gen.generate();
        let now = Utc::now();

        let model = antenna::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            name: Set(input.name),
            src: Set(input.src),
            user_list_id: Set(input.user_list_id),
            keywords: Set(json!(input.keywords)),
            exclude_keywords: Set(json!(input.exclude_keywords)),
            users: Set(json!(input.users)),
            instances: Set(json!(input.instances)),
            case_sensitive: Set(input.case_sensitive),
            with_replies: Set(input.with_replies),
            with_file: Set(input.with_file),
            notify: Set(input.notify),
            local_only: Set(input.local_only),
            is_active: Set(true),
            display_order: Set(0),
            notes_count: Set(0),
            last_used_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.antenna_repo.create(model).await
    }

    /// Update an antenna.
    pub async fn update(
        &self,
        user_id: &str,
        input: UpdateAntennaInput,
    ) -> AppResult<antenna::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Get antenna and verify ownership
        let antenna = self.get_by_id_for_user(&input.antenna_id, user_id).await?;

        // Validate keywords if provided
        if let Some(ref keywords) = input.keywords {
            self.validate_keywords(keywords)?;
        }
        if let Some(ref exclude_keywords) = input.exclude_keywords {
            self.validate_keywords(exclude_keywords)?;
        }

        let now = Utc::now();
        let mut active: antenna::ActiveModel = antenna.into();

        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(src) = input.src {
            active.src = Set(src);
        }
        if let Some(user_list_id) = input.user_list_id {
            active.user_list_id = Set(user_list_id);
        }
        if let Some(keywords) = input.keywords {
            active.keywords = Set(json!(keywords));
        }
        if let Some(exclude_keywords) = input.exclude_keywords {
            active.exclude_keywords = Set(json!(exclude_keywords));
        }
        if let Some(users) = input.users {
            active.users = Set(json!(users));
        }
        if let Some(instances) = input.instances {
            active.instances = Set(json!(instances));
        }
        if let Some(case_sensitive) = input.case_sensitive {
            active.case_sensitive = Set(case_sensitive);
        }
        if let Some(with_replies) = input.with_replies {
            active.with_replies = Set(with_replies);
        }
        if let Some(with_file) = input.with_file {
            active.with_file = Set(with_file);
        }
        if let Some(notify) = input.notify {
            active.notify = Set(notify);
        }
        if let Some(local_only) = input.local_only {
            active.local_only = Set(local_only);
        }
        if let Some(is_active) = input.is_active {
            active.is_active = Set(is_active);
        }

        active.updated_at = Set(Some(now.into()));

        self.antenna_repo.update(active).await
    }

    /// Delete an antenna.
    pub async fn delete(&self, antenna_id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        self.get_by_id_for_user(antenna_id, user_id).await?;
        self.antenna_repo.delete(antenna_id).await
    }

    /// Reorder antennas.
    pub async fn reorder(&self, user_id: &str, antenna_ids: Vec<String>) -> AppResult<()> {
        for (index, antenna_id) in antenna_ids.iter().enumerate() {
            // Verify ownership
            if let Ok(antenna) = self.antenna_repo.get_by_id(antenna_id).await
                && antenna.user_id == user_id
            {
                self.antenna_repo
                    .update_display_order(antenna_id, index as i32)
                    .await?;
            }
        }

        Ok(())
    }

    // ==================== Antenna Notes ====================

    /// Get notes from an antenna.
    pub async fn get_notes(
        &self,
        antenna_id: &str,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<antenna_note::Model>> {
        // Verify ownership
        self.get_by_id_for_user(antenna_id, user_id).await?;

        self.antenna_repo
            .find_notes_in_antenna(antenna_id, limit, until_id)
            .await
    }

    /// Mark all notes in an antenna as read.
    pub async fn mark_all_as_read(&self, antenna_id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        self.get_by_id_for_user(antenna_id, user_id).await?;
        self.antenna_repo.mark_notes_as_read(antenna_id).await
    }

    /// Get unread count for an antenna.
    pub async fn get_unread_count(&self, antenna_id: &str, user_id: &str) -> AppResult<u64> {
        // Verify ownership
        self.get_by_id_for_user(antenna_id, user_id).await?;
        self.antenna_repo.count_unread_notes(antenna_id).await
    }

    // ==================== Note Matching ====================

    /// Check if a note matches an antenna and add it if so.
    pub async fn process_note(
        &self,
        antenna: &antenna::Model,
        note_id: &str,
        context: &NoteMatchContext,
    ) -> AppResult<bool> {
        if !self.matches_antenna(antenna, context)? {
            return Ok(false);
        }

        // Check if already added
        if self
            .antenna_repo
            .is_note_in_antenna(&antenna.id, note_id)
            .await?
        {
            return Ok(false);
        }

        // Add note to antenna
        let id = self.id_gen.generate();
        self.antenna_repo
            .add_note(id, antenna.id.clone(), note_id.to_string())
            .await?;

        Ok(true)
    }

    /// Check if a note matches an antenna.
    pub fn matches_antenna(
        &self,
        antenna: &antenna::Model,
        context: &NoteMatchContext,
    ) -> AppResult<bool> {
        // Check if active
        if !antenna.is_active {
            return Ok(false);
        }

        // Check local_only
        if antenna.local_only && context.user_host.is_some() {
            return Ok(false);
        }

        // Check with_replies
        if !antenna.with_replies && context.is_reply {
            return Ok(false);
        }

        // Check with_file
        if antenna.with_file && !context.has_files {
            return Ok(false);
        }

        // Check source
        match antenna.src {
            AntennaSource::Users => {
                let users: Vec<String> =
                    serde_json::from_value(antenna.users.clone()).unwrap_or_default();
                if !users.contains(&context.user_id) {
                    return Ok(false);
                }
            }
            AntennaSource::List => {
                if let Some(ref list_id) = antenna.user_list_id {
                    if !context.list_memberships.contains(list_id) {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
            AntennaSource::Instances => {
                let instances: Vec<String> =
                    serde_json::from_value(antenna.instances.clone()).unwrap_or_default();
                match &context.user_host {
                    Some(host) => {
                        if !instances.iter().any(|i| i.eq_ignore_ascii_case(host)) {
                            return Ok(false);
                        }
                    }
                    None => {
                        // Local users don't match instance filter
                        return Ok(false);
                    }
                }
            }
            _ => {}
        }

        // Check keywords
        let keywords: Vec<Vec<String>> =
            serde_json::from_value(antenna.keywords.clone()).unwrap_or_default();
        let exclude_keywords: Vec<Vec<String>> =
            serde_json::from_value(antenna.exclude_keywords.clone()).unwrap_or_default();

        let text = if antenna.case_sensitive {
            context.text.clone()
        } else {
            context.text.to_lowercase()
        };

        // Check exclude keywords first (only if there are any)
        if !exclude_keywords.is_empty()
            && self.matches_keywords(&text, &exclude_keywords, antenna.case_sensitive)
        {
            return Ok(false);
        }

        // Check include keywords
        if !keywords.is_empty() && !self.matches_keywords(&text, &keywords, antenna.case_sensitive)
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Check if text matches keyword groups (OR of ANDs).
    fn matches_keywords(&self, text: &str, keywords: &[Vec<String>], case_sensitive: bool) -> bool {
        if keywords.is_empty() {
            return true;
        }

        // Each outer array is an OR group
        // Each inner array is an AND group
        for and_group in keywords {
            if and_group.is_empty() {
                continue;
            }

            // Build patterns for this AND group
            let patterns: Vec<String> = and_group
                .iter()
                .map(|k| {
                    if case_sensitive {
                        k.clone()
                    } else {
                        k.to_lowercase()
                    }
                })
                .collect();

            // Use Aho-Corasick for efficient multi-pattern matching
            if let Ok(ac) = AhoCorasick::new(&patterns) {
                let mut matched_patterns = vec![false; patterns.len()];

                for mat in ac.find_iter(text) {
                    matched_patterns[mat.pattern().as_usize()] = true;
                }

                // If all patterns in this AND group matched, return true (OR success)
                if matched_patterns.iter().all(|&m| m) {
                    return true;
                }
            }
        }

        false
    }

    /// Get all active antennas for note processing.
    pub async fn get_all_active_antennas(&self) -> AppResult<Vec<antenna::Model>> {
        self.antenna_repo.find_all_active().await
    }

    /// Process a note against all active antennas.
    ///
    /// This method checks if the note matches any active antennas and adds it to
    /// matching antennas. It's designed to be called after a note is created.
    ///
    /// Returns the list of antenna IDs that the note was added to.
    pub async fn process_note_for_all_antennas(
        &self,
        note_id: &str,
        context: &NoteMatchContext,
    ) -> AppResult<Vec<String>> {
        let antennas = self.get_all_active_antennas().await?;
        let mut matched_antenna_ids = Vec::new();

        for antenna in antennas {
            if matches!(
                self.process_note(&antenna, note_id, context).await,
                Ok(true)
            ) {
                matched_antenna_ids.push(antenna.id);
            }
        }

        Ok(matched_antenna_ids)
    }

    /// Create a `NoteMatchContext` from note data.
    ///
    /// Helper method to build the context needed for antenna matching.
    #[must_use]
    pub fn create_note_context(
        text: Option<&str>,
        user_id: &str,
        user_host: Option<&str>,
        reply_id: Option<&str>,
        file_ids: &[String],
        user_list_memberships: &[String],
    ) -> NoteMatchContext {
        NoteMatchContext {
            text: text.unwrap_or("").to_string(),
            user_id: user_id.to_string(),
            user_host: user_host.map(std::string::ToString::to_string),
            is_reply: reply_id.is_some(),
            has_files: !file_ids.is_empty(),
            list_memberships: user_list_memberships.to_vec(),
        }
    }

    // ==================== Helper Methods ====================

    fn validate_keywords(&self, keywords: &[Vec<String>]) -> AppResult<()> {
        let total_count: usize = keywords.iter().map(std::vec::Vec::len).sum();

        if total_count > MAX_KEYWORDS {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_KEYWORDS} keywords allowed"
            )));
        }

        for group in keywords {
            for keyword in group {
                if keyword.len() > MAX_KEYWORD_LENGTH {
                    return Err(AppError::Validation(format!(
                        "Keyword must be at most {MAX_KEYWORD_LENGTH} characters"
                    )));
                }
                if keyword.is_empty() {
                    return Err(AppError::Validation("Keywords cannot be empty".to_string()));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

    fn create_test_antenna(id: &str, user_id: &str, name: &str) -> antenna::Model {
        antenna::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            src: AntennaSource::All,
            user_list_id: None,
            keywords: json!([["test"]]),
            exclude_keywords: json!([]),
            users: json!([]),
            instances: json!([]),
            case_sensitive: false,
            with_replies: false,
            with_file: false,
            notify: false,
            local_only: false,
            is_active: true,
            display_order: 0,
            notes_count: 0,
            last_used_at: None,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let antenna = create_test_antenna("ant1", "user1", "My Antenna");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[antenna.clone()]])
                .into_connection(),
        );

        let repo = AntennaRepository::new(db);
        let service = AntennaService::new(repo);

        let result = service.get_by_id("ant1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Antenna");
    }

    #[test]
    fn test_matches_keywords_simple() {
        let service = AntennaService::new(AntennaRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // Simple keyword match
        assert!(service.matches_keywords("hello world", &[vec!["hello".to_string()]], false));
        assert!(!service.matches_keywords("goodbye world", &[vec!["hello".to_string()]], false));
    }

    #[test]
    fn test_matches_keywords_and() {
        let service = AntennaService::new(AntennaRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // AND group - all must match
        let keywords = vec![vec!["hello".to_string(), "world".to_string()]];
        assert!(service.matches_keywords("hello world", &keywords, false));
        assert!(!service.matches_keywords("hello there", &keywords, false));
    }

    #[test]
    fn test_matches_keywords_or() {
        let service = AntennaService::new(AntennaRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // OR groups - any group must match
        let keywords = vec![vec!["hello".to_string()], vec!["goodbye".to_string()]];
        assert!(service.matches_keywords("hello world", &keywords, false));
        assert!(service.matches_keywords("goodbye world", &keywords, false));
        assert!(!service.matches_keywords("hi world", &keywords, false));
    }

    #[test]
    fn test_matches_keywords_case_insensitive() {
        let service = AntennaService::new(AntennaRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        // When case_sensitive=false, both text and keywords are lowercased
        // The caller should lowercase the text before calling matches_keywords
        let keywords = vec![vec!["Hello".to_string()]];
        assert!(service.matches_keywords("hello world", &keywords, false));
        // HELLO WORLD should be lowercased to "hello world" before calling
        assert!(service.matches_keywords(&"HELLO WORLD".to_lowercase(), &keywords, false));
    }

    #[test]
    fn test_matches_keywords_case_sensitive() {
        let service = AntennaService::new(AntennaRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        let keywords = vec![vec!["Hello".to_string()]];
        assert!(service.matches_keywords("Hello world", &keywords, true));
        assert!(!service.matches_keywords("hello world", &keywords, true));
    }

    #[test]
    fn test_matches_antenna_local_only() {
        let service = AntennaService::new(AntennaRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        let mut antenna = create_test_antenna("ant1", "user1", "Local Antenna");
        antenna.local_only = true;
        antenna.keywords = json!([]);

        // Local user should match
        let local_context = NoteMatchContext {
            text: "test".to_string(),
            user_id: "user2".to_string(),
            user_host: None,
            is_reply: false,
            has_files: false,
            list_memberships: vec![],
        };
        assert!(service.matches_antenna(&antenna, &local_context).unwrap());

        // Remote user should not match
        let remote_context = NoteMatchContext {
            text: "test".to_string(),
            user_id: "user2".to_string(),
            user_host: Some("remote.example".to_string()),
            is_reply: false,
            has_files: false,
            list_memberships: vec![],
        };
        assert!(!service.matches_antenna(&antenna, &remote_context).unwrap());
    }

    #[test]
    fn test_matches_antenna_with_file() {
        let service = AntennaService::new(AntennaRepository::new(Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres).into_connection(),
        )));

        let mut antenna = create_test_antenna("ant1", "user1", "File Antenna");
        antenna.with_file = true;
        antenna.keywords = json!([]);

        let context_with_file = NoteMatchContext {
            text: "test".to_string(),
            user_id: "user2".to_string(),
            user_host: None,
            is_reply: false,
            has_files: true,
            list_memberships: vec![],
        };
        assert!(
            service
                .matches_antenna(&antenna, &context_with_file)
                .unwrap()
        );

        let context_without_file = NoteMatchContext {
            text: "test".to_string(),
            user_id: "user2".to_string(),
            user_host: None,
            is_reply: false,
            has_files: false,
            list_memberships: vec![],
        };
        assert!(
            !service
                .matches_antenna(&antenna, &context_without_file)
                .unwrap()
        );
    }
}
