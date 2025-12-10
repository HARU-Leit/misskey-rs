//! Translation endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::{
    LanguageDetectionResponse, SupportedLanguage, TranslateInput, TranslationProvider,
    TranslationResponse,
};
use serde::{Deserialize, Serialize};

use crate::{middleware::AppState, response::ApiResponse};

/// Request to translate a note.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateNoteRequest {
    /// Note ID to translate
    pub note_id: String,
    /// Target language code
    pub target_lang: String,
}

/// Request to translate text.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateTextRequest {
    /// Text to translate
    pub text: String,
    /// Target language code
    pub target_lang: String,
    /// Source language code (optional)
    pub source_lang: Option<String>,
}

/// Request to detect language.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectLanguageRequest {
    /// Text to detect language of
    pub text: String,
}

/// Translation service status response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationStatusResponse {
    /// Whether translation is available
    pub available: bool,
    /// Active provider
    pub provider: Option<TranslationProvider>,
}

/// Translate a note.
async fn translate_note(
    State(state): State<AppState>,
    Json(req): Json<TranslateNoteRequest>,
) -> AppResult<ApiResponse<TranslationResponse>> {
    // Get the note
    let note = state.note_service.get(&req.note_id).await?;

    // Get text to translate (prefer CW text + main text)
    let text = if let Some(cw) = &note.cw {
        format!("{}\n\n{}", cw, note.text.as_deref().unwrap_or(""))
    } else {
        note.text.clone().unwrap_or_default()
    };

    if text.is_empty() {
        return Err(misskey_common::AppError::BadRequest(
            "Note has no text to translate".to_string(),
        ));
    }

    // Check if translation service is available
    let translation_service = state.translation_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Translation service not configured".to_string())
    })?;

    let input = TranslateInput {
        text,
        target_lang: req.target_lang,
        source_lang: None,
    };

    let result = translation_service.translate(input).await?;
    Ok(ApiResponse::ok(result))
}

/// Translate arbitrary text.
async fn translate_text(
    State(state): State<AppState>,
    Json(req): Json<TranslateTextRequest>,
) -> AppResult<ApiResponse<TranslationResponse>> {
    // Check if translation service is available
    let translation_service = state.translation_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Translation service not configured".to_string())
    })?;

    let input = TranslateInput {
        text: req.text,
        target_lang: req.target_lang,
        source_lang: req.source_lang,
    };

    let result = translation_service.translate(input).await?;
    Ok(ApiResponse::ok(result))
}

/// Detect language of text.
async fn detect_language(
    State(state): State<AppState>,
    Json(req): Json<DetectLanguageRequest>,
) -> AppResult<ApiResponse<LanguageDetectionResponse>> {
    // Check if translation service is available
    let translation_service = state.translation_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Translation service not configured".to_string())
    })?;

    let result = translation_service.detect_language(&req.text).await?;
    Ok(ApiResponse::ok(result))
}

/// Get supported languages.
async fn supported_languages(
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<SupportedLanguage>>> {
    // Check if translation service is available
    let translation_service = state.translation_service.as_ref().ok_or_else(|| {
        misskey_common::AppError::BadRequest("Translation service not configured".to_string())
    })?;

    let result = translation_service.supported_languages().await?;
    Ok(ApiResponse::ok(result))
}

/// Get translation service status.
async fn translation_status(
    State(state): State<AppState>,
) -> AppResult<ApiResponse<TranslationStatusResponse>> {
    let response = if let Some(service) = &state.translation_service {
        TranslationStatusResponse {
            available: true,
            provider: Some(service.active_provider()),
        }
    } else {
        TranslationStatusResponse {
            available: false,
            provider: None,
        }
    };
    Ok(ApiResponse::ok(response))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/note", post(translate_note))
        .route("/text", post(translate_text))
        .route("/detect", post(detect_language))
        .route("/languages", post(supported_languages))
        .route("/status", post(translation_status))
}
