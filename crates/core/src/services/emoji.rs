//! Emoji service.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{entities::emoji, repositories::EmojiRepository};
use sea_orm::Set;
use serde_json::json;

/// Service for custom emoji operations.
#[derive(Clone)]
pub struct EmojiService {
    emoji_repo: EmojiRepository,
    id_gen: IdGenerator,
}

impl EmojiService {
    /// Create a new emoji service.
    #[must_use]
    pub const fn new(emoji_repo: EmojiRepository) -> Self {
        Self {
            emoji_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Get all local emojis.
    pub async fn list_local(&self) -> AppResult<Vec<emoji::Model>> {
        self.emoji_repo.find_local().await
    }

    /// Get local emojis with pagination.
    pub async fn list_local_paginated(
        &self,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<emoji::Model>> {
        self.emoji_repo.find_local_paginated(limit, offset).await
    }

    /// Get emojis by category.
    pub async fn list_by_category(&self, category: &str) -> AppResult<Vec<emoji::Model>> {
        self.emoji_repo.find_by_category(category).await
    }

    /// Get all categories.
    pub async fn list_categories(&self) -> AppResult<Vec<String>> {
        self.emoji_repo.find_categories().await
    }

    /// Search emojis.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<emoji::Model>> {
        self.emoji_repo.search(query, limit, offset).await
    }

    /// Get emoji by name.
    pub async fn get_by_name(&self, name: &str) -> AppResult<Option<emoji::Model>> {
        self.emoji_repo.find_by_name(name).await
    }

    /// Get emoji by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<emoji::Model>> {
        self.emoji_repo.find_by_id(id).await
    }

    /// Create a new emoji.
    pub async fn create(
        &self,
        name: String,
        url: String,
        content_type: String,
        category: Option<String>,
        aliases: Vec<String>,
        is_sensitive: bool,
        local_only: bool,
        license: Option<String>,
        width: Option<i32>,
        height: Option<i32>,
    ) -> AppResult<emoji::Model> {
        // Check if emoji with same name already exists
        if self.emoji_repo.find_by_name(&name).await?.is_some() {
            return Err(AppError::BadRequest(format!(
                "Emoji with name '{name}' already exists"
            )));
        }

        // Validate name (only alphanumeric and underscores)
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(AppError::BadRequest(
                "Emoji name can only contain alphanumeric characters and underscores".to_string(),
            ));
        }

        let model = emoji::ActiveModel {
            id: Set(self.id_gen.generate()),
            name: Set(name),
            category: Set(category),
            original_url: Set(url.clone()),
            static_url: Set(None),
            content_type: Set(content_type),
            aliases: Set(json!(aliases)),
            host: Set(None),
            license: Set(license),
            is_sensitive: Set(is_sensitive),
            local_only: Set(local_only),
            width: Set(width),
            height: Set(height),
            size: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(None),
        };

        self.emoji_repo.create(model).await
    }

    /// Update an emoji.
    pub async fn update(
        &self,
        id: &str,
        name: Option<String>,
        category: Option<Option<String>>,
        aliases: Option<Vec<String>>,
        is_sensitive: Option<bool>,
        local_only: Option<bool>,
        license: Option<Option<String>>,
    ) -> AppResult<emoji::Model> {
        let emoji = self
            .emoji_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Emoji not found: {id}")))?;

        // Check if new name conflicts with existing emoji
        if let Some(ref new_name) = name
            && new_name != &emoji.name
        {
            if self.emoji_repo.find_by_name(new_name).await?.is_some() {
                return Err(AppError::BadRequest(format!(
                    "Emoji with name '{new_name}' already exists"
                )));
            }

            // Validate name
            if !new_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(AppError::BadRequest(
                    "Emoji name can only contain alphanumeric characters and underscores"
                        .to_string(),
                ));
            }
        }

        let mut model: emoji::ActiveModel = emoji.into();

        if let Some(n) = name {
            model.name = Set(n);
        }
        if let Some(c) = category {
            model.category = Set(c);
        }
        if let Some(a) = aliases {
            model.aliases = Set(json!(a));
        }
        if let Some(s) = is_sensitive {
            model.is_sensitive = Set(s);
        }
        if let Some(l) = local_only {
            model.local_only = Set(l);
        }
        if let Some(lic) = license {
            model.license = Set(lic);
        }

        model.updated_at = Set(Some(chrono::Utc::now()));

        self.emoji_repo.update(model).await
    }

    /// Delete an emoji.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        // Verify emoji exists
        self.emoji_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Emoji not found: {id}")))?;

        self.emoji_repo.delete(id).await
    }

    /// Copy an emoji from a remote instance.
    pub async fn copy_from_remote(
        &self,
        name: &str,
        host: &str,
        url: &str,
        content_type: &str,
    ) -> AppResult<emoji::Model> {
        let model = emoji::ActiveModel {
            id: Set(self.id_gen.generate()),
            name: Set(name.to_string()),
            category: Set(None),
            original_url: Set(url.to_string()),
            static_url: Set(None),
            content_type: Set(content_type.to_string()),
            aliases: Set(json!([])),
            host: Set(Some(host.to_string())),
            license: Set(None),
            is_sensitive: Set(false),
            local_only: Set(false),
            width: Set(None),
            height: Set(None),
            size: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(None),
        };

        self.emoji_repo.import_remote(model).await
    }

    /// Count local emojis.
    pub async fn count(&self) -> AppResult<u64> {
        self.emoji_repo.count_local().await
    }

    /// Get multiple emojis by names.
    pub async fn get_by_names(&self, names: &[String]) -> AppResult<Vec<emoji::Model>> {
        self.emoji_repo.find_by_names(names).await
    }
}
