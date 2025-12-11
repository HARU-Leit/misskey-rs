//! Translation service for translating notes.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use misskey_common::{AppError, AppResult};

/// Supported translation providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationProvider {
    /// `DeepL` API
    DeepL,
    /// Google Translate API
    Google,
    /// `LibreTranslate` (self-hosted)
    LibreTranslate,
    /// `OpenAI` API (GPT-based translation)
    OpenAI,
    /// Anthropic Claude API
    Anthropic,
    /// Local LLM via Ollama
    Ollama,
}

/// Configuration for translation service.
#[derive(Debug, Clone)]
pub struct TranslationConfig {
    /// Active provider
    pub provider: TranslationProvider,
    /// `DeepL` API key
    pub deepl_api_key: Option<String>,
    /// Google Translate API key
    pub google_api_key: Option<String>,
    /// `LibreTranslate` URL
    pub libretranslate_url: Option<String>,
    /// `LibreTranslate` API key (optional)
    pub libretranslate_api_key: Option<String>,
    /// `OpenAI` API key
    pub openai_api_key: Option<String>,
    /// `OpenAI` model (e.g., "gpt-4o-mini")
    pub openai_model: Option<String>,
    /// Anthropic API key
    pub anthropic_api_key: Option<String>,
    /// Anthropic model (e.g., "claude-3-haiku-20240307")
    pub anthropic_model: Option<String>,
    /// Ollama URL
    pub ollama_url: Option<String>,
    /// Ollama model
    pub ollama_model: Option<String>,
    /// Enable caching
    pub cache_enabled: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            provider: TranslationProvider::LibreTranslate,
            deepl_api_key: None,
            google_api_key: None,
            libretranslate_url: Some("http://localhost:5000".to_string()),
            libretranslate_api_key: None,
            openai_api_key: None,
            openai_model: Some("gpt-4o-mini".to_string()),
            anthropic_api_key: None,
            anthropic_model: Some("claude-3-haiku-20240307".to_string()),
            ollama_url: Some("http://localhost:11434".to_string()),
            ollama_model: Some("llama3.2".to_string()),
            cache_enabled: true,
            cache_ttl_seconds: 3600,
        }
    }
}

/// Translation request input.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateInput {
    /// Text to translate
    pub text: String,
    /// Target language code (e.g., "en", "ja", "de")
    pub target_lang: String,
    /// Source language code (optional, auto-detect if not provided)
    pub source_lang: Option<String>,
}

/// Translation response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResponse {
    /// Translated text
    pub text: String,
    /// Detected source language
    pub source_lang: String,
    /// Target language
    pub target_lang: String,
    /// Provider used
    pub provider: TranslationProvider,
}

/// Detected language response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDetectionResponse {
    /// Detected language code
    pub language: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

/// Supported language info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedLanguage {
    /// Language code
    pub code: String,
    /// Language name
    pub name: String,
    /// Native name
    pub native_name: String,
}

/// Translation provider trait.
#[async_trait]
pub trait TranslationProviderTrait: Send + Sync {
    /// Translate text.
    async fn translate(
        &self,
        text: &str,
        target_lang: &str,
        source_lang: Option<&str>,
    ) -> AppResult<TranslationResponse>;

    /// Detect language of text.
    async fn detect_language(&self, text: &str) -> AppResult<LanguageDetectionResponse>;

    /// Get supported languages.
    async fn supported_languages(&self) -> AppResult<Vec<SupportedLanguage>>;
}

/// Cache entry for translations.
#[derive(Debug, Clone)]
struct CacheEntry {
    response: TranslationResponse,
    expires_at: std::time::Instant,
}

