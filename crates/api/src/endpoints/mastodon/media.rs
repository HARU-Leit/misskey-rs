//! Mastodon Media API.
//!
//! Provides media upload and management endpoints for Mastodon compatibility.
//!
//! Endpoints:
//! - POST /api/v1/media - Upload media
//! - GET /api/v1/media/:id - Get media by ID
//! - PUT /api/v1/media/:id - Update media description

use axum::{
    Json, Router,
    extract::{Multipart, Path, State},
    routing::{get, post, put},
};
use misskey_common::AppResult;
use misskey_core::drive::CreateFileInput;
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState};

/// Mastodon media attachment response.
#[derive(Debug, Clone, Serialize)]
pub struct MediaAttachment {
    pub id: String,
    #[serde(rename = "type")]
    pub media_type: String,
    pub url: String,
    pub preview_url: Option<String>,
    pub remote_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_url: Option<String>,
    pub description: Option<String>,
    pub blurhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MediaMeta>,
}

/// Media metadata.
#[derive(Debug, Clone, Serialize)]
pub struct MediaMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<MediaDimensions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small: Option<MediaDimensions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<MediaFocus>,
}

/// Media dimensions.
#[derive(Debug, Clone, Serialize)]
pub struct MediaDimensions {
    pub width: i32,
    pub height: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect: Option<f64>,
}

/// Media focus point.
#[derive(Debug, Clone, Serialize)]
pub struct MediaFocus {
    pub x: f64,
    pub y: f64,
}

/// Update media request.
#[derive(Debug, Deserialize)]
pub struct UpdateMediaRequest {
    /// Alternative text description
    pub description: Option<String>,
    /// Focus point (x,y as string "-0.5,0.5")
    pub focus: Option<String>,
}

/// Convert content type to Mastodon media type.
fn content_type_to_media_type(content_type: &str) -> String {
    if content_type.starts_with("image/gif") {
        "gifv".to_string()
    } else if content_type.starts_with("image/") {
        "image".to_string()
    } else if content_type.starts_with("video/") {
        "video".to_string()
    } else if content_type.starts_with("audio/") {
        "audio".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Convert drive file to Mastodon media attachment.
fn drive_file_to_media_attachment(
    file: misskey_db::entities::drive_file::Model,
) -> MediaAttachment {
    let media_type = content_type_to_media_type(&file.content_type);

    let meta = if file.width.is_some() || file.height.is_some() {
        Some(MediaMeta {
            original: match (file.width, file.height) {
                (Some(w), Some(h)) => Some(MediaDimensions {
                    width: w,
                    height: h,
                    size: Some(format!("{}x{}", w, h)),
                    aspect: if h > 0 {
                        Some(f64::from(w) / f64::from(h))
                    } else {
                        None
                    },
                }),
                _ => None,
            },
            small: None,
            focus: None,
        })
    } else {
        None
    };

    MediaAttachment {
        id: file.id,
        media_type,
        url: file.url.clone(),
        preview_url: file.thumbnail_url.or_else(|| Some(file.url.clone())),
        remote_url: None,
        text_url: None,
        description: file.comment,
        blurhash: file.blurhash,
        meta,
    }
}

/// POST /api/v1/media - Upload a media attachment.
async fn upload_media(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<MediaAttachment>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name = String::from("upload");
    let mut content_type = String::from("application/octet-stream");
    let mut description: Option<String> = None;
    let mut focus: Option<String> = None;

    // Parse multipart form
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        misskey_common::AppError::BadRequest(format!("Invalid multipart data: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                if let Some(ct) = field.content_type() {
                    content_type = ct.to_string();
                }
                if let Some(fname) = field.file_name() {
                    file_name = fname.to_string();
                }
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            misskey_common::AppError::BadRequest(format!(
                                "Failed to read file: {}",
                                e
                            ))
                        })?
                        .to_vec(),
                );
            }
            "description" => {
                description = Some(field.text().await.unwrap_or_default());
            }
            "focus" => {
                focus = Some(field.text().await.unwrap_or_default());
            }
            _ => {}
        }
    }

    let data = file_data
        .ok_or_else(|| misskey_common::AppError::BadRequest("No file provided".to_string()))?;

    let size = data.len() as i64;

    // Parse focus point if provided (format: "x,y" where x and y are -1.0 to 1.0)
    let _focus_point = focus.as_ref().and_then(|f| {
        let parts: Vec<&str> = f.split(',').collect();
        if parts.len() == 2 {
            let x = parts[0].trim().parse::<f64>().ok()?;
            let y = parts[1].trim().parse::<f64>().ok()?;
            Some((x, y))
        } else {
            None
        }
    });

    let input = CreateFileInput {
        name: file_name,
        content_type,
        size,
        data,
        folder_id: None,
        comment: description,
        is_sensitive: false,
    };

    let file = state.drive_service.upload_file(&user.id, input).await?;
    let attachment = drive_file_to_media_attachment(file);

    Ok(Json(attachment))
}

/// GET /api/v1/media/:id - Get a media attachment.
async fn get_media(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<MediaAttachment>> {
    let file = state.drive_service.get_file(&id).await?;

    // Verify ownership
    if file.user_id != user.id {
        return Err(misskey_common::AppError::NotFound(
            "Media not found".to_string(),
        ));
    }

    let attachment = drive_file_to_media_attachment(file);
    Ok(Json(attachment))
}

/// PUT /api/v1/media/:id - Update a media attachment.
async fn update_media(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMediaRequest>,
) -> AppResult<Json<MediaAttachment>> {
    let file = state
        .drive_service
        .update_file(
            &user.id,
            &id,
            None,                      // name
            None,                      // folder_id
            None,                      // is_sensitive
            req.description.map(Some), // comment (wrapped in Some to update)
        )
        .await?;

    let attachment = drive_file_to_media_attachment(file);
    Ok(Json(attachment))
}

/// Create the media router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(upload_media))
        .route("/{id}", get(get_media).put(update_media))
}
