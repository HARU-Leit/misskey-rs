//! Clip service.

use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::{clip, clip_note};
use misskey_db::repositories::ClipRepository;

/// Service for managing clips.
#[derive(Clone)]
pub struct ClipService {
    clip_repo: ClipRepository,
    id_gen: IdGenerator,
}

impl ClipService {
    /// Create a new clip service.
    #[must_use]
    pub const fn new(clip_repo: ClipRepository) -> Self {
        Self {
            clip_repo,
            id_gen: IdGenerator::new(),
        }
    }

    // ==================== Clip Operations ====================

    /// Get a clip by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<clip::Model>> {
        self.clip_repo.find_by_id(id).await
    }

    /// Get a clip by ID, verifying access permissions.
    pub async fn get_by_id_with_access(
        &self,
        id: &str,
        viewer_id: Option<&str>,
    ) -> AppResult<Option<clip::Model>> {
        let clip = self.clip_repo.find_by_id(id).await?;

        match clip {
            Some(c) => {
                // Check access: owner can always see, others can only see public clips
                if c.is_public || viewer_id == Some(&c.user_id) {
                    Ok(Some(c))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// List clips for a user (own clips).
    pub async fn list_my_clips(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip::Model>> {
        self.clip_repo.find_by_user(user_id, limit, offset).await
    }

    /// List public clips for a user (viewing someone else's clips).
    pub async fn list_user_clips(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip::Model>> {
        self.clip_repo
            .find_public_by_user(user_id, limit, offset)
            .await
    }

    /// Count clips for a user.
    pub async fn count_my_clips(&self, user_id: &str) -> AppResult<u64> {
        self.clip_repo.count_by_user(user_id).await
    }

    /// Create a new clip.
    pub async fn create(
        &self,
        user_id: &str,
        name: String,
        description: Option<String>,
        is_public: bool,
    ) -> AppResult<clip::Model> {
        // Validate name length
        if name.is_empty() || name.len() > 128 {
            return Err(AppError::Validation(
                "Clip name must be between 1 and 128 characters".to_string(),
            ));
        }

        // Validate description length
        if let Some(ref desc) = description {
            if desc.len() > 2048 {
                return Err(AppError::Validation(
                    "Clip description must be at most 2048 characters".to_string(),
                ));
            }
        }

        let id = self.id_gen.generate();

        self.clip_repo
            .create(id, user_id.to_string(), name, description, is_public)
            .await
    }

    /// Update a clip.
    pub async fn update(
        &self,
        id: &str,
        user_id: &str,
        name: Option<String>,
        description: Option<Option<String>>,
        is_public: Option<bool>,
    ) -> AppResult<clip::Model> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        // Validate name if provided
        if let Some(ref n) = name {
            if n.is_empty() || n.len() > 128 {
                return Err(AppError::Validation(
                    "Clip name must be between 1 and 128 characters".to_string(),
                ));
            }
        }

        // Validate description if provided
        if let Some(Some(ref desc)) = description {
            if desc.len() > 2048 {
                return Err(AppError::Validation(
                    "Clip description must be at most 2048 characters".to_string(),
                ));
            }
        }

        self.clip_repo
            .update(id, name, description, is_public)
            .await
    }

    /// Delete a clip.
    pub async fn delete(&self, id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        self.clip_repo.delete(id).await
    }

    /// Reorder clips.
    pub async fn reorder(&self, user_id: &str, clip_ids: Vec<String>) -> AppResult<()> {
        for (index, clip_id) in clip_ids.iter().enumerate() {
            // Verify ownership
            let clip = self.clip_repo.find_by_id(clip_id).await?;

            if let Some(c) = clip {
                if c.user_id == user_id {
                    self.clip_repo
                        .update_display_order(clip_id, index as i32)
                        .await?;
                }
            }
        }

        Ok(())
    }

    // ==================== Clip Note Operations ====================

    /// List notes in a clip.
    pub async fn list_notes(
        &self,
        clip_id: &str,
        viewer_id: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip_note::Model>> {
        // Verify access
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if !clip.is_public && viewer_id != Some(&clip.user_id) {
            return Err(AppError::Forbidden("Cannot access this clip".to_string()));
        }

        self.clip_repo
            .find_notes_in_clip(clip_id, limit, offset)
            .await
    }

    /// Add a note to a clip.
    pub async fn add_note(
        &self,
        clip_id: &str,
        note_id: &str,
        user_id: &str,
        comment: Option<String>,
    ) -> AppResult<clip_note::Model> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        // Check if already in clip
        if self.clip_repo.is_note_in_clip(clip_id, note_id).await? {
            return Err(AppError::Validation(
                "Note is already in this clip".to_string(),
            ));
        }

        // Validate comment length
        if let Some(ref c) = comment {
            if c.len() > 512 {
                return Err(AppError::Validation(
                    "Comment must be at most 512 characters".to_string(),
                ));
            }
        }

        let id = self.id_gen.generate();

        self.clip_repo
            .add_note_to_clip(id, clip_id.to_string(), note_id.to_string(), comment)
            .await
    }

    /// Remove a note from a clip.
    pub async fn remove_note(&self, clip_id: &str, note_id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        self.clip_repo.remove_note_from_clip(clip_id, note_id).await
    }

    /// Update clip note comment.
    pub async fn update_note_comment(
        &self,
        clip_note_id: &str,
        user_id: &str,
        comment: Option<String>,
    ) -> AppResult<()> {
        // Get clip note
        let clip_note = self
            .clip_repo
            .find_clip_note_by_id(clip_note_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip note not found".to_string()))?;

        // Verify ownership via clip
        let clip = self
            .clip_repo
            .find_by_id(&clip_note.clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        // Validate comment length
        if let Some(ref c) = comment {
            if c.len() > 512 {
                return Err(AppError::Validation(
                    "Comment must be at most 512 characters".to_string(),
                ));
            }
        }

        self.clip_repo
            .update_clip_note_comment(clip_note_id, comment)
            .await
    }

    /// Reorder notes in a clip.
    pub async fn reorder_notes(
        &self,
        clip_id: &str,
        user_id: &str,
        clip_note_ids: Vec<String>,
    ) -> AppResult<()> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        for (index, cn_id) in clip_note_ids.iter().enumerate() {
            self.clip_repo
                .update_clip_note_order(cn_id, index as i32)
                .await?;
        }

        Ok(())
    }

    /// Find which of user's clips contain a note.
    pub async fn find_clips_with_note(
        &self,
        note_id: &str,
        user_id: &str,
    ) -> AppResult<Vec<clip_note::Model>> {
        self.clip_repo
            .find_clips_containing_note(note_id, user_id)
            .await
    }

    /// Check if note is in any of user's clips.
    pub async fn is_note_clipped(&self, note_id: &str, user_id: &str) -> AppResult<bool> {
        let clips = self.find_clips_with_note(note_id, user_id).await?;
        Ok(!clips.is_empty())
    }

    // ==================== Search ====================

    /// Search notes within a clip by text content.
    /// Returns note IDs that match the query.
    pub async fn search_notes(
        &self,
        clip_id: &str,
        viewer_id: Option<&str>,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<String>> {
        // Verify access
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if !clip.is_public && viewer_id != Some(&clip.user_id) {
            return Err(AppError::Forbidden("Cannot access this clip".to_string()));
        }

        // Validate query
        if query.trim().is_empty() {
            return Err(AppError::BadRequest(
                "Search query cannot be empty".to_string(),
            ));
        }

        self.clip_repo
            .search_notes_in_clip(clip_id, query, limit, offset)
            .await
    }

    /// Search notes within a clip by comment content.
    pub async fn search_by_comment(
        &self,
        clip_id: &str,
        viewer_id: Option<&str>,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip_note::Model>> {
        // Verify access
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if !clip.is_public && viewer_id != Some(&clip.user_id) {
            return Err(AppError::Forbidden("Cannot access this clip".to_string()));
        }

        // Validate query
        if query.trim().is_empty() {
            return Err(AppError::BadRequest(
                "Search query cannot be empty".to_string(),
            ));
        }

        self.clip_repo
            .search_notes_by_comment(clip_id, query, limit, offset)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

    fn create_test_clip(id: &str, user_id: &str, name: &str, is_public: bool) -> clip::Model {
        clip::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            description: None,
            is_public,
            notes_count: 0,
            display_order: 0,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let clip = create_test_clip("clip1", "user1", "My Clip", false);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip.clone()]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let service = ClipService::new(repo);

        let result = service.get_by_id("clip1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Clip");
    }

    #[tokio::test]
    async fn test_get_by_id_with_access_public() {
        let clip = create_test_clip("clip1", "user1", "Public Clip", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip.clone()]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let service = ClipService::new(repo);

        // Different user can see public clip
        let result = service
            .get_by_id_with_access("clip1", Some("user2"))
            .await
            .unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_get_by_id_with_access_private_denied() {
        let clip = create_test_clip("clip1", "user1", "Private Clip", false);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip.clone()]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let service = ClipService::new(repo);

        // Different user cannot see private clip
        let result = service
            .get_by_id_with_access("clip1", Some("user2"))
            .await
            .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_with_access_private_owner() {
        let clip = create_test_clip("clip1", "user1", "Private Clip", false);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip.clone()]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let service = ClipService::new(repo);

        // Owner can see their own private clip
        let result = service
            .get_by_id_with_access("clip1", Some("user1"))
            .await
            .unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_list_my_clips() {
        let clip1 = create_test_clip("clip1", "user1", "Clip 1", false);
        let clip2 = create_test_clip("clip2", "user1", "Clip 2", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip1, clip2]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let service = ClipService::new(repo);

        let result = service.list_my_clips("user1", 10, 0).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_list_user_clips_only_public() {
        let clip = create_test_clip("clip1", "user1", "Public Clip", true);

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[clip]])
                .into_connection(),
        );

        let repo = ClipRepository::new(db);
        let service = ClipService::new(repo);

        let result = service.list_user_clips("user1", 10, 0).await.unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].is_public);
    }
}
