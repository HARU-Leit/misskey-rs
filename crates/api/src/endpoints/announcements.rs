//! Announcement endpoints.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use misskey_common::AppResult;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    extractors::{AuthUser, MaybeAuthUser},
    middleware::AppState,
    response::ApiResponse,
};

/// Create announcement router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_announcements))
        .route("/", post(create_announcement))
        .route("/unread", get(list_unread_announcements))
        .route("/{id}", get(get_announcement))
        .route("/{id}", put(update_announcement))
        .route("/{id}", delete(delete_announcement))
        .route("/{id}/read", post(mark_as_read))
}

/// Announcement response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementResponse {
    pub id: String,
    pub title: String,
    pub text: String,
    pub image_url: Option<String>,
    pub is_active: bool,
    pub needs_confirmation_to_read: bool,
    pub display_order: i32,
    pub icon: Option<String>,
    pub foreground_color: Option<String>,
    pub background_color: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub reads_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_read: Option<bool>,
}

impl From<misskey_db::entities::announcement::Model> for AnnouncementResponse {
    fn from(announcement: misskey_db::entities::announcement::Model) -> Self {
        Self {
            id: announcement.id,
            title: announcement.title,
            text: announcement.text,
            image_url: announcement.image_url,
            is_active: announcement.is_active,
            needs_confirmation_to_read: announcement.needs_confirmation_to_read,
            display_order: announcement.display_order,
            icon: announcement.icon,
            foreground_color: announcement.foreground_color,
            background_color: announcement.background_color,
            starts_at: announcement.starts_at,
            ends_at: announcement.ends_at,
            reads_count: announcement.reads_count,
            created_at: announcement.created_at,
            updated_at: announcement.updated_at,
            is_read: None,
        }
    }
}

/// List announcements response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnouncementListResponse {
    pub announcements: Vec<AnnouncementResponse>,
    pub total: u64,
}

/// List announcements query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAnnouncementsQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
    /// If true, only list active announcements (default for non-admin users)
    #[serde(default)]
    pub active_only: bool,
}

const fn default_limit() -> u64 {
    10
}

/// List announcements.
async fn list_announcements(
    MaybeAuthUser(user): MaybeAuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListAnnouncementsQuery>,
) -> AppResult<ApiResponse<AnnouncementListResponse>> {
    let is_admin = user.as_ref().is_some_and(|u| u.is_admin || u.is_moderator);

    // Non-admins can only see active announcements
    let announcements = if is_admin && !query.active_only {
        state
            .announcement_service
            .list_all(query.limit, query.offset)
            .await?
    } else {
        state.announcement_service.list_active().await?
    };

    let total = if is_admin && !query.active_only {
        state.announcement_service.count().await?
    } else {
        announcements.len() as u64
    };

    // If user is authenticated, check read status for each announcement
    let mut responses: Vec<AnnouncementResponse> = Vec::with_capacity(announcements.len());
    for ann in announcements {
        let mut response = AnnouncementResponse::from(ann);
        if let Some(ref auth_user) = user {
            response.is_read = Some(
                state
                    .announcement_service
                    .has_read(&auth_user.id, &response.id)
                    .await?,
            );
        }
        responses.push(response);
    }

    Ok(ApiResponse::ok(AnnouncementListResponse {
        announcements: responses,
        total,
    }))
}

