//! Poll repository.

use std::sync::Arc;

use crate::entities::{poll, poll_vote, Poll, PollVote};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
};

/// Poll repository for database operations.
#[derive(Clone)]
pub struct PollRepository {
    db: Arc<DatabaseConnection>,
}

impl PollRepository {
    /// Create a new poll repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a poll by note ID.
    pub async fn find_by_note_id(&self, note_id: &str) -> AppResult<Option<poll::Model>> {
        Poll::find_by_id(note_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a poll by note ID, returning error if not found.
    pub async fn get_by_note_id(&self, note_id: &str) -> AppResult<poll::Model> {
        self.find_by_note_id(note_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Poll not found for note: {note_id}")))
    }

    /// Create a new poll.
    pub async fn create(&self, model: poll::ActiveModel) -> AppResult<poll::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a poll.
    pub async fn update(&self, model: poll::ActiveModel) -> AppResult<poll::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a poll.
    pub async fn delete(&self, note_id: &str) -> AppResult<()> {
        Poll::delete_by_id(note_id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}

/// Poll vote repository for database operations.
#[derive(Clone)]
pub struct PollVoteRepository {
    db: Arc<DatabaseConnection>,
}

impl PollVoteRepository {
    /// Create a new poll vote repository.
    #[must_use] 
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a vote by user and note.
    pub async fn find_by_user_and_note(
        &self,
        user_id: &str,
        note_id: &str,
    ) -> AppResult<Vec<poll_vote::Model>> {
        PollVote::find()
            .filter(poll_vote::Column::UserId.eq(user_id))
            .filter(poll_vote::Column::NoteId.eq(note_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if user has voted on a poll.
    pub async fn has_voted(&self, user_id: &str, note_id: &str) -> AppResult<bool> {
        let count = PollVote::find()
            .filter(poll_vote::Column::UserId.eq(user_id))
            .filter(poll_vote::Column::NoteId.eq(note_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    /// Check if user has voted for a specific choice.
    pub async fn has_voted_choice(
        &self,
        user_id: &str,
        note_id: &str,
        choice: i32,
    ) -> AppResult<bool> {
        let count = PollVote::find()
            .filter(poll_vote::Column::UserId.eq(user_id))
            .filter(poll_vote::Column::NoteId.eq(note_id))
            .filter(poll_vote::Column::Choice.eq(choice))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    /// Create a new vote.
    pub async fn create(&self, model: poll_vote::ActiveModel) -> AppResult<poll_vote::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get votes for a note.
    pub async fn find_by_note(&self, note_id: &str) -> AppResult<Vec<poll_vote::Model>> {
        PollVote::find()
            .filter(poll_vote::Column::NoteId.eq(note_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count unique voters for a note.
    pub async fn count_voters(&self, note_id: &str) -> AppResult<i32> {
        use sea_orm::QueryOrder;
        // Count distinct user_ids - simplified approach
        let votes = PollVote::find()
            .filter(poll_vote::Column::NoteId.eq(note_id))
            .order_by_asc(poll_vote::Column::UserId)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut unique_users = std::collections::HashSet::new();
        for vote in votes {
            unique_users.insert(vote.user_id);
        }
        Ok(unique_users.len() as i32)
    }
}
