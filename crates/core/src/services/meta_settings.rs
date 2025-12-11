//! Meta settings service for instance configuration.

use misskey_common::{AppError, AppResult};
use misskey_db::entities::{meta_settings, meta_settings::META_SETTINGS_ID};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Input for updating meta settings.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UpdateMetaSettingsInput {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub maintainer_name: Option<String>,
    pub maintainer_email: Option<String>,
    pub langs: Option<Vec<String>>,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub theme_color: Option<String>,
    pub disable_registration: Option<bool>,
    pub email_required_for_signup: Option<bool>,
    pub require_registration_approval: Option<bool>,
    pub force_nsfw_media: Option<bool>,
    pub default_blur_nsfw: Option<bool>,
    pub default_hide_ads: Option<bool>,
    pub max_note_text_length: Option<i32>,
    pub max_remote_note_text_length: Option<i32>,
    pub max_page_content_length: Option<i32>,
    pub max_pages_per_user: Option<i32>,
    pub default_drive_capacity_mb: Option<i32>,
    pub max_file_size_mb: Option<i32>,
}

/// Meta settings service for managing instance configuration.
#[derive(Clone)]
pub struct MetaSettingsService {
    db: Arc<DatabaseConnection>,
}

impl MetaSettingsService {
    /// Create a new meta settings service.
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Get meta settings, creating default if not exists.
    pub async fn get(&self) -> AppResult<meta_settings::Model> {
        let settings = meta_settings::Entity::find_by_id(META_SETTINGS_ID)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        match settings {
            Some(s) => Ok(s),
            None => {
                // Create default settings
                let now = chrono::Utc::now();
                let model = meta_settings::ActiveModel {
                    id: Set(META_SETTINGS_ID.to_string()),
                    name: Set(Some("Misskey-RS Instance".to_string())),
                    short_name: Set(Some("misskey-rs".to_string())),
                    description: Set(None),
                    maintainer_name: Set(None),
                    maintainer_email: Set(None),
                    langs: Set(serde_json::json!(["en"])),
                    icon_url: Set(None),
                    banner_url: Set(None),
                    theme_color: Set(Some("#86b300".to_string())),
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

                let result = model
                    .insert(self.db.as_ref())
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

                Ok(result)
            }
        }
    }

    /// Update meta settings.
    pub async fn update(&self, input: UpdateMetaSettingsInput) -> AppResult<meta_settings::Model> {
        // Ensure settings exist
        let _ = self.get().await?;

        let now = chrono::Utc::now();
        let mut model = meta_settings::ActiveModel {
            id: Set(META_SETTINGS_ID.to_string()),
            updated_at: Set(Some(now.into())),
            ..Default::default()
        };

        if let Some(name) = input.name {
            model.name = Set(Some(name));
        }
        if let Some(short_name) = input.short_name {
            model.short_name = Set(Some(short_name));
        }
        if let Some(description) = input.description {
            model.description = Set(Some(description));
        }
        if let Some(maintainer_name) = input.maintainer_name {
            model.maintainer_name = Set(Some(maintainer_name));
        }
        if let Some(maintainer_email) = input.maintainer_email {
            model.maintainer_email = Set(Some(maintainer_email));
        }
        if let Some(langs) = input.langs {
            model.langs = Set(serde_json::json!(langs));
        }
        if let Some(icon_url) = input.icon_url {
            model.icon_url = Set(Some(icon_url));
        }
        if let Some(banner_url) = input.banner_url {
            model.banner_url = Set(Some(banner_url));
        }
        if let Some(theme_color) = input.theme_color {
            model.theme_color = Set(Some(theme_color));
        }
        if let Some(disable_registration) = input.disable_registration {
            model.disable_registration = Set(disable_registration);
        }
        if let Some(email_required_for_signup) = input.email_required_for_signup {
            model.email_required_for_signup = Set(email_required_for_signup);
        }
        if let Some(require_registration_approval) = input.require_registration_approval {
            model.require_registration_approval = Set(require_registration_approval);
        }
        if let Some(force_nsfw_media) = input.force_nsfw_media {
            model.force_nsfw_media = Set(force_nsfw_media);
        }
        if let Some(default_blur_nsfw) = input.default_blur_nsfw {
            model.default_blur_nsfw = Set(default_blur_nsfw);
        }
        if let Some(default_hide_ads) = input.default_hide_ads {
            model.default_hide_ads = Set(default_hide_ads);
        }
        if let Some(max_note_text_length) = input.max_note_text_length {
            model.max_note_text_length = Set(max_note_text_length);
        }
        if let Some(max_remote_note_text_length) = input.max_remote_note_text_length {
            model.max_remote_note_text_length = Set(max_remote_note_text_length);
        }
        if let Some(max_page_content_length) = input.max_page_content_length {
            model.max_page_content_length = Set(max_page_content_length);
        }
        if let Some(max_pages_per_user) = input.max_pages_per_user {
            model.max_pages_per_user = Set(max_pages_per_user);
        }
        if let Some(default_drive_capacity_mb) = input.default_drive_capacity_mb {
            model.default_drive_capacity_mb = Set(default_drive_capacity_mb);
        }
        if let Some(max_file_size_mb) = input.max_file_size_mb {
            model.max_file_size_mb = Set(max_file_size_mb);
        }

        let result = model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result)
    }

    /// Get note text length limit for local users.
    pub async fn get_local_note_limit(&self) -> AppResult<i32> {
        let settings = self.get().await?;
        Ok(settings.max_note_text_length)
    }

    /// Get note text length limit for remote users.
    pub async fn get_remote_note_limit(&self) -> AppResult<i32> {
        let settings = self.get().await?;
        Ok(settings.max_remote_note_text_length)
    }

    /// Check if registration approval is required.
    pub async fn is_registration_approval_required(&self) -> AppResult<bool> {
        let settings = self.get().await?;
        Ok(settings.require_registration_approval)
    }

    /// Check if NSFW media is forced.
    pub async fn is_force_nsfw_media(&self) -> AppResult<bool> {
        let settings = self.get().await?;
        Ok(settings.force_nsfw_media)
    }
}
