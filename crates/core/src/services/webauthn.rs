//! WebAuthn/Passkey authentication service.
//!
//! This service handles security key registration and authentication
//! using the `WebAuthn` standard.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::security_key;
use misskey_db::repositories::{SecurityKeyRepository, UserProfileRepository, UserRepository};
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;
use webauthn_rs::prelude::*;

/// `WebAuthn` configuration.
#[derive(Debug, Clone)]
pub struct WebAuthnConfig {
    /// The relying party ID (usually the domain name).
    pub rp_id: String,
    /// The relying party name (displayed to users).
    pub rp_name: String,
    /// The origin URL (e.g., `https://example.com`).
    pub origin: Url,
}

impl WebAuthnConfig {
    /// Create a new `WebAuthn` configuration from a server URL.
    ///
    /// # Errors
    /// Returns an error if the URL is invalid.
    pub fn from_server_url(server_url: &str, rp_name: &str) -> AppResult<Self> {
        let url = Url::parse(server_url)
            .map_err(|e| AppError::Internal(format!("Invalid server URL: {e}")))?;

        let rp_id = url
            .host_str()
            .ok_or_else(|| AppError::Internal("Server URL has no host".to_string()))?
            .to_string();

        Ok(Self {
            rp_id,
            rp_name: rp_name.to_string(),
            origin: url,
        })
    }
}

/// Registration challenge state (stored temporarily during registration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationState {
    /// The passkey registration state (serialized).
    pub state: String,
    /// When this challenge expires.
    pub expires_at: i64,
}

/// Authentication challenge state (stored temporarily during authentication).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationState {
    /// The passkey authentication state (serialized).
    pub state: String,
    /// When this challenge expires.
    pub expires_at: i64,
}

/// Response for starting security key registration.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginRegistrationResponse {
    /// The challenge ID (for tracking the registration flow).
    pub challenge_id: String,
    /// The public key credential creation options (send to browser).
    pub options: serde_json::Value,
}

/// Input for completing security key registration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRegistrationInput {
    /// The challenge ID from the begin response.
    pub challenge_id: String,
    /// User-provided name for this security key.
    pub name: String,
    /// The credential response from the browser.
    pub credential: serde_json::Value,
}

/// Response for a registered security key.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityKeyResponse {
    pub id: String,
    pub name: String,
    pub is_passkey: bool,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

impl From<security_key::Model> for SecurityKeyResponse {
    fn from(key: security_key::Model) -> Self {
        Self {
            id: key.id,
            name: key.name,
            is_passkey: key.is_passkey,
            last_used_at: key.last_used_at.map(|t| t.to_rfc3339()),
            created_at: key.created_at.to_rfc3339(),
        }
    }
}

/// Response for starting security key authentication.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginAuthenticationResponse {
    /// The challenge ID (for tracking the authentication flow).
    pub challenge_id: String,
    /// The public key credential request options (send to browser).
    pub options: serde_json::Value,
}

/// Input for completing security key authentication.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteAuthenticationInput {
    /// The challenge ID from the begin response.
    pub challenge_id: String,
    /// The credential response from the browser.
    pub credential: serde_json::Value,
}

/// Challenge expiration time in seconds.
const CHALLENGE_EXPIRY_SECS: i64 = 300; // 5 minutes

/// Service for managing WebAuthn/Passkey authentication.
#[derive(Clone)]
pub struct WebAuthnService {
    webauthn: Arc<Webauthn>,
    security_key_repo: SecurityKeyRepository,
    user_repo: UserRepository,
    profile_repo: UserProfileRepository,
    id_gen: IdGenerator,
    /// In-memory storage for registration challenges.
    /// In production, this should use Redis or similar.
    registration_challenges: Arc<RwLock<HashMap<String, RegistrationState>>>,
    /// In-memory storage for authentication challenges.
    authentication_challenges: Arc<RwLock<HashMap<String, AuthenticationState>>>,
}

