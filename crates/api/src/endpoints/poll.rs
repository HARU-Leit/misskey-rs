//! Poll endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};

use crate::{
    extractors::{AuthUser, MaybeAuthUser},
    middleware::AppState,
    response::ApiResponse,
};

/// Poll response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollResponse {
    pub choices: Vec<PollChoiceResponse>,
    pub multiple: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub voters_count: i32,
    pub is_expired: bool,
}

/// Poll choice response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollChoiceResponse {
    pub text: String,
    pub votes: i32,
    pub is_voted: bool,
}

/// Show poll request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowPollRequest {
    pub note_id: String,
}

/// Get poll details.
async fn show_poll(
    MaybeAuthUser(maybe_user): MaybeAuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowPollRequest>,
) -> AppResult<ApiResponse<PollResponse>> {
    let user_id = maybe_user.map(|u| u.id);
    let poll_status = state
        .poll_service
        .get_poll_with_status(&req.note_id, user_id.as_deref())
        .await?;

    let choices: Vec<String> = serde_json::from_value(poll_status.poll.choices.clone())
        .map_err(|e| misskey_common::AppError::Internal(format!("Invalid poll choices: {e}")))?;
    let votes: Vec<i32> = serde_json::from_value(poll_status.poll.votes.clone())
        .map_err(|e| misskey_common::AppError::Internal(format!("Invalid poll votes: {e}")))?;

    let choice_responses: Vec<PollChoiceResponse> = choices
        .into_iter()
        .enumerate()
        .map(|(i, text)| PollChoiceResponse {
            text,
            votes: votes.get(i).copied().unwrap_or(0),
            is_voted: poll_status.user_votes.contains(&(i as i32)),
        })
        .collect();

    Ok(ApiResponse::ok(PollResponse {
        choices: choice_responses,
        multiple: poll_status.poll.multiple,
        expires_at: poll_status.poll.expires_at.map(|e| e.to_rfc3339()),
        voters_count: poll_status.poll.voters_count,
        is_expired: poll_status.is_expired,
    }))
}

/// Vote request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteRequest {
    pub note_id: String,
    pub choice: i32,
}

/// Vote on a poll.
async fn vote(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<VoteRequest>,
) -> AppResult<ApiResponse<PollResponse>> {
    let poll = state
        .poll_service
        .vote(&user.id, &req.note_id, req.choice)
        .await?;

    // Get updated poll with user's votes
    let user_votes = state
        .poll_service
        .get_poll_with_status(&req.note_id, Some(&user.id))
        .await?;

    let choices: Vec<String> = serde_json::from_value(poll.choices.clone())
        .map_err(|e| misskey_common::AppError::Internal(format!("Invalid poll choices: {e}")))?;
    let votes: Vec<i32> = serde_json::from_value(poll.votes.clone())
        .map_err(|e| misskey_common::AppError::Internal(format!("Invalid poll votes: {e}")))?;

    let choice_responses: Vec<PollChoiceResponse> = choices
        .into_iter()
        .enumerate()
        .map(|(i, text)| PollChoiceResponse {
            text,
            votes: votes.get(i).copied().unwrap_or(0),
            is_voted: user_votes.user_votes.contains(&(i as i32)),
        })
        .collect();

    let is_expired = poll
        .expires_at
        .as_ref()
        .is_some_and(|exp| *exp < chrono::Utc::now());

    Ok(ApiResponse::ok(PollResponse {
        choices: choice_responses,
        multiple: poll.multiple,
        expires_at: poll.expires_at.map(|e| e.to_rfc3339()),
        voters_count: poll.voters_count,
        is_expired,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/show", post(show_poll))
        .route("/vote", post(vote))
}