/// Translation service.
#[derive(Clone)]
pub struct TranslationService {
    config: TranslationConfig,
    http_client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl TranslationService {
    /// Create a new translation service.
    #[must_use]
    pub fn new(config: TranslationConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a cache key from request parameters.
    fn cache_key(text: &str, target_lang: &str, source_lang: Option<&str>) -> String {
        format!("{}:{}:{}", source_lang.unwrap_or("auto"), target_lang, text)
    }

    /// Check cache for translation.
    async fn check_cache(&self, key: &str) -> Option<TranslationResponse> {
        if !self.config.cache_enabled {
            return None;
        }

        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key)
            && entry.expires_at > std::time::Instant::now()
        {
            return Some(entry.response.clone());
        }
        None
    }

    /// Store translation in cache.
    async fn store_cache(&self, key: String, response: TranslationResponse) {
        if !self.config.cache_enabled {
            return;
        }

        let entry = CacheEntry {
            response,
            expires_at: std::time::Instant::now()
                + std::time::Duration::from_secs(self.config.cache_ttl_seconds),
        };

        let mut cache = self.cache.write().await;
        cache.insert(key, entry);

        // Clean up expired entries occasionally
        if cache.len() > 1000 {
            let now = std::time::Instant::now();
            cache.retain(|_, v| v.expires_at > now);
        }
    }

    /// Translate text.
    pub async fn translate(&self, input: TranslateInput) -> AppResult<TranslationResponse> {
        let cache_key = Self::cache_key(
            &input.text,
            &input.target_lang,
            input.source_lang.as_deref(),
        );

        // Check cache first
        if let Some(cached) = self.check_cache(&cache_key).await {
            return Ok(cached);
        }

        // Perform translation based on provider
        let response = match self.config.provider {
            TranslationProvider::DeepL => {
                self.translate_deepl(
                    &input.text,
                    &input.target_lang,
                    input.source_lang.as_deref(),
                )
                .await?
            }
            TranslationProvider::Google => {
                self.translate_google(
                    &input.text,
                    &input.target_lang,
                    input.source_lang.as_deref(),
                )
                .await?
            }
            TranslationProvider::LibreTranslate => {
                self.translate_libretranslate(
                    &input.text,
                    &input.target_lang,
                    input.source_lang.as_deref(),
                )
                .await?
            }
            TranslationProvider::OpenAI => {
                self.translate_openai(
                    &input.text,
                    &input.target_lang,
                    input.source_lang.as_deref(),
                )
                .await?
            }
            TranslationProvider::Anthropic => {
                self.translate_anthropic(
                    &input.text,
                    &input.target_lang,
                    input.source_lang.as_deref(),
                )
                .await?
            }
            TranslationProvider::Ollama => {
                self.translate_ollama(
                    &input.text,
                    &input.target_lang,
                    input.source_lang.as_deref(),
                )
                .await?
            }
        };

        // Store in cache
        self.store_cache(cache_key, response.clone()).await;

        Ok(response)
    }

    /// Detect language.
    pub async fn detect_language(&self, text: &str) -> AppResult<LanguageDetectionResponse> {
        match self.config.provider {
            TranslationProvider::DeepL => self.detect_language_deepl(text).await,
            TranslationProvider::LibreTranslate => self.detect_language_libretranslate(text).await,
            _ => {
                // Use simple heuristics for providers that don't support detection
                self.detect_language_heuristic(text)
            }
        }
    }

    /// Get supported languages.
    pub async fn supported_languages(&self) -> AppResult<Vec<SupportedLanguage>> {
        // Return common languages supported by most providers
        Ok(vec![
            SupportedLanguage {
                code: "en".to_string(),
                name: "English".to_string(),
                native_name: "English".to_string(),
            },
            SupportedLanguage {
                code: "ja".to_string(),
                name: "Japanese".to_string(),
                native_name: "日本語".to_string(),
            },
            SupportedLanguage {
                code: "zh".to_string(),
                name: "Chinese".to_string(),
                native_name: "中文".to_string(),
            },
            SupportedLanguage {
                code: "ko".to_string(),
                name: "Korean".to_string(),
                native_name: "한국어".to_string(),
            },
            SupportedLanguage {
                code: "de".to_string(),
                name: "German".to_string(),
                native_name: "Deutsch".to_string(),
            },
            SupportedLanguage {
                code: "fr".to_string(),
                name: "French".to_string(),
                native_name: "Français".to_string(),
            },
            SupportedLanguage {
                code: "es".to_string(),
                name: "Spanish".to_string(),
                native_name: "Español".to_string(),
            },
            SupportedLanguage {
                code: "it".to_string(),
                name: "Italian".to_string(),
                native_name: "Italiano".to_string(),
            },
            SupportedLanguage {
                code: "pt".to_string(),
                name: "Portuguese".to_string(),
                native_name: "Português".to_string(),
            },
            SupportedLanguage {
                code: "ru".to_string(),
                name: "Russian".to_string(),
                native_name: "Русский".to_string(),
            },
            SupportedLanguage {
                code: "ar".to_string(),
                name: "Arabic".to_string(),
                native_name: "العربية".to_string(),
            },
            SupportedLanguage {
                code: "hi".to_string(),
                name: "Hindi".to_string(),
                native_name: "हिन्दी".to_string(),
            },
        ])
    }

    /// Get active provider.
    #[must_use]
    pub const fn active_provider(&self) -> TranslationProvider {
        self.config.provider
    }

    // Provider-specific implementations

    async fn translate_deepl(
        &self,
        text: &str,
        target_lang: &str,
        source_lang: Option<&str>,
    ) -> AppResult<TranslationResponse> {
        let api_key = self
            .config
            .deepl_api_key
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("DeepL API key not configured".to_string()))?;

        let mut params = vec![
            ("text", text.to_string()),
            ("target_lang", target_lang.to_uppercase()),
        ];
        if let Some(src) = source_lang {
            params.push(("source_lang", src.to_uppercase()));
        }

        let response = self
            .http_client
            .post("https://api-free.deepl.com/v2/translate")
            .header("Authorization", format!("DeepL-Auth-Key {api_key}"))
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("DeepL request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "DeepL API error: {status} - {body}"
            )));
        }

        #[derive(Deserialize)]
        struct DeepLResponse {
            translations: Vec<DeepLTranslation>,
        }

        #[derive(Deserialize)]
        struct DeepLTranslation {
            text: String,
            detected_source_language: String,
        }

        let deepl_response: DeepLResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse DeepL response: {e}"))
        })?;

        let translation = deepl_response
            .translations
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ExternalService("No translation returned".to_string()))?;

        Ok(TranslationResponse {
            text: translation.text,
            source_lang: translation.detected_source_language.to_lowercase(),
            target_lang: target_lang.to_lowercase(),
            provider: TranslationProvider::DeepL,
        })
    }

    async fn translate_google(
        &self,
        text: &str,
        target_lang: &str,
        source_lang: Option<&str>,
    ) -> AppResult<TranslationResponse> {
        let api_key = self.config.google_api_key.as_ref().ok_or_else(|| {
            AppError::BadRequest("Google Translate API key not configured".to_string())
        })?;

        let mut url =
            format!("https://translation.googleapis.com/language/translate/v2?key={api_key}");
        // URL encode the text using percent-encoding
        let encoded_text: String = text
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect();
        url.push_str(&format!("&q={encoded_text}"));
        url.push_str(&format!("&target={target_lang}"));
        if let Some(src) = source_lang {
            url.push_str(&format!("&source={src}"));
        }

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            AppError::ExternalService(format!("Google Translate request failed: {e}"))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Google Translate API error: {status} - {body}"
            )));
        }

        #[derive(Deserialize)]
        struct GoogleResponse {
            data: GoogleData,
        }

        #[derive(Deserialize)]
        struct GoogleData {
            translations: Vec<GoogleTranslation>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GoogleTranslation {
            translated_text: String,
            detected_source_language: Option<String>,
        }

        let google_response: GoogleResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse Google response: {e}"))
        })?;

        let translation = google_response
            .data
            .translations
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ExternalService("No translation returned".to_string()))?;

        Ok(TranslationResponse {
            text: translation.translated_text,
            source_lang: translation
                .detected_source_language
                .unwrap_or_else(|| source_lang.unwrap_or("auto").to_string()),
            target_lang: target_lang.to_string(),
            provider: TranslationProvider::Google,
        })
    }

    async fn translate_libretranslate(
        &self,
        text: &str,
        target_lang: &str,
        source_lang: Option<&str>,
    ) -> AppResult<TranslationResponse> {
        let url =
            self.config.libretranslate_url.as_ref().ok_or_else(|| {
                AppError::BadRequest("LibreTranslate URL not configured".to_string())
            })?;

        let mut body = serde_json::json!({
            "q": text,
            "source": source_lang.unwrap_or("auto"),
            "target": target_lang,
        });

        if let Some(api_key) = &self.config.libretranslate_api_key {
            body["api_key"] = serde_json::Value::String(api_key.clone());
        }

        let response = self
            .http_client
            .post(format!("{url}/translate"))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("LibreTranslate request failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "LibreTranslate API error: {status} - {body}"
            )));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct LibreTranslateResponse {
            translated_text: String,
            detected_language: Option<DetectedLang>,
        }

        #[derive(Deserialize)]
        struct DetectedLang {
            language: String,
        }

        let libre_response: LibreTranslateResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse LibreTranslate response: {e}"))
        })?;

        Ok(TranslationResponse {
            text: libre_response.translated_text,
            source_lang: libre_response
                .detected_language
                .map_or_else(|| source_lang.unwrap_or("auto").to_string(), |d| d.language),
            target_lang: target_lang.to_string(),
            provider: TranslationProvider::LibreTranslate,
        })
    }

    async fn translate_openai(
        &self,
        text: &str,
        target_lang: &str,
        _source_lang: Option<&str>,
    ) -> AppResult<TranslationResponse> {
        let api_key = self
            .config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("OpenAI API key not configured".to_string()))?;

        let model = self.config.openai_model.as_deref().unwrap_or("gpt-4o-mini");

        let lang_name = self.language_code_to_name(target_lang);
        let prompt = format!(
            "Translate the following text to {lang_name}. Output only the translated text without any explanation or additional content:\n\n{text}"
        );

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.3,
        });

        let response = self
            .http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenAI request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenAI API error: {status} - {body}"
            )));
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
        }

        #[derive(Deserialize)]
        struct OpenAIChoice {
            message: OpenAIMessage,
        }

        #[derive(Deserialize)]
        struct OpenAIMessage {
            content: String,
        }

        let openai_response: OpenAIResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse OpenAI response: {e}"))
        })?;

        let translated = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ExternalService("No translation returned".to_string()))?
            .message
            .content
            .trim()
            .to_string();

        Ok(TranslationResponse {
            text: translated,
            source_lang: "auto".to_string(),
            target_lang: target_lang.to_string(),
            provider: TranslationProvider::OpenAI,
        })
    }

    async fn translate_anthropic(
        &self,
        text: &str,
        target_lang: &str,
        _source_lang: Option<&str>,
    ) -> AppResult<TranslationResponse> {
        let api_key =
            self.config.anthropic_api_key.as_ref().ok_or_else(|| {
                AppError::BadRequest("Anthropic API key not configured".to_string())
            })?;

        let model = self
            .config
            .anthropic_model
            .as_deref()
            .unwrap_or("claude-3-haiku-20240307");

        let lang_name = self.language_code_to_name(target_lang);
        let prompt = format!(
            "Translate the following text to {lang_name}. Output only the translated text without any explanation or additional content:\n\n{text}"
        );

        let body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": [
                {"role": "user", "content": prompt}
            ],
        });

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Anthropic request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Anthropic API error: {status} - {body}"
            )));
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContent>,
        }

        #[derive(Deserialize)]
        struct AnthropicContent {
            text: String,
        }

        let anthropic_response: AnthropicResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse Anthropic response: {e}"))
        })?;

        let translated = anthropic_response
            .content
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ExternalService("No translation returned".to_string()))?
            .text
            .trim()
            .to_string();

        Ok(TranslationResponse {
            text: translated,
            source_lang: "auto".to_string(),
            target_lang: target_lang.to_string(),
            provider: TranslationProvider::Anthropic,
        })
    }

    async fn translate_ollama(
        &self,
        text: &str,
        target_lang: &str,
        _source_lang: Option<&str>,
    ) -> AppResult<TranslationResponse> {
        let url = self
            .config
            .ollama_url
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("Ollama URL not configured".to_string()))?;

        let model = self.config.ollama_model.as_deref().unwrap_or("llama3.2");

        let lang_name = self.language_code_to_name(target_lang);
        let prompt = format!(
            "Translate the following text to {lang_name}. Output only the translated text without any explanation:\n\n{text}"
        );

        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        });

        let response = self
            .http_client
            .post(format!("{url}/api/generate"))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Ollama request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Ollama API error: {status} - {body}"
            )));
        }

        #[derive(Deserialize)]
        struct OllamaResponse {
            response: String,
        }

        let ollama_response: OllamaResponse = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse Ollama response: {e}"))
        })?;

        Ok(TranslationResponse {
            text: ollama_response.response.trim().to_string(),
            source_lang: "auto".to_string(),
            target_lang: target_lang.to_string(),
            provider: TranslationProvider::Ollama,
        })
    }

    async fn detect_language_deepl(&self, text: &str) -> AppResult<LanguageDetectionResponse> {
        // DeepL auto-detects during translation, so we translate to English to detect
        let response = self.translate_deepl(text, "en", None).await?;
        Ok(LanguageDetectionResponse {
            language: response.source_lang,
            confidence: 0.9, // DeepL doesn't return confidence, assume high
        })
    }

    async fn detect_language_libretranslate(
        &self,
        text: &str,
    ) -> AppResult<LanguageDetectionResponse> {
        let url =
            self.config.libretranslate_url.as_ref().ok_or_else(|| {
                AppError::BadRequest("LibreTranslate URL not configured".to_string())
            })?;

        let mut body = serde_json::json!({
            "q": text,
        });

        if let Some(api_key) = &self.config.libretranslate_api_key {
            body["api_key"] = serde_json::Value::String(api_key.clone());
        }

        let response = self
            .http_client
            .post(format!("{url}/detect"))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("LibreTranslate detect request failed: {e}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "LibreTranslate detect API error: {status} - {body}"
            )));
        }

        #[derive(Deserialize)]
        struct DetectResponse {
            language: String,
            confidence: f64,
        }

        let results: Vec<DetectResponse> = response.json().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to parse detect response: {e}"))
        })?;

        let best = results
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ExternalService("No language detected".to_string()))?;

        Ok(LanguageDetectionResponse {
            language: best.language,
            confidence: best.confidence,
        })
    }

    #[allow(clippy::unwrap_used)] // counts array is never empty, max_by_key always returns Some
    fn detect_language_heuristic(&self, text: &str) -> AppResult<LanguageDetectionResponse> {
        // Simple heuristic based on character ranges
        let mut ja_count = 0;
        let mut ko_count = 0;
        let mut zh_count = 0;
        let mut latin_count = 0;
        let mut cyrillic_count = 0;
        let mut arabic_count = 0;

        for c in text.chars() {
            match c {
                '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' => ja_count += 1, // Hiragana, Katakana
                '\u{AC00}'..='\u{D7AF}' | '\u{1100}'..='\u{11FF}' => ko_count += 1, // Korean
                '\u{4E00}'..='\u{9FFF}' => zh_count += 1, // CJK (counted for Chinese, but could be Japanese kanji)
                'A'..='Z' | 'a'..='z' => latin_count += 1,
                '\u{0400}'..='\u{04FF}' => cyrillic_count += 1,
                '\u{0600}'..='\u{06FF}' => arabic_count += 1,
                _ => {}
            }
        }

        let counts = [
            (ja_count, "ja"),
            (ko_count, "ko"),
            (zh_count, "zh"),
            (latin_count, "en"),
            (cyrillic_count, "ru"),
            (arabic_count, "ar"),
        ];

        let total: usize = counts.iter().map(|(c, _)| c).sum();
        if total == 0 {
            return Ok(LanguageDetectionResponse {
                language: "en".to_string(),
                confidence: 0.1,
            });
        }

        // counts is a fixed-size array, so this should always succeed
        let Some((max_count, lang)) = counts.iter().max_by_key(|(c, _)| c) else {
            return Ok(LanguageDetectionResponse {
                language: "en".to_string(),
                confidence: 0.1,
            });
        };

        Ok(LanguageDetectionResponse {
            language: (*lang).to_string(),
            confidence: *max_count as f64 / total as f64,
        })
    }

    fn language_code_to_name(&self, code: &str) -> &'static str {
        match code.to_lowercase().as_str() {
            "en" => "English",
            "ja" => "Japanese",
            "zh" | "zh-cn" | "zh-tw" => "Chinese",
            "ko" => "Korean",
            "de" => "German",
            "fr" => "French",
            "es" => "Spanish",
            "it" => "Italian",
            "pt" => "Portuguese",
            "ru" => "Russian",
            "ar" => "Arabic",
            "hi" => "Hindi",
            "th" => "Thai",
            "vi" => "Vietnamese",
            "id" => "Indonesian",
            "nl" => "Dutch",
            "pl" => "Polish",
            "tr" => "Turkish",
            "uk" => "Ukrainian",
            _ => "the target language",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection_heuristic() {
        let config = TranslationConfig::default();
        let service = TranslationService::new(config);

        // Japanese
        let result = service.detect_language_heuristic("こんにちは世界").unwrap();
        assert_eq!(result.language, "ja");

        // English
        let result = service.detect_language_heuristic("Hello World").unwrap();
        assert_eq!(result.language, "en");

        // Korean
        let result = service.detect_language_heuristic("안녕하세요").unwrap();
        assert_eq!(result.language, "ko");
    }

    #[test]
    fn test_cache_key() {
        let key = TranslationService::cache_key("hello", "ja", Some("en"));
        assert_eq!(key, "en:ja:hello");

        let key = TranslationService::cache_key("hello", "ja", None);
        assert_eq!(key, "auto:ja:hello");
    }
}