impl WebAuthnService {
    /// Create a new `WebAuthn` service.
    ///
    /// # Errors
    /// Returns an error if the `WebAuthn` configuration is invalid.
    pub fn new(
        config: &WebAuthnConfig,
        security_key_repo: SecurityKeyRepository,
        user_repo: UserRepository,
        profile_repo: UserProfileRepository,
    ) -> AppResult<Self> {
        let rp_id = config.rp_id.clone();
        let rp_origin = config.origin.clone();

        let builder = WebauthnBuilder::new(&rp_id, &rp_origin)
            .map_err(|e| AppError::Internal(format!("Failed to create WebAuthn builder: {e}")))?
            .rp_name(&config.rp_name);

        let webauthn = builder
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build WebAuthn: {e}")))?;

        Ok(Self {
            webauthn: Arc::new(webauthn),
            security_key_repo,
            user_repo,
            profile_repo,
            id_gen: IdGenerator::new(),
            registration_challenges: Arc::new(RwLock::new(HashMap::new())),
            authentication_challenges: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    // ==================== Registration ====================

    /// Begin security key registration for a user.
    pub async fn begin_registration(&self, user_id: &str) -> AppResult<BeginRegistrationResponse> {
        // Check if user exists
        let user = self.user_repo.get_by_id(user_id).await?;

        // Check if user has reached the limit
        if self.security_key_repo.user_at_limit(user_id).await? {
            return Err(AppError::Validation(
                "Maximum number of security keys reached".to_string(),
            ));
        }

        // Get existing credentials to exclude
        let existing_keys = self.security_key_repo.find_by_user_id(user_id).await?;
        let exclude_credentials: Vec<CredentialID> = existing_keys
            .iter()
            .filter_map(|k| URL_SAFE_NO_PAD.decode(&k.credential_id).ok())
            .map(CredentialID::from)
            .collect();

        // Create user unique ID (use user_id bytes)
        let user_unique_id = user_id.as_bytes().to_vec();

        // Start registration
        let (ccr, reg_state) = self
            .webauthn
            .start_passkey_registration(
                Uuid::new_v4(),
                &user.username,
                &user.name.clone().unwrap_or_else(|| user.username.clone()),
                Some(exclude_credentials),
            )
            .map_err(|e| AppError::Internal(format!("Failed to start registration: {e}")))?;

        // Generate challenge ID
        let challenge_id = self.id_gen.generate();

        // Serialize and store the registration state
        let state_json = serde_json::to_string(&reg_state)
            .map_err(|e| AppError::Internal(format!("Failed to serialize state: {e}")))?;

        let expires_at = chrono::Utc::now().timestamp() + CHALLENGE_EXPIRY_SECS;

        {
            let mut challenges = self.registration_challenges.write().await;
            challenges.insert(
                format!("{user_id}:{challenge_id}"),
                RegistrationState {
                    state: state_json,
                    expires_at,
                },
            );
        }

        // Convert options to JSON
        let options = serde_json::to_value(&ccr)
            .map_err(|e| AppError::Internal(format!("Failed to serialize options: {e}")))?;

        Ok(BeginRegistrationResponse {
            challenge_id,
            options,
        })
    }

    /// Complete security key registration.
    pub async fn complete_registration(
        &self,
        user_id: &str,
        input: CompleteRegistrationInput,
    ) -> AppResult<SecurityKeyResponse> {
        // Retrieve and remove the registration state
        let state = {
            let mut challenges = self.registration_challenges.write().await;
            let key = format!("{}:{}", user_id, input.challenge_id);
            challenges
                .remove(&key)
                .ok_or_else(|| AppError::Validation("Invalid or expired challenge".to_string()))?
        };

        // Check expiration
        if chrono::Utc::now().timestamp() > state.expires_at {
            return Err(AppError::Validation("Challenge has expired".to_string()));
        }

        // Deserialize the registration state
        let reg_state: PasskeyRegistration = serde_json::from_str(&state.state)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize state: {e}")))?;

        // Parse the credential response
        let reg_response: RegisterPublicKeyCredential = serde_json::from_value(input.credential)
            .map_err(|e| AppError::Validation(format!("Invalid credential response: {e}")))?;

        // Complete registration
        let passkey = self
            .webauthn
            .finish_passkey_registration(&reg_response, &reg_state)
            .map_err(|e| AppError::Validation(format!("Registration failed: {e}")))?;

        // Extract credential data
        let credential_id = URL_SAFE_NO_PAD.encode(passkey.cred_id());
        let public_key = serde_json::to_string(&passkey)
            .map_err(|e| AppError::Internal(format!("Failed to serialize passkey: {e}")))?;

        // Extract transports
        let transports: Vec<String> = vec![]; // webauthn-rs handles this internally

        // Create security key record
        let key_id = self.id_gen.generate();
        let now = chrono::Utc::now();

        let model = security_key::ActiveModel {
            id: Set(key_id.clone()),
            user_id: Set(user_id.to_string()),
            name: Set(input.name),
            credential_id: Set(credential_id),
            public_key: Set(public_key),
            counter: Set(0),
            credential_type: Set("public-key".to_string()),
            transports: Set(json!(transports)),
            aaguid: Set(None),
            is_passkey: Set(true),
            last_used_at: Set(None),
            created_at: Set(now.into()),
        };

        let key = self.security_key_repo.create(model).await?;

        Ok(key.into())
    }

    // ==================== Authentication ====================

    /// Begin security key authentication for a user.
    pub async fn begin_authentication(
        &self,
        user_id: &str,
    ) -> AppResult<BeginAuthenticationResponse> {
        // Get user's security keys
        let keys = self.security_key_repo.find_by_user_id(user_id).await?;

        if keys.is_empty() {
            return Err(AppError::Validation(
                "No security keys registered".to_string(),
            ));
        }

        // Convert to passkeys
        let passkeys: Vec<Passkey> = keys
            .iter()
            .filter_map(|k| serde_json::from_str(&k.public_key).ok())
            .collect();

        if passkeys.is_empty() {
            return Err(AppError::Internal(
                "Failed to load security keys".to_string(),
            ));
        }

        // Start authentication
        let (rcr, auth_state) = self
            .webauthn
            .start_passkey_authentication(&passkeys)
            .map_err(|e| AppError::Internal(format!("Failed to start authentication: {e}")))?;

        // Generate challenge ID
        let challenge_id = self.id_gen.generate();

        // Serialize and store the authentication state
        let state_json = serde_json::to_string(&auth_state)
            .map_err(|e| AppError::Internal(format!("Failed to serialize state: {e}")))?;

        let expires_at = chrono::Utc::now().timestamp() + CHALLENGE_EXPIRY_SECS;

        {
            let mut challenges = self.authentication_challenges.write().await;
            challenges.insert(
                format!("{user_id}:{challenge_id}"),
                AuthenticationState {
                    state: state_json,
                    expires_at,
                },
            );
        }

        // Convert options to JSON
        let options = serde_json::to_value(&rcr)
            .map_err(|e| AppError::Internal(format!("Failed to serialize options: {e}")))?;

        Ok(BeginAuthenticationResponse {
            challenge_id,
            options,
        })
    }

    /// Complete security key authentication.
    pub async fn complete_authentication(
        &self,
        user_id: &str,
        input: CompleteAuthenticationInput,
    ) -> AppResult<bool> {
        // Retrieve and remove the authentication state
        let state = {
            let mut challenges = self.authentication_challenges.write().await;
            let key = format!("{}:{}", user_id, input.challenge_id);
            challenges
                .remove(&key)
                .ok_or_else(|| AppError::Validation("Invalid or expired challenge".to_string()))?
        };

        // Check expiration
        if chrono::Utc::now().timestamp() > state.expires_at {
            return Err(AppError::Validation("Challenge has expired".to_string()));
        }

        // Deserialize the authentication state
        let auth_state: PasskeyAuthentication = serde_json::from_str(&state.state)
            .map_err(|e| AppError::Internal(format!("Failed to deserialize state: {e}")))?;

        // Parse the credential response
        let auth_response: PublicKeyCredential = serde_json::from_value(input.credential)
            .map_err(|e| AppError::Validation(format!("Invalid credential response: {e}")))?;

        // Complete authentication
        let auth_result = self
            .webauthn
            .finish_passkey_authentication(&auth_response, &auth_state)
            .map_err(|e| AppError::Validation(format!("Authentication failed: {e}")))?;

        // Update counter for the used credential
        let credential_id = URL_SAFE_NO_PAD.encode(auth_result.cred_id());
        if let Some(key) = self
            .security_key_repo
            .find_by_credential_id(&credential_id)
            .await?
        {
            self.security_key_repo
                .update_counter(&key.id, i64::from(auth_result.counter()))
                .await?;
        }

        Ok(true)
    }

    // ==================== Management ====================

    /// List all security keys for a user.
    pub async fn list_keys(&self, user_id: &str) -> AppResult<Vec<SecurityKeyResponse>> {
        let keys = self.security_key_repo.find_by_user_id(user_id).await?;
        Ok(keys.into_iter().map(Into::into).collect())
    }

    /// Rename a security key.
    pub async fn rename_key(&self, user_id: &str, key_id: &str, name: &str) -> AppResult<()> {
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::Validation(
                "Name must be between 1 and 100 characters".to_string(),
            ));
        }

        self.security_key_repo
            .update_name(key_id, user_id, name)
            .await
    }

    /// Delete a security key.
    pub async fn delete_key(&self, user_id: &str, key_id: &str) -> AppResult<()> {
        self.security_key_repo.delete(key_id, user_id).await
    }

    /// Check if a user has any security keys.
    pub async fn has_security_keys(&self, user_id: &str) -> AppResult<bool> {
        self.security_key_repo.user_has_security_keys(user_id).await
    }

    /// Clean up expired challenges (should be called periodically).
    pub async fn cleanup_expired_challenges(&self) {
        let now = chrono::Utc::now().timestamp();

        {
            let mut reg_challenges = self.registration_challenges.write().await;
            reg_challenges.retain(|_, state| state.expires_at > now);
        }

        {
            let mut auth_challenges = self.authentication_challenges.write().await;
            auth_challenges.retain(|_, state| state.expires_at > now);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_webauthn_config_from_url() {
        let config =
            WebAuthnConfig::from_server_url("https://example.com", "Example Server").unwrap();

        assert_eq!(config.rp_id, "example.com");
        assert_eq!(config.rp_name, "Example Server");
    }

    #[test]
    fn test_webauthn_config_invalid_url() {
        let result = WebAuthnConfig::from_server_url("not-a-url", "Test");
        assert!(result.is_err());
    }
}
