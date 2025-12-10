//! User lists endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::CreateListInput;
use misskey_db::entities::user_list;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// List response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse {
    pub id: String,
    pub name: String,
    pub is_public: bool,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_ids: Option<Vec<String>>,
}

impl From<user_list::Model> for ListResponse {
    fn from(list: user_list::Model) -> Self {
        Self {
            id: list.id,
            name: list.name,
            is_public: list.is_public,
            created_at: list.created_at.to_rfc3339(),
            user_ids: None,
        }
    }
}

/// Create list request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateListRequest {
    pub name: String,
    #[serde(default)]
    pub is_public: bool,
}

/// Update list request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateListRequest {
    pub list_id: String,
    pub name: Option<String>,
    pub is_public: Option<bool>,
}

/// Delete list request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteListRequest {
    pub list_id: String,
}

/// Show list request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowListRequest {
    pub list_id: String,
}

/// Member request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRequest {
    pub list_id: String,
    pub user_id: String,
}

/// Create a new list.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateListRequest>,
) -> AppResult<ApiResponse<ListResponse>> {
    let list = state
        .user_list_service
        .create(
            &user.id,
            CreateListInput {
                name: req.name,
                is_public: req.is_public,
            },
        )
        .await?;

    Ok(ApiResponse::ok(list.into()))
}

/// Update a list.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateListRequest>,
) -> AppResult<ApiResponse<ListResponse>> {
    let list = state
        .user_list_service
        .update(&user.id, &req.list_id, req.name, req.is_public)
        .await?;

    Ok(ApiResponse::ok(list.into()))
}

/// Delete a list.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteListRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .user_list_service
        .delete(&user.id, &req.list_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Get a list with members.
async fn show(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowListRequest>,
) -> AppResult<ApiResponse<ListResponse>> {
    // Check if user can view the list
    if !state
        .user_list_service
        .can_view(Some(&user.id), &req.list_id)
        .await?
    {
        return Err(misskey_common::AppError::Forbidden(
            "Cannot view this list".to_string(),
        ));
    }

    let list = state.user_list_service.get(&req.list_id).await?;
    let member_ids = state.user_list_service.get_members(&req.list_id).await?;

    let mut response: ListResponse = list.into();
    response.user_ids = Some(member_ids);

    Ok(ApiResponse::ok(response))
}

/// Get all lists for the current user.
async fn list(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<ListResponse>>> {
    let lists = state.user_list_service.get_lists(&user.id).await?;

    let responses: Vec<ListResponse> = lists.into_iter().map(std::convert::Into::into).collect();

    Ok(ApiResponse::ok(responses))
}

/// Add a user to a list.
async fn push(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<MemberRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .user_list_service
        .add_member(&user.id, &req.list_id, &req.user_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Remove a user from a list.
async fn pull(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<MemberRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .user_list_service
        .remove_member(&user.id, &req.list_id, &req.user_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/update", post(update))
        .route("/delete", post(delete))
        .route("/show", post(show))
        .route("/list", post(list))
        .route("/push", post(push))
        .route("/pull", post(pull))
}
