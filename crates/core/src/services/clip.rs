//! Clip service.

use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::{clip, clip_note};
use misskey_db::repositories::ClipRepository;
use serde::{Deserialize, Serialize};

// Re-export for convenience
pub use misskey_db::repositories::SmartClipConditions;

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
        if let Some(ref desc) = description
            && desc.len() > 2048
        {
            return Err(AppError::Validation(
                "Clip description must be at most 2048 characters".to_string(),
            ));
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
        if let Some(ref n) = name
            && (n.is_empty() || n.len() > 128)
        {
            return Err(AppError::Validation(
                "Clip name must be between 1 and 128 characters".to_string(),
            ));
        }

        // Validate description if provided
        if let Some(Some(ref desc)) = description
            && desc.len() > 2048
        {
            return Err(AppError::Validation(
                "Clip description must be at most 2048 characters".to_string(),
            ));
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

            if let Some(c) = clip
                && c.user_id == user_id
            {
                self.clip_repo
                    .update_display_order(clip_id, index as i32)
                    .await?;
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
        if let Some(ref c) = comment
            && c.len() > 512
        {
            return Err(AppError::Validation(
                "Comment must be at most 512 characters".to_string(),
            ));
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
        if let Some(ref c) = comment
            && c.len() > 512
        {
            return Err(AppError::Validation(
                "Comment must be at most 512 characters".to_string(),
            ));
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

    // ==================== Smart Clip Operations ====================

    /// Create a smart clip with automatic note collection conditions.
    pub async fn create_smart_clip(
        &self,
        user_id: &str,
        name: String,
        description: Option<String>,
        is_public: bool,
        conditions: SmartClipConditions,
        max_notes: Option<i32>,
    ) -> AppResult<clip::Model> {
        // Validate name length
        if name.is_empty() || name.len() > 128 {
            return Err(AppError::Validation(
                "Clip name must be between 1 and 128 characters".to_string(),
            ));
        }

        // Validate description length
        if let Some(ref desc) = description
            && desc.len() > 2048
        {
            return Err(AppError::Validation(
                "Clip description must be at most 2048 characters".to_string(),
            ));
        }

        // Validate conditions - at least one must be set
        if conditions.keywords.is_empty()
            && conditions.users.is_empty()
            && conditions.hashtags.is_empty()
            && conditions.min_reactions.is_none()
            && !conditions.has_files
        {
            return Err(AppError::Validation(
                "Smart clip must have at least one condition".to_string(),
            ));
        }

        // Validate max_notes
        if let Some(max) = max_notes
            && max < 1
        {
            return Err(AppError::Validation(
                "Max notes must be at least 1".to_string(),
            ));
        }

        let id = self.id_gen.generate();

        self.clip_repo
            .create_smart_clip(
                id,
                user_id.to_string(),
                name,
                description,
                is_public,
                conditions,
                max_notes,
            )
            .await
    }

    /// Update smart clip conditions.
    pub async fn update_smart_conditions(
        &self,
        clip_id: &str,
        user_id: &str,
        conditions: SmartClipConditions,
        max_notes: Option<i32>,
    ) -> AppResult<clip::Model> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        if !clip.is_smart_clip {
            return Err(AppError::BadRequest(
                "This clip is not a smart clip".to_string(),
            ));
        }

        // Validate conditions
        if conditions.keywords.is_empty()
            && conditions.users.is_empty()
            && conditions.hashtags.is_empty()
            && conditions.min_reactions.is_none()
            && !conditions.has_files
        {
            return Err(AppError::Validation(
                "Smart clip must have at least one condition".to_string(),
            ));
        }

        self.clip_repo
            .update_smart_conditions(clip_id, conditions, max_notes)
            .await
    }

    /// Convert a regular clip to a smart clip.
    pub async fn convert_to_smart_clip(
        &self,
        clip_id: &str,
        user_id: &str,
        conditions: SmartClipConditions,
        max_notes: Option<i32>,
    ) -> AppResult<clip::Model> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        if clip.is_smart_clip {
            return Err(AppError::BadRequest(
                "This clip is already a smart clip".to_string(),
            ));
        }

        // Validate conditions
        if conditions.keywords.is_empty()
            && conditions.users.is_empty()
            && conditions.hashtags.is_empty()
            && conditions.min_reactions.is_none()
            && !conditions.has_files
        {
            return Err(AppError::Validation(
                "Smart clip must have at least one condition".to_string(),
            ));
        }

        self.clip_repo
            .convert_to_smart_clip(clip_id, conditions, max_notes)
            .await
    }

    /// Convert a smart clip to a regular clip (removes conditions, keeps notes).
    pub async fn convert_to_regular_clip(
        &self,
        clip_id: &str,
        user_id: &str,
    ) -> AppResult<clip::Model> {
        // Verify ownership
        let clip = self
            .clip_repo
            .find_by_id(clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Clip not found".to_string()))?;

        if clip.user_id != user_id {
            return Err(AppError::Forbidden("Not the clip owner".to_string()));
        }

        if !clip.is_smart_clip {
            return Err(AppError::BadRequest(
                "This clip is not a smart clip".to_string(),
            ));
        }

        self.clip_repo.convert_to_regular_clip(clip_id).await
    }

    /// Get smart clips for a user.
    pub async fn list_smart_clips(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<clip::Model>> {
        self.clip_repo
            .find_smart_clips_by_user(user_id, limit, offset)
            .await
    }

    // ==================== Clip Note Move/Copy Operations ====================

    /// Copy a note from one clip to another.
    pub async fn copy_note(
        &self,
        source_clip_id: &str,
        target_clip_id: &str,
        note_id: &str,
        user_id: &str,
    ) -> AppResult<clip_note::Model> {
        // Verify ownership of source clip
        let source = self
            .clip_repo
            .find_by_id(source_clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Source clip not found".to_string()))?;

        if source.user_id != user_id {
            return Err(AppError::Forbidden("Not the source clip owner".to_string()));
        }

        // Verify ownership of target clip
        let target = self
            .clip_repo
            .find_by_id(target_clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Target clip not found".to_string()))?;

        if target.user_id != user_id {
            return Err(AppError::Forbidden("Not the target clip owner".to_string()));
        }

        // Verify note exists in source clip
        if !self
            .clip_repo
            .is_note_in_clip(source_clip_id, note_id)
            .await?
        {
            return Err(AppError::NotFound(
                "Note not found in source clip".to_string(),
            ));
        }

        // Check if already in target clip
        if self
            .clip_repo
            .is_note_in_clip(target_clip_id, note_id)
            .await?
        {
            return Err(AppError::Validation(
                "Note is already in target clip".to_string(),
            ));
        }

        // Add to target clip
        let id = self.id_gen.generate();
        self.clip_repo
            .add_note_to_clip(id, target_clip_id.to_string(), note_id.to_string(), None)
            .await
    }

    /// Move a note from one clip to another.
    pub async fn move_note(
        &self,
        source_clip_id: &str,
        target_clip_id: &str,
        note_id: &str,
        user_id: &str,
    ) -> AppResult<clip_note::Model> {
        // First copy to target
        let result = self
            .copy_note(source_clip_id, target_clip_id, note_id, user_id)
            .await?;

        // Then remove from source
        self.clip_repo
            .remove_note_from_clip(source_clip_id, note_id)
            .await?;

        Ok(result)
    }

    /// Copy multiple notes from one clip to another.
    pub async fn copy_notes_bulk(
        &self,
        source_clip_id: &str,
        target_clip_id: &str,
        note_ids: Vec<String>,
        user_id: &str,
    ) -> AppResult<BulkOperationResult> {
        // Verify ownership of both clips
        let source = self
            .clip_repo
            .find_by_id(source_clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Source clip not found".to_string()))?;

        if source.user_id != user_id {
            return Err(AppError::Forbidden("Not the source clip owner".to_string()));
        }

        let target = self
            .clip_repo
            .find_by_id(target_clip_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Target clip not found".to_string()))?;

        if target.user_id != user_id {
            return Err(AppError::Forbidden("Not the target clip owner".to_string()));
        }

        let mut copied = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        for note_id in note_ids {
            // Check if in source
            if !self
                .clip_repo
                .is_note_in_clip(source_clip_id, &note_id)
                .await?
            {
                errors.push(format!("{note_id}: not in source clip"));
                continue;
            }

            // Check if already in target
            if self
                .clip_repo
                .is_note_in_clip(target_clip_id, &note_id)
                .await?
            {
                skipped += 1;
                continue;
            }

            // Copy
            let id = self.id_gen.generate();
            match self
                .clip_repo
                .add_note_to_clip(id, target_clip_id.to_string(), note_id.clone(), None)
                .await
            {
                Ok(_) => copied += 1,
                Err(e) => errors.push(format!("{note_id}: {e}")),
            }
        }

        Ok(BulkOperationResult {
            processed: copied,
            skipped,
            errors,
        })
    }

    /// Move multiple notes from one clip to another.
    pub async fn move_notes_bulk(
        &self,
        source_clip_id: &str,
        target_clip_id: &str,
        note_ids: Vec<String>,
        user_id: &str,
    ) -> AppResult<BulkOperationResult> {
        // First copy all
        let copy_result = self
            .copy_notes_bulk(source_clip_id, target_clip_id, note_ids.clone(), user_id)
            .await?;

        // Then remove successfully copied ones from source
        for note_id in &note_ids {
            // Only remove if it was copied (exists in target now)
            if self
                .clip_repo
                .is_note_in_clip(target_clip_id, note_id)
                .await?
            {
                let _ = self
                    .clip_repo
                    .remove_note_from_clip(source_clip_id, note_id)
                    .await;
            }
        }

        Ok(copy_result)
    }
}

/// Result of a bulk operation on clip notes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOperationResult {
    /// Number of notes successfully processed.
    pub processed: u64,
    /// Number of notes skipped (already exists, etc.).
    pub skipped: u64,
    /// Errors encountered.
    pub errors: Vec<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
            is_smart_clip: false,
            smart_conditions: None,
            smart_max_notes: None,
            smart_last_processed_at: None,
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
