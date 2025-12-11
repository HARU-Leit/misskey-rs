//! Page service for managing user pages.

use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::{page, page_like};
use misskey_db::repositories::PageRepository;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Input for creating a page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePageInput {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub summary: Option<String>,
    pub content: Vec<serde_json::Value>,
    #[serde(default)]
    pub variables: Vec<serde_json::Value>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub visible_user_ids: Vec<String>,
    #[serde(default)]
    pub eyecatch_image_id: Option<String>,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub hide_title_when_pinned: bool,
    #[serde(default)]
    pub align_center: bool,
}

fn default_visibility() -> String {
    "public".to_string()
}

/// Input for updating a page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePageInput {
    pub name: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: Option<Vec<serde_json::Value>>,
    pub variables: Option<Vec<serde_json::Value>>,
    pub script: Option<String>,
    pub visibility: Option<String>,
    pub visible_user_ids: Option<Vec<String>>,
    pub eyecatch_image_id: Option<String>,
    pub font: Option<String>,
    pub hide_title_when_pinned: Option<bool>,
    pub align_center: Option<bool>,
}

/// Response for a page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub title: String,
    pub summary: Option<String>,
    pub content: Vec<serde_json::Value>,
    pub variables: Vec<serde_json::Value>,
    pub script: Option<String>,
    pub visibility: String,
    pub visible_user_ids: Vec<String>,
    pub eyecatch_image_id: Option<String>,
    pub font: Option<String>,
    pub hide_title_when_pinned: bool,
    pub align_center: bool,
    pub liked_count: i32,
    pub view_count: i32,
    pub is_liked: Option<bool>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl From<page::Model> for PageResponse {
    fn from(p: page::Model) -> Self {
        Self {
            id: p.id,
            user_id: p.user_id,
            name: p.name,
            title: p.title,
            summary: p.summary,
            content: serde_json::from_value(p.content).unwrap_or_default(),
            variables: serde_json::from_value(p.variables).unwrap_or_default(),
            script: p.script,
            visibility: match p.visibility {
                page::PageVisibility::Public => "public".to_string(),
                page::PageVisibility::Followers => "followers".to_string(),
                page::PageVisibility::Specified => "specified".to_string(),
            },
            visible_user_ids: serde_json::from_value(p.visible_user_ids).unwrap_or_default(),
            eyecatch_image_id: p.eyecatch_image_id,
            font: p.font,
            hide_title_when_pinned: p.hide_title_when_pinned,
            align_center: p.align_center,
            liked_count: p.liked_count,
            view_count: p.view_count,
            is_liked: None,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Service for managing pages.
#[derive(Clone)]
pub struct PageService {
    page_repo: PageRepository,
    id_gen: IdGenerator,
}

impl PageService {
    /// Create a new page service.
    #[must_use]
    pub const fn new(page_repo: PageRepository) -> Self {
        Self {
            page_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new page.
    pub async fn create(&self, user_id: &str, input: CreatePageInput) -> AppResult<PageResponse> {
        // Validate name
        if input.name.is_empty() || input.name.len() > 256 {
            return Err(AppError::Validation(
                "Name must be between 1 and 256 characters".to_string(),
            ));
        }

        // Validate name format (URL-safe)
        if !input
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::Validation(
                "Name must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        // Validate title
        if input.title.is_empty() || input.title.len() > 256 {
            return Err(AppError::Validation(
                "Title must be between 1 and 256 characters".to_string(),
            ));
        }

        // Validate visibility
        let visibility = match input.visibility.as_str() {
            "public" => page::PageVisibility::Public,
            "followers" => page::PageVisibility::Followers,
            "specified" => page::PageVisibility::Specified,
            _ => {
                return Err(AppError::Validation(format!(
                    "Invalid visibility: {}",
                    input.visibility
                )));
            }
        };

        // Check limit
        if self.page_repo.user_at_limit(user_id).await? {
            return Err(AppError::Validation(
                "Maximum number of pages reached".to_string(),
            ));
        }

        // Check for duplicate name
        if self.page_repo.name_exists(user_id, &input.name).await? {
            return Err(AppError::Conflict(format!(
                "Page with name '{}' already exists",
                input.name
            )));
        }

        let now = chrono::Utc::now();
        let id = self.id_gen.generate();

        let model = page::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            name: Set(input.name),
            title: Set(input.title),
            summary: Set(input.summary),
            content: Set(json!(input.content)),
            variables: Set(json!(input.variables)),
            script: Set(input.script),
            visibility: Set(visibility),
            visible_user_ids: Set(json!(input.visible_user_ids)),
            eyecatch_image_id: Set(input.eyecatch_image_id),
            file_ids: Set(json!([])),
            font: Set(input.font),
            hide_title_when_pinned: Set(input.hide_title_when_pinned),
            align_center: Set(input.align_center),
            liked_count: Set(0),
            view_count: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        let page = self.page_repo.create(model).await?;
        Ok(page.into())
    }

    /// Update a page.
    pub async fn update(
        &self,
        user_id: &str,
        page_id: &str,
        input: UpdatePageInput,
    ) -> AppResult<PageResponse> {
        let page = self.page_repo.get_by_id(page_id).await?;

        // Verify ownership
        if page.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only update your own pages".to_string(),
            ));
        }

        let mut active: page::ActiveModel = page.into();

        if let Some(name) = input.name {
            if name.is_empty() || name.len() > 256 {
                return Err(AppError::Validation(
                    "Name must be between 1 and 256 characters".to_string(),
                ));
            }
            if !name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err(AppError::Validation(
                    "Name must contain only alphanumeric characters, hyphens, and underscores"
                        .to_string(),
                ));
            }
            // Check for duplicate name (excluding current page)
            if let Some(existing) = self.page_repo.find_by_user_and_name(user_id, &name).await?
                && existing.id != page_id
            {
                return Err(AppError::Conflict(format!(
                    "Page with name '{name}' already exists"
                )));
            }
            active.name = Set(name);
        }

        if let Some(title) = input.title {
            if title.is_empty() || title.len() > 256 {
                return Err(AppError::Validation(
                    "Title must be between 1 and 256 characters".to_string(),
                ));
            }
            active.title = Set(title);
        }

        if let Some(summary) = input.summary {
            active.summary = Set(Some(summary));
        }

        if let Some(content) = input.content {
            active.content = Set(json!(content));
        }

        if let Some(variables) = input.variables {
            active.variables = Set(json!(variables));
        }

        if let Some(script) = input.script {
            active.script = Set(Some(script));
        }

        if let Some(visibility_str) = input.visibility {
            let visibility = match visibility_str.as_str() {
                "public" => page::PageVisibility::Public,
                "followers" => page::PageVisibility::Followers,
                "specified" => page::PageVisibility::Specified,
                _ => {
                    return Err(AppError::Validation(format!(
                        "Invalid visibility: {visibility_str}"
                    )));
                }
            };
            active.visibility = Set(visibility);
        }

        if let Some(visible_user_ids) = input.visible_user_ids {
            active.visible_user_ids = Set(json!(visible_user_ids));
        }

        if let Some(eyecatch_image_id) = input.eyecatch_image_id {
            active.eyecatch_image_id = Set(Some(eyecatch_image_id));
        }

        if let Some(font) = input.font {
            active.font = Set(Some(font));
        }

        if let Some(hide_title_when_pinned) = input.hide_title_when_pinned {
            active.hide_title_when_pinned = Set(hide_title_when_pinned);
        }

        if let Some(align_center) = input.align_center {
            active.align_center = Set(align_center);
        }

        active.updated_at = Set(Some(chrono::Utc::now().into()));

        let updated = self.page_repo.update(active).await?;
        Ok(updated.into())
    }

    /// Delete a page.
    pub async fn delete(&self, user_id: &str, page_id: &str) -> AppResult<()> {
        let page = self.page_repo.get_by_id(page_id).await?;

        // Verify ownership
        if page.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only delete your own pages".to_string(),
            ));
        }

        self.page_repo.delete(page_id).await
    }

    /// Get a page by ID.
    pub async fn get(&self, page_id: &str, viewer_id: Option<&str>) -> AppResult<PageResponse> {
        let page = self.page_repo.get_by_id(page_id).await?;

        // Check visibility
        if !self.can_view(&page, viewer_id) {
            return Err(AppError::NotFound(format!("Page: {page_id}")));
        }

        // Increment view count
        let _ = self.page_repo.increment_view_count(page_id).await;

        let mut response: PageResponse = page.into();

        // Check if viewer has liked
        if let Some(uid) = viewer_id {
            response.is_liked = Some(self.page_repo.has_liked(page_id, uid).await?);
        }

        Ok(response)
    }

    /// Get a page by username and name.
    pub async fn get_by_name(
        &self,
        user_id: &str,
        name: &str,
        viewer_id: Option<&str>,
    ) -> AppResult<PageResponse> {
        let page = self
            .page_repo
            .find_by_user_and_name(user_id, name)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Page: @{user_id}/{name}")))?;

        // Check visibility
        if !self.can_view(&page, viewer_id) {
            return Err(AppError::NotFound(format!("Page: @{user_id}/{name}")));
        }

        // Increment view count
        let _ = self.page_repo.increment_view_count(&page.id).await;

        let mut response: PageResponse = page.into();

        // Check if viewer has liked
        if let Some(uid) = viewer_id {
            response.is_liked = Some(self.page_repo.has_liked(&response.id, uid).await?);
        }

        Ok(response)
    }

    /// List pages for a user.
    pub async fn list_by_user(&self, user_id: &str) -> AppResult<Vec<PageResponse>> {
        let pages = self.page_repo.find_by_user_id(user_id).await?;
        Ok(pages.into_iter().map(Into::into).collect())
    }

    /// List featured pages.
    pub async fn list_featured(&self, limit: Option<u64>) -> AppResult<Vec<PageResponse>> {
        let pages = self.page_repo.find_featured(limit.unwrap_or(10)).await?;
        Ok(pages.into_iter().map(Into::into).collect())
    }

    /// Like a page.
    pub async fn like(&self, user_id: &str, page_id: &str) -> AppResult<()> {
        let page = self.page_repo.get_by_id(page_id).await?;

        // Can't like own page
        if page.user_id == user_id {
            return Err(AppError::Validation(
                "Cannot like your own page".to_string(),
            ));
        }

        // Check if already liked
        if self.page_repo.has_liked(page_id, user_id).await? {
            return Err(AppError::Conflict("Already liked this page".to_string()));
        }

        let now = chrono::Utc::now();
        let id = self.id_gen.generate();

        let model = page_like::ActiveModel {
            id: Set(id),
            page_id: Set(page_id.to_string()),
            user_id: Set(user_id.to_string()),
            created_at: Set(now.into()),
        };

        self.page_repo.like(model).await?;
        Ok(())
    }

    /// Unlike a page.
    pub async fn unlike(&self, user_id: &str, page_id: &str) -> AppResult<()> {
        // Check if liked
        if !self.page_repo.has_liked(page_id, user_id).await? {
            return Err(AppError::NotFound("Like not found".to_string()));
        }

        self.page_repo.unlike(page_id, user_id).await
    }

    // ==================== Helper Methods ====================

    fn can_view(&self, page: &page::Model, viewer_id: Option<&str>) -> bool {
        // Owner can always view
        if let Some(uid) = viewer_id
            && page.user_id == uid
        {
            return true;
        }

        match page.visibility {
            page::PageVisibility::Public => true,
            page::PageVisibility::Followers => {
                // Would need to check following relationship
                // For now, only owner can see
                false
            }
            page::PageVisibility::Specified => {
                if let Some(uid) = viewer_id {
                    let visible_user_ids: Vec<String> =
                        serde_json::from_value(page.visible_user_ids.clone()).unwrap_or_default();
                    visible_user_ids.contains(&uid.to_string())
                } else {
                    false
                }
            }
        }
    }
}
