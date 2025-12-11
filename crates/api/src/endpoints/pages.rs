//! Page endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::{CreatePageInput, PageResponse, UpdatePageInput};
use serde::Deserialize;

use crate::{
    extractors::{AuthUser, MaybeAuthUser},
    middleware::AppState,
    response::ApiResponse,
};

/// Request to get a page by ID.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPageRequest {
    pub page_id: String,
}

/// Request to get a page by username and name.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPageByNameRequest {
    pub username: String,
    pub name: String,
}

/// Request to update a page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePageRequest {
    pub page_id: String,
    #[serde(flatten)]
    pub input: UpdatePageInput,
}

/// Request to delete a page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePageRequest {
    pub page_id: String,
}

/// Request to like a page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikePageRequest {
    pub page_id: String,
}

/// Request to unlike a page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlikePageRequest {
    pub page_id: String,
}

/// Request to list featured pages.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedPagesRequest {
    #[serde(default)]
    pub limit: Option<u64>,
}

/// Create a new page.
async fn create_page(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreatePageInput>,
) -> AppResult<ApiResponse<PageResponse>> {
    let page = state.page_service.create(&user.id, input).await?;
    Ok(ApiResponse::ok(page))
}

/// List pages for the current user.
async fn list_my_pages(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<PageResponse>>> {
    let pages = state.page_service.list_by_user(&user.id).await?;
    Ok(ApiResponse::ok(pages))
}

/// Get a page by ID.
async fn get_page(
    MaybeAuthUser(user): MaybeAuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetPageRequest>,
) -> AppResult<ApiResponse<PageResponse>> {
    let viewer_id = user.as_ref().map(|u| u.id.as_str());
    let page = state.page_service.get(&req.page_id, viewer_id).await?;
    Ok(ApiResponse::ok(page))
}

/// Get a page by username and name.
async fn get_page_by_name(
    MaybeAuthUser(user): MaybeAuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetPageByNameRequest>,
) -> AppResult<ApiResponse<PageResponse>> {
    // Look up user by username
    let page_user = state
        .user_service
        .find_local_by_username(&req.username)
        .await?
        .ok_or_else(|| misskey_common::AppError::NotFound(format!("User: {}", req.username)))?;

    let viewer_id = user.as_ref().map(|u| u.id.as_str());
    let page = state
        .page_service
        .get_by_name(&page_user.id, &req.name, viewer_id)
        .await?;
    Ok(ApiResponse::ok(page))
}

/// Update a page.
async fn update_page(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdatePageRequest>,
) -> AppResult<ApiResponse<PageResponse>> {
    let page = state
        .page_service
        .update(&user.id, &req.page_id, req.input)
        .await?;
    Ok(ApiResponse::ok(page))
}

/// Delete a page.
async fn delete_page(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeletePageRequest>,
) -> AppResult<ApiResponse<()>> {
    state.page_service.delete(&user.id, &req.page_id).await?;
    Ok(ApiResponse::ok(()))
}

/// Like a page.
async fn like_page(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<LikePageRequest>,
) -> AppResult<ApiResponse<()>> {
    state.page_service.like(&user.id, &req.page_id).await?;
    Ok(ApiResponse::ok(()))
}

/// Unlike a page.
async fn unlike_page(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UnlikePageRequest>,
) -> AppResult<ApiResponse<()>> {
    state.page_service.unlike(&user.id, &req.page_id).await?;
    Ok(ApiResponse::ok(()))
}

/// List featured pages.
async fn featured_pages(
    State(state): State<AppState>,
    Json(req): Json<FeaturedPagesRequest>,
) -> AppResult<ApiResponse<Vec<PageResponse>>> {
    let pages = state.page_service.list_featured(req.limit).await?;
    Ok(ApiResponse::ok(pages))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create_page))
        .route("/mine", post(list_my_pages))
        .route("/show", post(get_page))
        .route("/show-by-name", post(get_page_by_name))
        .route("/update", post(update_page))
        .route("/delete", post(delete_page))
        .route("/like", post(like_page))
        .route("/unlike", post(unlike_page))
        .route("/featured", post(featured_pages))
}
