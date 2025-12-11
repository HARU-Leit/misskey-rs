//! Gallery endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::{CreateGalleryPostInput, GalleryPostResponse, UpdateGalleryPostInput};
use serde::Deserialize;

use crate::{
    extractors::{AuthUser, MaybeAuthUser},
    middleware::AppState,
    response::ApiResponse,
};

/// Request to get a gallery post by ID.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPostRequest {
    pub post_id: String,
}

/// Request to list gallery posts with pagination.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPostsRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

const fn default_limit() -> u64 {
    10
}

/// Request to list gallery posts for a user.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUserPostsRequest {
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Request to update a gallery post.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePostRequest {
    pub post_id: String,
    #[serde(flatten)]
    pub input: UpdateGalleryPostInput,
}

/// Request to delete a gallery post.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePostRequest {
    pub post_id: String,
}

/// Request to like/unlike a gallery post.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikePostRequest {
    pub post_id: String,
}

/// Request to list featured/popular posts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedPostsRequest {
    #[serde(default)]
    pub limit: Option<u64>,
}

/// Request to search posts by tag.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchByTagRequest {
    pub tag: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Request to get liked posts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedPostsRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Create a new gallery post.
async fn create_post(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateGalleryPostInput>,
) -> AppResult<ApiResponse<GalleryPostResponse>> {
    let post = state.gallery_service.create(&user.id, input).await?;
    Ok(ApiResponse::ok(post))
}

/// List all gallery posts.
async fn list_posts(
    State(state): State<AppState>,
    Json(req): Json<ListPostsRequest>,
) -> AppResult<ApiResponse<Vec<GalleryPostResponse>>> {
    let posts = state.gallery_service.list(req.limit, req.offset).await?;
    Ok(ApiResponse::ok(posts))
}

/// List gallery posts for a user.
async fn list_user_posts(
    State(state): State<AppState>,
    Json(req): Json<ListUserPostsRequest>,
) -> AppResult<ApiResponse<Vec<GalleryPostResponse>>> {
    let posts = state
        .gallery_service
        .list_by_user(&req.user_id, req.limit, req.offset)
        .await?;
    Ok(ApiResponse::ok(posts))
}

/// List gallery posts for the current user.
async fn list_my_posts(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListPostsRequest>,
) -> AppResult<ApiResponse<Vec<GalleryPostResponse>>> {
    let posts = state
        .gallery_service
        .list_by_user(&user.id, req.limit, req.offset)
        .await?;
    Ok(ApiResponse::ok(posts))
}

/// Get a gallery post by ID.
async fn get_post(
    MaybeAuthUser(user): MaybeAuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetPostRequest>,
) -> AppResult<ApiResponse<GalleryPostResponse>> {
    let viewer_id = user.as_ref().map(|u| u.id.as_str());
    let post = state.gallery_service.get(&req.post_id, viewer_id).await?;
    Ok(ApiResponse::ok(post))
}

/// Update a gallery post.
async fn update_post(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdatePostRequest>,
) -> AppResult<ApiResponse<GalleryPostResponse>> {
    let post = state
        .gallery_service
        .update(&user.id, &req.post_id, req.input)
        .await?;
    Ok(ApiResponse::ok(post))
}

/// Delete a gallery post.
async fn delete_post(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeletePostRequest>,
) -> AppResult<ApiResponse<()>> {
    state.gallery_service.delete(&user.id, &req.post_id).await?;
    Ok(ApiResponse::ok(()))
}

/// List featured gallery posts.
async fn featured_posts(
    State(state): State<AppState>,
    Json(req): Json<FeaturedPostsRequest>,
) -> AppResult<ApiResponse<Vec<GalleryPostResponse>>> {
    let posts = state.gallery_service.list_featured(req.limit).await?;
    Ok(ApiResponse::ok(posts))
}

/// List popular gallery posts.
async fn popular_posts(
    State(state): State<AppState>,
    Json(req): Json<FeaturedPostsRequest>,
) -> AppResult<ApiResponse<Vec<GalleryPostResponse>>> {
    let posts = state.gallery_service.list_popular(req.limit).await?;
    Ok(ApiResponse::ok(posts))
}

/// Search gallery posts by tag.
async fn search_by_tag(
    State(state): State<AppState>,
    Json(req): Json<SearchByTagRequest>,
) -> AppResult<ApiResponse<Vec<GalleryPostResponse>>> {
    let posts = state
        .gallery_service
        .search_by_tag(&req.tag, req.limit, req.offset)
        .await?;
    Ok(ApiResponse::ok(posts))
}

/// Like a gallery post.
async fn like_post(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<LikePostRequest>,
) -> AppResult<ApiResponse<()>> {
    state.gallery_service.like(&user.id, &req.post_id).await?;
    Ok(ApiResponse::ok(()))
}

/// Unlike a gallery post.
async fn unlike_post(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<LikePostRequest>,
) -> AppResult<ApiResponse<()>> {
    state.gallery_service.unlike(&user.id, &req.post_id).await?;
    Ok(ApiResponse::ok(()))
}

/// List liked gallery posts.
async fn liked_posts(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<LikedPostsRequest>,
) -> AppResult<ApiResponse<Vec<GalleryPostResponse>>> {
    let posts = state
        .gallery_service
        .liked_posts(&user.id, req.limit, req.offset)
        .await?;
    Ok(ApiResponse::ok(posts))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/posts/create", post(create_post))
        .route("/posts", post(list_posts))
        .route("/posts/show", post(get_post))
        .route("/posts/update", post(update_post))
        .route("/posts/delete", post(delete_post))
        .route("/posts/like", post(like_post))
        .route("/posts/unlike", post(unlike_post))
        .route("/posts/liked", post(liked_posts))
        .route("/posts/user", post(list_user_posts))
        .route("/posts/mine", post(list_my_posts))
        .route("/featured", post(featured_posts))
        .route("/popular", post(popular_posts))
        .route("/search/tag", post(search_by_tag))
}
