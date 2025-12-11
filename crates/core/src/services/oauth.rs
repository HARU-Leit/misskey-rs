//! OAuth 2.0 service for application authentication.
//!
//! Implements OAuth 2.0 Authorization Code Flow with PKCE support.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::{oauth_app, oauth_token};
use misskey_db::repositories::OAuthRepository;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

/// OAuth scopes.
pub mod scopes {
    pub const READ: &str = "read";
    pub const WRITE: &str = "write";
    pub const READ_ACCOUNT: &str = "read:account";
    pub const WRITE_ACCOUNT: &str = "write:account";
    pub const READ_NOTES: &str = "read:notes";
    pub const WRITE_NOTES: &str = "write:notes";
    pub const READ_NOTIFICATIONS: &str = "read:notifications";
    pub const WRITE_NOTIFICATIONS: &str = "write:notifications";
    pub const READ_FOLLOWING: &str = "read:following";
    pub const WRITE_FOLLOWING: &str = "write:following";
    pub const READ_DRIVE: &str = "read:drive";
    pub const WRITE_DRIVE: &str = "write:drive";
    pub const READ_FAVORITES: &str = "read:favorites";
    pub const WRITE_FAVORITES: &str = "write:favorites";

    /// Get all valid scopes.
    #[must_use]
    pub fn all() -> Vec<&'static str> {
        vec![
            READ,
            WRITE,
            READ_ACCOUNT,
            WRITE_ACCOUNT,
            READ_NOTES,
            WRITE_NOTES,
            READ_NOTIFICATIONS,
            WRITE_NOTIFICATIONS,
            READ_FOLLOWING,
            WRITE_FOLLOWING,
            READ_DRIVE,
            WRITE_DRIVE,
            READ_FAVORITES,
            WRITE_FAVORITES,
        ]
    }

    /// Check if a scope is valid.
    #[must_use]
    pub fn is_valid(scope: &str) -> bool {
        all().contains(&scope)
    }
}

/// Token expiration times in seconds.
pub mod expiry {
    pub const AUTHORIZATION_CODE: i64 = 600; // 10 minutes
    pub const ACCESS_TOKEN: i64 = 3600; // 1 hour
    pub const REFRESH_TOKEN: i64 = 30 * 24 * 3600; // 30 days
}

/// Input for creating an OAuth application.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppInput {
    pub name: String,
    pub description: Option<String>,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub website_url: Option<String>,
}

/// Input for updating an OAuth application.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
    pub website_url: Option<String>,
    pub is_active: Option<bool>,
}

/// Response for an OAuth application.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAppResponse {
    pub id: String,
    pub client_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub website_url: Option<String>,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub is_trusted: bool,
    pub is_active: bool,
    pub created_at: String,
}

impl From<oauth_app::Model> for OAuthAppResponse {
    fn from(app: oauth_app::Model) -> Self {
        Self {
            id: app.id,
            client_id: app.client_id,
            name: app.name,
            description: app.description,
            icon_url: app.icon_url,
            website_url: app.website_url,
            redirect_uris: serde_json::from_value(app.redirect_uris).unwrap_or_default(),
            scopes: serde_json::from_value(app.scopes).unwrap_or_default(),
            is_trusted: app.is_trusted,
            is_active: app.is_active,
            created_at: app.created_at.to_rfc3339(),
        }
    }
}

/// Response for an OAuth application including secret (only shown once).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAppWithSecretResponse {
    #[serde(flatten)]
    pub app: OAuthAppResponse,
    /// The client secret (only shown on creation).
    pub client_secret: String,
}

/// Input for authorization request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeInput {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Response for authorization (the authorization code).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizeResponse {
    pub code: String,
    pub state: Option<String>,
}

/// Input for token exchange.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenExchangeInput {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
}

/// Response for token exchange.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

/// Authorized application information for a user.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedAppResponse {
    pub app: OAuthAppResponse,
    pub scopes: Vec<String>,
    pub authorized_at: String,
}

/// OAuth service for managing applications and tokens.
#[derive(Clone)]
pub struct OAuthService {
    oauth_repo: OAuthRepository,
    id_gen: IdGenerator,
}

impl OAuthService {
    /// Create a new OAuth service.
    #[must_use]
    pub const fn new(oauth_repo: OAuthRepository) -> Self {
        Self {
            oauth_repo,
            id_gen: IdGenerator::new(),
        }
    }

    // ==================== Application Management ====================

