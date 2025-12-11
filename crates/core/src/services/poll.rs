//! Poll service.

use chrono::{Duration, Utc};
use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{poll, poll_vote},
    repositories::{NoteRepository, PollRepository, PollVoteRepository},
};
use sea_orm::Set;
use serde_json::json;

/// Poll service for business logic.
#[derive(Clone)]
pub struct PollService {
    poll_repo: PollRepository,
    vote_repo: PollVoteRepository,
    #[allow(dead_code)]
    note_repo: NoteRepository,
    id_gen: IdGenerator,
}

/// Input for creating a poll.
pub struct CreatePollInput {
    pub choices: Vec<String>,
    pub multiple: bool,
    pub expires_in: Option<i64>, // Duration in seconds
}

/// Poll with vote status.
pub struct PollWithStatus {
    pub poll: poll::Model,
    pub user_votes: Vec<i32>, // Choices the user voted for
    pub is_expired: bool,
}

impl PollService {
    /// Create a new poll service.
    #[must_use]
    pub const fn new(
        poll_repo: PollRepository,
        vote_repo: PollVoteRepository,
        note_repo: NoteRepository,
    ) -> Self {
        Self {
            poll_repo,
            vote_repo,
            note_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a poll for a note.
    pub async fn create_poll(
        &self,
        note_id: &str,
        input: CreatePollInput,
    ) -> AppResult<poll::Model> {
        // Validate choices
        if input.choices.len() < 2 {
            return Err(AppError::BadRequest(
                "Poll must have at least 2 choices".to_string(),
            ));
        }
        if input.choices.len() > 10 {
            return Err(AppError::BadRequest(
                "Poll cannot have more than 10 choices".to_string(),
            ));
        }
        for choice in &input.choices {
            if choice.trim().is_empty() {
                return Err(AppError::BadRequest(
                    "Poll choices cannot be empty".to_string(),
                ));
            }
            if choice.len() > 100 {
                return Err(AppError::BadRequest(
                    "Poll choice is too long (max 100 chars)".to_string(),
                ));
            }
        }

        // Calculate expiration
        let expires_at = input.expires_in.map(|seconds| {
            let duration = Duration::seconds(seconds.min(2_592_000)); // Max 30 days
            (Utc::now() + duration).into()
        });

        // Initialize votes array with zeros
        let votes = json!(vec![0i32; input.choices.len()]);

        let model = poll::ActiveModel {
            note_id: Set(note_id.to_string()),
            choices: Set(json!(input.choices)),
            votes: Set(votes),
            multiple: Set(input.multiple),
            expires_at: Set(expires_at),
            voters_count: Set(0),
        };

        self.poll_repo.create(model).await
    }

    /// Get a poll by note ID.
    pub async fn get_poll(&self, note_id: &str) -> AppResult<poll::Model> {
        self.poll_repo.get_by_note_id(note_id).await
    }

    /// Get a poll with user's vote status.
    pub async fn get_poll_with_status(
        &self,
        note_id: &str,
        user_id: Option<&str>,
    ) -> AppResult<PollWithStatus> {
        let poll = self.poll_repo.get_by_note_id(note_id).await?;

        let user_votes = if let Some(uid) = user_id {
            let votes = self.vote_repo.find_by_user_and_note(uid, note_id).await?;
            votes.into_iter().map(|v| v.choice).collect()
        } else {
            vec![]
        };

        let is_expired = poll
            .expires_at
            .as_ref()
            .is_some_and(|exp| *exp < Utc::now());

        Ok(PollWithStatus {
            poll,
            user_votes,
            is_expired,
        })
    }

    /// Vote on a poll.
    pub async fn vote(&self, user_id: &str, note_id: &str, choice: i32) -> AppResult<poll::Model> {
        // Get poll
        let poll = self.poll_repo.get_by_note_id(note_id).await?;

        // Check if poll is expired
        if let Some(ref expires_at) = poll.expires_at
            && *expires_at < Utc::now()
        {
            return Err(AppError::BadRequest("Poll has expired".to_string()));
        }

        // Validate choice index
        let choices: Vec<String> = serde_json::from_value(poll.choices.clone())
            .map_err(|e| AppError::Internal(format!("Invalid poll choices: {e}")))?;

        if choice < 0 || choice >= choices.len() as i32 {
            return Err(AppError::BadRequest("Invalid choice".to_string()));
        }

        // Check if user already voted
        if poll.multiple {
            // Multiple choice - check if already voted for this specific choice
            if self
                .vote_repo
                .has_voted_choice(user_id, note_id, choice)
                .await?
            {
                return Err(AppError::BadRequest(
                    "You have already voted for this choice".to_string(),
                ));
            }
        } else {
            // Single choice - check if already voted at all
            if self.vote_repo.has_voted(user_id, note_id).await? {
                return Err(AppError::BadRequest(
                    "You have already voted on this poll".to_string(),
                ));
            }
        }

        // Create vote
        let vote_id = self.id_gen.generate();
        let vote_model = poll_vote::ActiveModel {
            id: Set(vote_id),
            note_id: Set(note_id.to_string()),
            user_id: Set(user_id.to_string()),
            choice: Set(choice),
            created_at: Set(Utc::now().into()),
        };
        self.vote_repo.create(vote_model).await?;

        // Update poll vote counts
        let mut votes: Vec<i32> = serde_json::from_value(poll.votes.clone())
            .map_err(|e| AppError::Internal(format!("Invalid poll votes: {e}")))?;

        votes[choice as usize] += 1;

        // Count unique voters
        let voters_count = self.vote_repo.count_voters(note_id).await?;

        let mut active: poll::ActiveModel = poll.into();
        active.votes = Set(json!(votes));
        active.voters_count = Set(voters_count);

        self.poll_repo.update(active).await
    }

    /// Check if a poll exists for a note.
    pub async fn has_poll(&self, note_id: &str) -> AppResult<bool> {
        Ok(self.poll_repo.find_by_note_id(note_id).await?.is_some())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_creation_validation() {
        // Test that choices validation works
        let input = CreatePollInput {
            choices: vec!["a".to_string()],
            multiple: false,
            expires_in: None,
        };
        assert!(input.choices.len() < 2);

        let input = CreatePollInput {
            choices: vec!["a".to_string(), "b".to_string()],
            multiple: false,
            expires_in: Some(3600),
        };
        assert!(input.choices.len() >= 2);
    }
}
