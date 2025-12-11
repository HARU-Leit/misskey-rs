//! Gallery service for managing gallery posts.

use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::{gallery_like, gallery_post};
use misskey_db::repositories::GalleryRepository;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Maximum number of files per gallery post.
const MAX_FILES_PER_POST: usize = 32;

/// Maximum number of tags per gallery post.
const MAX_TAGS_PER_POST: usize = 32;

/// Input for creating a gallery post.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGalleryPostInput {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub file_ids: Vec<String>,
    #[serde(default)]
    pub is_sensitive: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Input for updating a gallery post.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGalleryPostInput {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub file_ids: Option<Vec<String>>,
    pub is_sensitive: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// Response for a gallery post.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryPostResponse {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: Option<String>,
    pub file_ids: Vec<String>,
    pub is_sensitive: bool,
    pub tags: Vec<String>,
    pub liked_count: i32,
    pub is_liked: Option<bool>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl From<gallery_post::Model> for GalleryPostResponse {
    fn from(p: gallery_post::Model) -> Self {
        Self {
            id: p.id,
            user_id: p.user_id,
            title: p.title,
            description: p.description,
            file_ids: serde_json::from_value(p.file_ids).unwrap_or_default(),
            is_sensitive: p.is_sensitive,
            tags: serde_json::from_value(p.tags).unwrap_or_default(),
            liked_count: p.liked_count,
            is_liked: None,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Service for managing gallery posts.
#[derive(Clone)]
pub struct GalleryService {
    gallery_repo: GalleryRepository,
    id_gen: IdGenerator,
}

impl GalleryService {
    /// Create a new gallery service.
    #[must_use]
    pub const fn new(gallery_repo: GalleryRepository) -> Self {
        Self {
            gallery_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new gallery post.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateGalleryPostInput,
    ) -> AppResult<GalleryPostResponse> {
        // Validate title
        if input.title.is_empty() || input.title.len() > 256 {
            return Err(AppError::Validation(
                "Title must be between 1 and 256 characters".to_string(),
            ));
        }

        // Validate description
        if let Some(ref desc) = input.description
            && desc.len() > 2048
        {
            return Err(AppError::Validation(
                "Description must be at most 2048 characters".to_string(),
            ));
        }

        // Validate file_ids
        if input.file_ids.is_empty() {
            return Err(AppError::Validation(
                "At least one file is required".to_string(),
            ));
        }
        if input.file_ids.len() > MAX_FILES_PER_POST {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_FILES_PER_POST} files allowed per post"
            )));
        }

        // Validate tags
        if input.tags.len() > MAX_TAGS_PER_POST {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_TAGS_PER_POST} tags allowed per post"
            )));
        }
        for tag in &input.tags {
            if tag.len() > 128 {
                return Err(AppError::Validation(
                    "Tag must be at most 128 characters".to_string(),
                ));
            }
        }

        // Check limit
        if self.gallery_repo.user_at_limit(user_id).await? {
            return Err(AppError::Validation(
                "Maximum number of gallery posts reached".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        let id = self.id_gen.generate();

        let model = gallery_post::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            title: Set(input.title),
            description: Set(input.description),
            file_ids: Set(json!(input.file_ids)),
            is_sensitive: Set(input.is_sensitive),
            tags: Set(json!(input.tags)),
            liked_count: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        let post = self.gallery_repo.create(model).await?;
        Ok(post.into())
    }

    /// Update a gallery post.
    pub async fn update(
        &self,
        user_id: &str,
        post_id: &str,
        input: UpdateGalleryPostInput,
    ) -> AppResult<GalleryPostResponse> {
        let post = self.gallery_repo.get_by_id(post_id).await?;

        // Verify ownership
        if post.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only update your own posts".to_string(),
            ));
        }

        let mut active: gallery_post::ActiveModel = post.into();

        if let Some(title) = input.title {
            if title.is_empty() || title.len() > 256 {
                return Err(AppError::Validation(
                    "Title must be between 1 and 256 characters".to_string(),
                ));
            }
            active.title = Set(title);
        }

        if let Some(description) = input.description {
            if let Some(ref desc) = description
                && desc.len() > 2048
            {
                return Err(AppError::Validation(
                    "Description must be at most 2048 characters".to_string(),
                ));
            }
            active.description = Set(description);
        }

        if let Some(file_ids) = input.file_ids {
            if file_ids.is_empty() {
                return Err(AppError::Validation(
                    "At least one file is required".to_string(),
                ));
            }
            if file_ids.len() > MAX_FILES_PER_POST {
                return Err(AppError::Validation(format!(
                    "Maximum of {MAX_FILES_PER_POST} files allowed per post"
                )));
            }
            active.file_ids = Set(json!(file_ids));
        }

        if let Some(is_sensitive) = input.is_sensitive {
            active.is_sensitive = Set(is_sensitive);
        }

        if let Some(tags) = input.tags {
            if tags.len() > MAX_TAGS_PER_POST {
                return Err(AppError::Validation(format!(
                    "Maximum of {MAX_TAGS_PER_POST} tags allowed per post"
                )));
            }
            for tag in &tags {
                if tag.len() > 128 {
                    return Err(AppError::Validation(
                        "Tag must be at most 128 characters".to_string(),
                    ));
                }
            }
            active.tags = Set(json!(tags));
        }

        active.updated_at = Set(Some(chrono::Utc::now().into()));

        let updated = self.gallery_repo.update(active).await?;
        Ok(updated.into())
    }

    /// Delete a gallery post.
    pub async fn delete(&self, user_id: &str, post_id: &str) -> AppResult<()> {
        let post = self.gallery_repo.get_by_id(post_id).await?;

        // Verify ownership
        if post.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only delete your own posts".to_string(),
            ));
        }

        self.gallery_repo.delete(post_id).await
    }

    /// Get a gallery post by ID.
    pub async fn get(
        &self,
        post_id: &str,
        viewer_id: Option<&str>,
    ) -> AppResult<GalleryPostResponse> {
        let post = self.gallery_repo.get_by_id(post_id).await?;
        let mut response: GalleryPostResponse = post.into();

        // Check if viewer has liked
        if let Some(uid) = viewer_id {
            response.is_liked = Some(self.gallery_repo.has_liked(post_id, uid).await?);
        }

        Ok(response)
    }

    /// List gallery posts for a user.
    pub async fn list_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<GalleryPostResponse>> {
        let posts = self
            .gallery_repo
            .find_by_user_id(user_id, limit, offset)
            .await?;
        Ok(posts.into_iter().map(Into::into).collect())
    }

    /// List all gallery posts with pagination.
    pub async fn list(&self, limit: u64, offset: u64) -> AppResult<Vec<GalleryPostResponse>> {
        let posts = self
            .gallery_repo
            .find_with_pagination(limit, offset)
            .await?;
        Ok(posts.into_iter().map(Into::into).collect())
    }

    /// List featured gallery posts (most liked).
    pub async fn list_featured(&self, limit: Option<u64>) -> AppResult<Vec<GalleryPostResponse>> {
        let posts = self.gallery_repo.find_featured(limit.unwrap_or(10)).await?;
        Ok(posts.into_iter().map(Into::into).collect())
    }

    /// List popular gallery posts.
    pub async fn list_popular(&self, limit: Option<u64>) -> AppResult<Vec<GalleryPostResponse>> {
        let posts = self.gallery_repo.find_popular(limit.unwrap_or(10)).await?;
        Ok(posts.into_iter().map(Into::into).collect())
    }

    /// Search gallery posts by tag.
    pub async fn search_by_tag(
        &self,
        tag: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<GalleryPostResponse>> {
        let posts = self.gallery_repo.find_by_tag(tag, limit, offset).await?;
        Ok(posts.into_iter().map(Into::into).collect())
    }

    /// Like a gallery post.
    pub async fn like(&self, user_id: &str, post_id: &str) -> AppResult<()> {
        let post = self.gallery_repo.get_by_id(post_id).await?;

        // Can't like own post
        if post.user_id == user_id {
            return Err(AppError::Validation(
                "Cannot like your own post".to_string(),
            ));
        }

        // Check if already liked
        if self.gallery_repo.has_liked(post_id, user_id).await? {
            return Err(AppError::Conflict("Already liked this post".to_string()));
        }

        let now = chrono::Utc::now();
        let id = self.id_gen.generate();

        let model = gallery_like::ActiveModel {
            id: Set(id),
            post_id: Set(post_id.to_string()),
            user_id: Set(user_id.to_string()),
            created_at: Set(now.into()),
        };

        self.gallery_repo.like(model).await?;
        Ok(())
    }

    /// Unlike a gallery post.
    pub async fn unlike(&self, user_id: &str, post_id: &str) -> AppResult<()> {
        // Check if liked
        if !self.gallery_repo.has_liked(post_id, user_id).await? {
            return Err(AppError::NotFound("Like not found".to_string()));
        }

        self.gallery_repo.unlike(post_id, user_id).await
    }

    /// Get posts liked by a user.
    pub async fn liked_posts(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<GalleryPostResponse>> {
        let likes = self
            .gallery_repo
            .find_liked_by_user(user_id, limit, offset)
            .await?;
        let mut posts = Vec::new();

        for like in likes {
            if let Ok(post) = self.gallery_repo.get_by_id(&like.post_id).await {
                let mut response: GalleryPostResponse = post.into();
                response.is_liked = Some(true);
                posts.push(response);
            }
        }

        Ok(posts)
    }
}