    /// Create a new OAuth application.
    pub async fn create_app(
        &self,
        user_id: &str,
        input: CreateAppInput,
    ) -> AppResult<OAuthAppWithSecretResponse> {
        // Validate input
        if input.name.is_empty() || input.name.len() > 100 {
            return Err(AppError::Validation(
                "Name must be between 1 and 100 characters".to_string(),
            ));
        }

        if input.redirect_uris.is_empty() {
            return Err(AppError::Validation(
                "At least one redirect URI is required".to_string(),
            ));
        }

        // Validate redirect URIs
        for uri in &input.redirect_uris {
            if !uri.starts_with("http://")
                && !uri.starts_with("https://")
                && !uri.starts_with("urn:")
            {
                return Err(AppError::Validation(format!("Invalid redirect URI: {uri}")));
            }
        }

        // Validate scopes
        for scope in &input.scopes {
            if !scopes::is_valid(scope) {
                return Err(AppError::Validation(format!("Invalid scope: {scope}")));
            }
        }

        // Generate client ID and secret
        let client_id = self.generate_client_id();
        let client_secret = self.generate_client_secret();
        let client_secret_hash = self.hash_secret(&client_secret);

        let now = chrono::Utc::now();
        let id = self.id_gen.generate();

        let model = oauth_app::ActiveModel {
            id: Set(id),
            client_id: Set(client_id),
            client_secret: Set(client_secret_hash),
            name: Set(input.name),
            description: Set(input.description),
            icon_url: Set(None),
            website_url: Set(input.website_url),
            redirect_uris: Set(json!(input.redirect_uris)),
            scopes: Set(json!(input.scopes)),
            user_id: Set(user_id.to_string()),
            is_trusted: Set(false),
            is_active: Set(true),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        let app = self.oauth_repo.create_app(model).await?;

        Ok(OAuthAppWithSecretResponse {
            app: app.into(),
            client_secret,
        })
    }

    /// Update an OAuth application.
    pub async fn update_app(
        &self,
        user_id: &str,
        app_id: &str,
        input: UpdateAppInput,
    ) -> AppResult<OAuthAppResponse> {
        let app = self.oauth_repo.get_app_by_id(app_id).await?;

        // Verify ownership
        if app.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only update your own applications".to_string(),
            ));
        }

        let mut active: oauth_app::ActiveModel = app.into();

        if let Some(name) = input.name {
            if name.is_empty() || name.len() > 100 {
                return Err(AppError::Validation(
                    "Name must be between 1 and 100 characters".to_string(),
                ));
            }
            active.name = Set(name);
        }

        if let Some(description) = input.description {
            active.description = Set(Some(description));
        }

        if let Some(redirect_uris) = input.redirect_uris {
            if redirect_uris.is_empty() {
                return Err(AppError::Validation(
                    "At least one redirect URI is required".to_string(),
                ));
            }
            for uri in &redirect_uris {
                if !uri.starts_with("http://")
                    && !uri.starts_with("https://")
                    && !uri.starts_with("urn:")
                {
                    return Err(AppError::Validation(format!("Invalid redirect URI: {uri}")));
                }
            }
            active.redirect_uris = Set(json!(redirect_uris));
        }

        if let Some(requested_scopes) = input.scopes {
            for scope in &requested_scopes {
                if !scopes::is_valid(scope) {
                    return Err(AppError::Validation(format!("Invalid scope: {scope}")));
                }
            }
            active.scopes = Set(json!(requested_scopes));
        }

        if let Some(website_url) = input.website_url {
            active.website_url = Set(Some(website_url));
        }

        if let Some(is_active) = input.is_active {
            active.is_active = Set(is_active);
        }

        active.updated_at = Set(Some(chrono::Utc::now().into()));

        let updated = self.oauth_repo.update_app(active).await?;
        Ok(updated.into())
    }

    /// Delete an OAuth application.
    pub async fn delete_app(&self, user_id: &str, app_id: &str) -> AppResult<()> {
        let app = self.oauth_repo.get_app_by_id(app_id).await?;

        // Verify ownership
        if app.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only delete your own applications".to_string(),
            ));
        }

        self.oauth_repo.delete_app(app_id).await
    }

    /// Get an application by client ID (public info only).
    pub async fn get_app_by_client_id(&self, client_id: &str) -> AppResult<OAuthAppResponse> {
        let app = self.oauth_repo.get_app_by_client_id(client_id).await?;

        if !app.is_active {
            return Err(AppError::NotFound(
                "Application not found or inactive".to_string(),
            ));
        }

        Ok(app.into())
    }

    /// List applications created by a user.
    pub async fn list_apps_by_user(&self, user_id: &str) -> AppResult<Vec<OAuthAppResponse>> {
        let apps = self.oauth_repo.find_apps_by_user_id(user_id).await?;
        Ok(apps.into_iter().map(Into::into).collect())
    }

    // ==================== Authorization ====================

    /// Process an authorization request and generate an authorization code.
    pub async fn authorize(
        &self,
        user_id: &str,
        input: AuthorizeInput,
    ) -> AppResult<AuthorizeResponse> {
        // Validate response_type
        if input.response_type != "code" {
            return Err(AppError::Validation(
                "Only 'code' response_type is supported".to_string(),
            ));
        }

        // Get the application
        let app = self
            .oauth_repo
            .get_app_by_client_id(&input.client_id)
            .await?;

        if !app.is_active {
            return Err(AppError::Validation(
                "Application is not active".to_string(),
            ));
        }

        // Validate redirect URI
        let allowed_uris: Vec<String> =
            serde_json::from_value(app.redirect_uris.clone()).unwrap_or_default();
        if !allowed_uris.contains(&input.redirect_uri) {
            return Err(AppError::Validation("Invalid redirect_uri".to_string()));
        }

        // Validate scopes
        let requested_scopes: Vec<&str> = input.scope.split_whitespace().collect();
        let app_scopes: Vec<String> =
            serde_json::from_value(app.scopes.clone()).unwrap_or_default();
        for scope in &requested_scopes {
            if !app_scopes.iter().any(|s| s == *scope) {
                return Err(AppError::Validation(format!(
                    "Scope '{scope}' is not allowed for this application"
                )));
            }
        }

        // Validate PKCE if provided
        if let Some(ref method) = input.code_challenge_method {
            if method != "S256" && method != "plain" {
                return Err(AppError::Validation(
                    "Only 'S256' and 'plain' code_challenge_method are supported".to_string(),
                ));
            }
            if input.code_challenge.is_none() {
                return Err(AppError::Validation(
                    "code_challenge is required when code_challenge_method is provided".to_string(),
                ));
            }
        }

        // Generate authorization code
        let code = self.generate_token();
        let code_hash = self.hash_token(&code);

        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(expiry::AUTHORIZATION_CODE);

        let token_model = oauth_token::ActiveModel {
            id: Set(self.id_gen.generate()),
            token_hash: Set(code_hash),
            token_type: Set(oauth_token::TokenType::AuthorizationCode),
            app_id: Set(app.id),
            user_id: Set(user_id.to_string()),
            scopes: Set(json!(requested_scopes)),
            code_challenge: Set(input.code_challenge),
            code_challenge_method: Set(input.code_challenge_method),
            redirect_uri: Set(Some(input.redirect_uri)),
            expires_at: Set(expires_at.into()),
            is_revoked: Set(false),
            created_at: Set(now.into()),
            last_used_at: Set(None),
        };

        self.oauth_repo.create_token(token_model).await?;

        Ok(AuthorizeResponse {
            code,
            state: input.state,
        })
    }

    /// Exchange an authorization code for access and refresh tokens.
    pub async fn exchange_token(&self, input: TokenExchangeInput) -> AppResult<TokenResponse> {
        match input.grant_type.as_str() {
            "authorization_code" => self.exchange_authorization_code(input).await,
            "refresh_token" => self.exchange_refresh_token(input).await,
            _ => Err(AppError::Validation(format!(
                "Unsupported grant_type: {}",
                input.grant_type
            ))),
        }
    }

    async fn exchange_authorization_code(
        &self,
        input: TokenExchangeInput,
    ) -> AppResult<TokenResponse> {
        let code = input
            .code
            .ok_or_else(|| AppError::Validation("code is required".to_string()))?;

        let redirect_uri = input
            .redirect_uri
            .ok_or_else(|| AppError::Validation("redirect_uri is required".to_string()))?;

        // Find the authorization code
        let code_hash = self.hash_token(&code);
        let token = self
            .oauth_repo
            .find_token_by_hash(&code_hash)
            .await?
            .ok_or_else(|| {
                AppError::Validation("Invalid or expired authorization code".to_string())
            })?;

        // Verify it's an authorization code
        if token.token_type != oauth_token::TokenType::AuthorizationCode {
            return Err(AppError::Validation("Invalid token type".to_string()));
        }

        // Verify not revoked and not expired
        if token.is_revoked {
            return Err(AppError::Validation(
                "Authorization code has been revoked".to_string(),
            ));
        }

        let now = chrono::Utc::now().fixed_offset();
        if token.expires_at < now {
            return Err(AppError::Validation(
                "Authorization code has expired".to_string(),
            ));
        }

        // Verify redirect_uri matches
        if token.redirect_uri.as_ref() != Some(&redirect_uri) {
            return Err(AppError::Validation("redirect_uri mismatch".to_string()));
        }

        // Get the application
        let app = self.oauth_repo.get_app_by_id(&token.app_id).await?;

        // Verify client_id
        if app.client_id != input.client_id {
            return Err(AppError::Validation("client_id mismatch".to_string()));
        }

        // Verify PKCE if it was used
        if let Some(ref challenge) = token.code_challenge {
            let verifier = input
                .code_verifier
                .ok_or_else(|| AppError::Validation("code_verifier is required".to_string()))?;

            let method = token.code_challenge_method.as_deref().unwrap_or("plain");
            let computed_challenge = if method == "S256" {
                let mut hasher = Sha256::new();
                hasher.update(verifier.as_bytes());
                URL_SAFE_NO_PAD.encode(hasher.finalize())
            } else {
                verifier
            };

            if &computed_challenge != challenge {
                return Err(AppError::Validation("Invalid code_verifier".to_string()));
            }
        } else if !app.is_trusted {
            // Non-trusted apps should use PKCE, but verify client_secret instead
            let client_secret = input.client_secret.ok_or_else(|| {
                AppError::Validation("client_secret is required for non-PKCE flow".to_string())
            })?;

            if !self.verify_secret(&client_secret, &app.client_secret) {
                return Err(AppError::Validation("Invalid client_secret".to_string()));
            }
        }

        // Revoke the authorization code (single use)
        self.oauth_repo.revoke_token(&token.id).await?;

        // Generate access token
        let access_token = self.generate_token();
        let access_token_hash = self.hash_token(&access_token);
        let access_expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(expiry::ACCESS_TOKEN);

        let access_token_model = oauth_token::ActiveModel {
            id: Set(self.id_gen.generate()),
            token_hash: Set(access_token_hash),
            token_type: Set(oauth_token::TokenType::AccessToken),
            app_id: Set(app.id.clone()),
            user_id: Set(token.user_id.clone()),
            scopes: Set(token.scopes.clone()),
            code_challenge: Set(None),
            code_challenge_method: Set(None),
            redirect_uri: Set(None),
            expires_at: Set(access_expires_at.into()),
            is_revoked: Set(false),
            created_at: Set(chrono::Utc::now().into()),
            last_used_at: Set(None),
        };

        self.oauth_repo.create_token(access_token_model).await?;

        // Generate refresh token
        let refresh_token = self.generate_token();
        let refresh_token_hash = self.hash_token(&refresh_token);
        let refresh_expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(expiry::REFRESH_TOKEN);

        let refresh_token_model = oauth_token::ActiveModel {
            id: Set(self.id_gen.generate()),
            token_hash: Set(refresh_token_hash),
            token_type: Set(oauth_token::TokenType::RefreshToken),
            app_id: Set(app.id),
            user_id: Set(token.user_id),
            scopes: Set(token.scopes.clone()),
            code_challenge: Set(None),
            code_challenge_method: Set(None),
            redirect_uri: Set(None),
            expires_at: Set(refresh_expires_at.into()),
            is_revoked: Set(false),
            created_at: Set(chrono::Utc::now().into()),
            last_used_at: Set(None),
        };

        self.oauth_repo.create_token(refresh_token_model).await?;

        let scopes: Vec<String> = serde_json::from_value(token.scopes).unwrap_or_default();

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: expiry::ACCESS_TOKEN,
            refresh_token: Some(refresh_token),
            scope: scopes.join(" "),
        })
    }

    async fn exchange_refresh_token(&self, input: TokenExchangeInput) -> AppResult<TokenResponse> {
        let refresh_token_str = input
            .refresh_token
            .ok_or_else(|| AppError::Validation("refresh_token is required".to_string()))?;

        // Find the refresh token
        let refresh_hash = self.hash_token(&refresh_token_str);
        let refresh_token = self
            .oauth_repo
            .find_token_by_hash(&refresh_hash)
            .await?
            .ok_or_else(|| AppError::Validation("Invalid refresh token".to_string()))?;

        // Verify it's a refresh token
        if refresh_token.token_type != oauth_token::TokenType::RefreshToken {
            return Err(AppError::Validation("Invalid token type".to_string()));
        }

        // Verify not revoked and not expired
        if refresh_token.is_revoked {
            return Err(AppError::Validation(
                "Refresh token has been revoked".to_string(),
            ));
        }

        let now = chrono::Utc::now().fixed_offset();
        if refresh_token.expires_at < now {
            return Err(AppError::Validation(
                "Refresh token has expired".to_string(),
            ));
        }

        // Get the application
        let app = self.oauth_repo.get_app_by_id(&refresh_token.app_id).await?;

        // Verify client_id
        if app.client_id != input.client_id {
            return Err(AppError::Validation("client_id mismatch".to_string()));
        }

        // Generate new access token
        let access_token = self.generate_token();
        let access_token_hash = self.hash_token(&access_token);
        let access_expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(expiry::ACCESS_TOKEN);

        let access_token_model = oauth_token::ActiveModel {
            id: Set(self.id_gen.generate()),
            token_hash: Set(access_token_hash),
            token_type: Set(oauth_token::TokenType::AccessToken),
            app_id: Set(app.id),
            user_id: Set(refresh_token.user_id),
            scopes: Set(refresh_token.scopes.clone()),
            code_challenge: Set(None),
            code_challenge_method: Set(None),
            redirect_uri: Set(None),
            expires_at: Set(access_expires_at.into()),
            is_revoked: Set(false),
            created_at: Set(chrono::Utc::now().into()),
            last_used_at: Set(None),
        };

        self.oauth_repo.create_token(access_token_model).await?;

        let scopes: Vec<String> = serde_json::from_value(refresh_token.scopes).unwrap_or_default();

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: expiry::ACCESS_TOKEN,
            refresh_token: None, // Don't rotate refresh token
            scope: scopes.join(" "),
        })
    }

    /// Validate an access token and return the user ID if valid.
    pub async fn validate_access_token(&self, token: &str) -> AppResult<(String, Vec<String>)> {
        let token_hash = self.hash_token(token);
        let token_record = self
            .oauth_repo
            .find_token_by_hash(&token_hash)
            .await?
            .ok_or(AppError::Unauthorized)?;

        // Verify it's an access token
        if token_record.token_type != oauth_token::TokenType::AccessToken {
            return Err(AppError::Unauthorized);
        }

        // Verify not revoked and not expired
        if token_record.is_revoked {
            return Err(AppError::Unauthorized);
        }

        let now = chrono::Utc::now().fixed_offset();
        if token_record.expires_at < now {
            return Err(AppError::Unauthorized);
        }

        // Update last_used_at
        let _ = self.oauth_repo.touch_token(&token_record.id).await;

        let scopes: Vec<String> = serde_json::from_value(token_record.scopes).unwrap_or_default();

        Ok((token_record.user_id, scopes))
    }

    /// Revoke a token.
    pub async fn revoke_token(&self, token: &str) -> AppResult<()> {
        let token_hash = self.hash_token(token);
        if let Some(token_record) = self.oauth_repo.find_token_by_hash(&token_hash).await? {
            self.oauth_repo.revoke_token(&token_record.id).await?;
        }
        Ok(())
    }

    /// Revoke all tokens for a user and application.
    pub async fn revoke_app_authorization(&self, user_id: &str, app_id: &str) -> AppResult<()> {
        self.oauth_repo
            .revoke_tokens_for_user_app(user_id, app_id)
            .await?;
        Ok(())
    }

    /// List authorized applications for a user.
    pub async fn list_authorized_apps(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<AuthorizedAppResponse>> {
        let tokens = self.oauth_repo.find_tokens_by_user_id(user_id).await?;

        // Group by app_id and get latest
        let mut apps_map = std::collections::HashMap::new();
        for token in tokens {
            if token.token_type == oauth_token::TokenType::AccessToken {
                apps_map.entry(token.app_id.clone()).or_insert(token);
            }
        }

        let mut result = Vec::new();
        for (_, token) in apps_map {
            if let Ok(app) = self.oauth_repo.get_app_by_id(&token.app_id).await {
                let scopes: Vec<String> = serde_json::from_value(token.scopes).unwrap_or_default();
                result.push(AuthorizedAppResponse {
                    app: app.into(),
                    scopes,
                    authorized_at: token.created_at.to_rfc3339(),
                });
            }
        }

        Ok(result)
    }

    /// Cleanup expired tokens.
    pub async fn cleanup_expired_tokens(&self) -> AppResult<u64> {
        self.oauth_repo.delete_expired_tokens().await
    }

    // ==================== Helper Methods ====================

    fn generate_client_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn generate_client_secret(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn generate_token(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        URL_SAFE_NO_PAD.encode(hasher.finalize())
    }

    fn hash_secret(&self, secret: &str) -> String {
        self.hash_token(secret)
    }

    fn verify_secret(&self, secret: &str, hash: &str) -> bool {
        self.hash_secret(secret) == hash
    }
}
