//! Meta settings repository.

use std::sync::Arc;

use crate::entities::{MetaSettings, meta_settings};
use misskey_common::{AppError, AppResult};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;

/// Repository for instance-wide settings.
#[derive(Clone)]
pub struct MetaSettingsRepository {
    db: Arc<DatabaseConnection>,
}

/// Singleton ID for the meta settings
pub const META_SETTINGS_ID: &str = "instance";

impl MetaSettingsRepository {
    /// Create a new meta settings repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Get the instance settings, creating default if not exists.
    pub async fn get_or_create(&self) -> AppResult<meta_settings::Model> {
        if let Some(settings) = self.find().await? {
            return Ok(settings);
        }

        // Create default settings
        let now = chrono::Utc::now();
        let model = meta_settings::ActiveModel {
            id: Set(META_SETTINGS_ID.to_string()),
            name: Set(Some("misskey-rs".to_string())),
            short_name: Set(Some("misskey-rs".to_string())),
            description: Set(Some("A Misskey server implemented in Rust".to_string())),
            maintainer_name: Set(None),
            maintainer_email: Set(None),
            langs: Set(json!(["ja", "en"])),
            icon_url: Set(None),
            banner_url: Set(None),
            theme_color: Set(None),
            disable_registration: Set(false),
            email_required_for_signup: Set(false),
            require_registration_approval: Set(false),
            force_nsfw_media: Set(false),
            default_blur_nsfw: Set(true),
            default_hide_ads: Set(false),
            max_note_text_length: Set(3000),
            max_remote_note_text_length: Set(10000),
            max_page_content_length: Set(65536),
            max_pages_per_user: Set(100),
            default_drive_capacity_mb: Set(1024),
            max_file_size_mb: Set(256),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find the instance settings.
    pub async fn find(&self) -> AppResult<Option<meta_settings::Model>> {
        MetaSettings::find()
            .filter(meta_settings::Column::Id.eq(META_SETTINGS_ID))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update the instance settings.
    pub async fn update(
        &self,
        model: meta_settings::ActiveModel,
    ) -> AppResult<meta_settings::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
