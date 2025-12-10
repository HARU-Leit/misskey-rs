//! Following endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::FollowResult;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Follow request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowRequest {
    pub user_id: String,
}

/// Follow result response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowResponse {
    pub status: String,
}

/// Follow a user.
async fn follow(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FollowRequest>,
) -> AppResult<ApiResponse<FollowResponse>> {
    let result = state.following_service.follow(&user.id, &req.user_id).await?;

    let status = match &result {
        FollowResult::Following => {
            // Create follow notification for the followee
            let _ = state
                .notification_service
                .create_follow_notification(&req.user_id, &user.id)
                .await;
            "following"
        }
        FollowResult::Pending => {
            // Create follow request notification for the followee
            let _ = state
                .notification_service
                .create_follow_request_notification(&req.user_id, &user.id)
                .await;
            "pending"
        }
    };

    Ok(ApiResponse::ok(FollowResponse {
        status: status.to_string(),
    }))
}

/// Unfollow a user.
async fn unfollow(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FollowRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .following_service
        .unfollow(&user.id, &req.user_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Accept follow request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptFollowRequest {
    pub user_id: String,
}

/// Accept a follow request.
async fn accept(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<AcceptFollowRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .following_service
        .accept_request(&user.id, &req.user_id)
        .await?;

    // Notify the requester that their follow request was accepted
    let _ = state
        .notification_service
        .create_follow_request_accepted_notification(&req.user_id, &user.id)
        .await;

    Ok(ApiResponse::ok(()))
}

/// Reject a follow request.
async fn reject(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<AcceptFollowRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .following_service
        .reject_request(&user.id, &req.user_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Cancel a follow request.
async fn cancel(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<FollowRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .following_service
        .cancel_request(&user.id, &req.user_id)
        .await?;
    Ok(ApiResponse::ok(()))
}

/// Pending follow requests response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowRequestItemResponse {
    pub id: String,
    pub created_at: String,
    pub follower_id: String,
    pub followee_id: String,
}

impl From<misskey_db::entities::follow_request::Model> for FollowRequestItemResponse {
    fn from(f: misskey_db::entities::follow_request::Model) -> Self {
        Self {
            id: f.id,
            created_at: f.created_at.to_rfc3339(),
            follower_id: f.follower_id,
            followee_id: f.followee_id,
        }
    }
}

/// List pending request params.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingListRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

/// List received follow requests (pending).
async fn list_pending(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<PendingListRequest>,
) -> AppResult<ApiResponse<Vec<FollowRequestItemResponse>>> {
    let limit = req.limit.min(100);
    let requests = state
        .following_service
        .get_pending_requests(&user.id, limit, req.until_id.as_deref())
        .await?;

    Ok(ApiResponse::ok(
        requests.into_iter().map(Into::into).collect(),
    ))
}

/// List followers/following request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequest {
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    pub until_id: Option<String>,
}

const fn default_limit() -> u64 {
    10
}

/// Following item response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowingItemResponse {
    pub id: String,
    pub created_at: String,
    pub follower_id: String,
    pub followee_id: String,
}

impl From<misskey_db::entities::following::Model> for FollowingItemResponse {
    fn from(f: misskey_db::entities::following::Model) -> Self {
        Self {
            id: f.id,
            created_at: f.created_at.to_rfc3339(),
            follower_id: f.follower_id,
            followee_id: f.followee_id,
        }
    }
}

/// Get followers of a user.
async fn followers(
    State(state): State<AppState>,
    Json(req): Json<ListRequest>,
) -> AppResult<ApiResponse<Vec<FollowingItemResponse>>> {
    let limit = req.limit.min(100);
    let followers = state
        .following_service
        .get_followers(&req.user_id, limit, req.until_id.as_deref())
        .await?;

    Ok(ApiResponse::ok(
        followers.into_iter().map(Into::into).collect(),
    ))
}

/// Get users that a user is following.
async fn following(
    State(state): State<AppState>,
    Json(req): Json<ListRequest>,
) -> AppResult<ApiResponse<Vec<FollowingItemResponse>>> {
    let limit = req.limit.min(100);
    let following = state
        .following_service
        .get_following(&req.user_id, limit, req.until_id.as_deref())
        .await?;

    Ok(ApiResponse::ok(
        following.into_iter().map(Into::into).collect(),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(follow))
        .route("/delete", post(unfollow))
        .route("/requests/accept", post(accept))
        .route("/requests/reject", post(reject))
        .route("/requests/cancel", post(cancel))
        .route("/requests/list", post(list_pending))
        .route("/followers", post(followers))
        .route("/following", post(following))
}
