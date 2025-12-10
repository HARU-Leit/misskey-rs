//! Word filter endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::services::word_filter::{CreateFilterInput, UpdateFilterInput};
use misskey_db::entities::word_filter::{self, FilterAction, FilterContext};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

// ==================== Request/Response Types ====================

/// Word filter response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordFilterResponse {
    pub id: String,
    pub user_id: String,
    pub phrase: String,
    pub is_regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub action: String,
    pub context: String,
    pub expires_at: Option<String>,
    pub match_count: i64,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl From<word_filter::Model> for WordFilterResponse {
    fn from(f: word_filter::Model) -> Self {
        Self {
            id: f.id,
            user_id: f.user_id,
            phrase: f.phrase,
            is_regex: f.is_regex,
            case_sensitive: f.case_sensitive,
            whole_word: f.whole_word,
            action: match f.action {
                FilterAction::Hide => "hide".to_string(),
                FilterAction::Warn => "warn".to_string(),
                FilterAction::ContentWarning => "cw".to_string(),
            },
            context: match f.context {
                FilterContext::Home => "home".to_string(),
                FilterContext::Notifications => "notifications".to_string(),
                FilterContext::Public => "public".to_string(),
                FilterContext::Search => "search".to_string(),
                FilterContext::All => "all".to_string(),
            },
            expires_at: f.expires_at.map(|dt| dt.to_rfc3339()),
            match_count: f.match_count,
            created_at: f.created_at.to_rfc3339(),
            updated_at: f.updated_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Filter check result response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterCheckResponse {
    pub matched: bool,
    pub matched_filter_ids: Vec<String>,
    pub action: Option<String>,
    pub matched_phrases: Vec<String>,
}

/// List filters request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFiltersRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Show filter request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowFilterRequest {
    pub filter_id: String,
}

/// Delete filter request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFilterRequest {
    pub filter_id: String,
}

/// Check content against filters request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckContentRequest {
    pub content: String,
    #[serde(default = "default_context")]
    pub context: FilterContext,
}

const fn default_limit() -> u64 {
    10
}

fn default_context() -> FilterContext {
    FilterContext::All
}

// ==================== Handlers ====================

/// Create a new word filter.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateFilterInput>,
) -> AppResult<ApiResponse<WordFilterResponse>> {
    let filter = state.word_filter_service.create(&user.id, input).await?;

    Ok(ApiResponse::ok(filter.into()))
}

/// Update a word filter.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<UpdateFilterInput>,
) -> AppResult<ApiResponse<WordFilterResponse>> {
    let filter = state.word_filter_service.update(&user.id, input).await?;

    Ok(ApiResponse::ok(filter.into()))
}

/// Delete a word filter.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteFilterRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .word_filter_service
        .delete(&req.filter_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Show a word filter.
async fn show(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowFilterRequest>,
) -> AppResult<ApiResponse<WordFilterResponse>> {
    let filter = state
        .word_filter_service
        .get_by_id_for_user(&req.filter_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(filter.into()))
}

/// List word filters.
async fn list(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListFiltersRequest>,
) -> AppResult<ApiResponse<Vec<WordFilterResponse>>> {
    let limit = req.limit.min(100);
    let filters = state
        .word_filter_service
        .list_filters(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        filters.into_iter().map(Into::into).collect(),
    ))
}

/// List active (non-expired) word filters.
async fn list_active(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<WordFilterResponse>>> {
    let filters = state
        .word_filter_service
        .list_active_filters(&user.id)
        .await?;

    Ok(ApiResponse::ok(
        filters.into_iter().map(Into::into).collect(),
    ))
}

/// Check content against user's filters.
async fn check(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CheckContentRequest>,
) -> AppResult<ApiResponse<FilterCheckResponse>> {
    let result = state
        .word_filter_service
        .apply_filters(&user.id, &req.content, req.context)
        .await?;

    Ok(ApiResponse::ok(FilterCheckResponse {
        matched: result.matched,
        matched_filter_ids: result.matched_filter_ids,
        action: result.action.map(|a| match a {
            FilterAction::Hide => "hide".to_string(),
            FilterAction::Warn => "warn".to_string(),
            FilterAction::ContentWarning => "cw".to_string(),
        }),
        matched_phrases: result.matched_phrases,
    }))
}

/// Delete all filters for the current user.
async fn delete_all(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<u64>> {
    let count = state
        .word_filter_service
        .delete_all_for_user(&user.id)
        .await?;

    Ok(ApiResponse::ok(count))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/create", post(create))
        .route("/update", post(update))
        .route("/delete", post(delete))
        .route("/show", post(show))
        .route("/list", post(list))
        .route("/list-active", post(list_active))
        .route("/check", post(check))
        .route("/delete-all", post(delete_all))
}