/// List unread announcements for the authenticated user.
async fn list_unread_announcements(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<AnnouncementListResponse>> {
    let announcements = state
        .announcement_service
        .get_unread_for_user(&user.id)
        .await?;
    let total = announcements.len() as u64;

    let responses: Vec<AnnouncementResponse> = announcements
        .into_iter()
        .map(|ann| {
            let mut response = AnnouncementResponse::from(ann);
            response.is_read = Some(false);
            response
        })
        .collect();

    Ok(ApiResponse::ok(AnnouncementListResponse {
        announcements: responses,
        total,
    }))
}

/// Get a single announcement.
async fn get_announcement(
    MaybeAuthUser(user): MaybeAuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<ApiResponse<AnnouncementResponse>> {
    let announcement = state
        .announcement_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| {
            misskey_common::AppError::NotFound(format!("Announcement not found: {id}"))
        })?;

    let mut response = AnnouncementResponse::from(announcement);

    // Check if user has read this announcement
    if let Some(auth_user) = user {
        response.is_read = Some(
            state
                .announcement_service
                .has_read(&auth_user.id, &id)
                .await?,
        );
    }

    Ok(ApiResponse::ok(response))
}

/// Create announcement request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnouncementRequest {
    pub title: String,
    pub text: String,
    pub image_url: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub needs_confirmation_to_read: bool,
    #[serde(default)]
    pub display_order: i32,
    pub icon: Option<String>,
    pub foreground_color: Option<String>,
    pub background_color: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

const fn default_true() -> bool {
    true
}

/// Create announcement (admin only).
async fn create_announcement(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateAnnouncementRequest>,
) -> AppResult<ApiResponse<AnnouncementResponse>> {
    // Check if user is admin or moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only admins and moderators can create announcements".to_string(),
        ));
    }

    info!(user_id = %user.id, title = %req.title, "Creating announcement");

    let announcement = state
        .announcement_service
        .create(
            req.title,
            req.text,
            req.image_url,
            req.is_active,
            req.needs_confirmation_to_read,
            req.display_order,
            req.icon,
            req.foreground_color,
            req.background_color,
            req.starts_at,
            req.ends_at,
        )
        .await?;

    Ok(ApiResponse::ok(AnnouncementResponse::from(announcement)))
}

/// Update announcement request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub text: Option<String>,
    pub image_url: Option<Option<String>>,
    pub is_active: Option<bool>,
    pub needs_confirmation_to_read: Option<bool>,
    pub display_order: Option<i32>,
    pub icon: Option<Option<String>>,
    pub foreground_color: Option<Option<String>>,
    pub background_color: Option<Option<String>>,
    pub starts_at: Option<Option<DateTime<Utc>>>,
    pub ends_at: Option<Option<DateTime<Utc>>>,
}

/// Update announcement (admin only).
async fn update_announcement(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAnnouncementRequest>,
) -> AppResult<ApiResponse<AnnouncementResponse>> {
    // Check if user is admin or moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only admins and moderators can update announcements".to_string(),
        ));
    }

    info!(user_id = %user.id, announcement_id = %id, "Updating announcement");

    let announcement = state
        .announcement_service
        .update(
            &id,
            req.title,
            req.text,
            req.image_url,
            req.is_active,
            req.needs_confirmation_to_read,
            req.display_order,
            req.icon,
            req.foreground_color,
            req.background_color,
            req.starts_at,
            req.ends_at,
        )
        .await?;

    Ok(ApiResponse::ok(AnnouncementResponse::from(announcement)))
}

/// Delete announcement (admin only).
async fn delete_announcement(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<ApiResponse<()>> {
    // Check if user is admin or moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only admins and moderators can delete announcements".to_string(),
        ));
    }

    info!(user_id = %user.id, announcement_id = %id, "Deleting announcement");

    state.announcement_service.delete(&id).await?;

    Ok(ApiResponse::ok(()))
}

/// Mark announcement as read.
async fn mark_as_read(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<ApiResponse<()>> {
    info!(user_id = %user.id, announcement_id = %id, "Marking announcement as read");

    state
        .announcement_service
        .mark_as_read(&user.id, &id)
        .await?;

    Ok(ApiResponse::ok(()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_announcement_response_serialization() {
        let response = AnnouncementResponse {
            id: "123".to_string(),
            title: "Test Announcement".to_string(),
            text: "This is a test announcement".to_string(),
            image_url: None,
            is_active: true,
            needs_confirmation_to_read: false,
            display_order: 0,
            icon: None,
            foreground_color: None,
            background_color: None,
            starts_at: None,
            ends_at: None,
            reads_count: 0,
            created_at: Utc::now(),
            updated_at: None,
            is_read: Some(false),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"title\":\"Test Announcement\""));
        assert!(json.contains("\"isActive\":true"));
    }
}
