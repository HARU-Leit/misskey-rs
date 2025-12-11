//! Two-factor authentication service.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use misskey_common::{AppError, AppResult};
use misskey_db::entities::user_profile;
use misskey_db::repositories::UserProfileRepository;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, Secret, TOTP};

/// Number of backup codes to generate.
const BACKUP_CODE_COUNT: usize = 10;

/// Length of each backup code (digits).
const BACKUP_CODE_LENGTH: usize = 8;

/// TOTP configuration.
const TOTP_DIGITS: usize = 6;
const TOTP_STEP: u64 = 30;
const TOTP_SKEW: u8 = 1;

/// Response for 2FA setup initiation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorSetupResponse {
    /// The secret in base32 format (for manual entry).
    pub secret: String,
    /// QR code as base64 PNG image.
    pub qr_code: String,
    /// otpauth URI for authenticator apps.
    pub otpauth_url: String,
}

/// Response for successful 2FA confirmation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorConfirmResponse {
    /// Backup codes (plain text, shown only once).
    pub backup_codes: Vec<String>,
}

/// Input for confirming 2FA setup.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmTwoFactorInput {
    /// The TOTP code from the authenticator app.
    pub token: String,
    /// User's password to confirm identity.
    pub password: String,
}

/// Input for disabling 2FA.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisableTwoFactorInput {
    /// User's password to confirm identity.
    pub password: String,
    /// A TOTP token or backup code.
    pub token: String,
}

/// Input for verifying 2FA during login.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTwoFactorInput {
    /// TOTP token or backup code.
    pub token: String,
}

/// Service for managing two-factor authentication.
#[derive(Clone)]
pub struct TwoFactorService {
    profile_repo: UserProfileRepository,
}

impl TwoFactorService {
    /// Create a new two-factor service.
    #[must_use]
    pub const fn new(profile_repo: UserProfileRepository) -> Self {
        Self { profile_repo }
    }

    /// Check if 2FA is enabled for a user.
    pub async fn is_enabled(&self, user_id: &str) -> AppResult<bool> {
        let profile = self.profile_repo.find_by_user_id(user_id).await?;
        Ok(profile.map(|p| p.two_factor_enabled).unwrap_or(false))
    }

    /// Initiate 2FA setup for a user.
    pub async fn begin_setup(
        &self,
        user_id: &str,
        username: &str,
        issuer: &str,
    ) -> AppResult<TwoFactorSetupResponse> {
        // Check if already enabled
        let profile = self.profile_repo.get_by_user_id(user_id).await?;

        if profile.two_factor_enabled {
            return Err(AppError::Validation(
                "Two-factor authentication is already enabled".to_string(),
            ));
        }

        // Generate a new secret
        let secret = Secret::generate_secret();
        let secret_base32 = secret.to_encoded().to_string();

        // Create TOTP instance
        let totp = TOTP::new(
            Algorithm::SHA1,
            TOTP_DIGITS,
            TOTP_SKEW,
            TOTP_STEP,
            secret
                .to_bytes()
                .map_err(|e| AppError::Internal(e.to_string()))?,
            Some(issuer.to_string()),
            username.to_string(),
        )
        .map_err(|e| AppError::Internal(format!("Failed to create TOTP: {}", e)))?;

        // Generate QR code
        let qr_code = totp
            .get_qr_base64()
            .map_err(|e| AppError::Internal(format!("Failed to generate QR code: {}", e)))?;

        // Get otpauth URL
        let otpauth_url = totp.get_url();

        // Store the pending secret
        let mut active: user_profile::ActiveModel = profile.into();
        active.two_factor_pending = Set(Some(secret_base32.clone()));
        self.profile_repo.update(active).await?;

        Ok(TwoFactorSetupResponse {
            secret: secret_base32,
            qr_code,
            otpauth_url,
        })
    }

    /// Confirm 2FA setup by verifying a TOTP token.
    pub async fn confirm_setup(
        &self,
        user_id: &str,
        input: ConfirmTwoFactorInput,
    ) -> AppResult<TwoFactorConfirmResponse> {
        let profile = self.profile_repo.get_by_user_id(user_id).await?;

        // Verify password
        self.verify_password(&profile, &input.password)?;

        // Get pending secret (clone to avoid borrow issues)
        let pending_secret = profile
            .two_factor_pending
            .clone()
            .ok_or_else(|| AppError::Validation("No pending 2FA setup found".to_string()))?;

        // Verify the token
        if !self.verify_totp(&pending_secret, &input.token)? {
            return Err(AppError::Validation(
                "Invalid verification code".to_string(),
            ));
        }

        // Generate backup codes
        let (plain_codes, hashed_codes) = self.generate_backup_codes()?;

        // Enable 2FA
        let mut active: user_profile::ActiveModel = profile.into();
        active.two_factor_secret = Set(Some(pending_secret));
        active.two_factor_enabled = Set(true);
        active.two_factor_pending = Set(None);
        active.two_factor_backup_codes = Set(Some(serde_json::json!(hashed_codes)));
        self.profile_repo.update(active).await?;

        Ok(TwoFactorConfirmResponse {
            backup_codes: plain_codes,
        })
    }

    /// Disable 2FA for a user.
    pub async fn disable(&self, user_id: &str, input: DisableTwoFactorInput) -> AppResult<()> {
        let profile = self.profile_repo.get_by_user_id(user_id).await?;

        if !profile.two_factor_enabled {
            return Err(AppError::Validation(
                "Two-factor authentication is not enabled".to_string(),
            ));
        }

        // Verify password
        self.verify_password(&profile, &input.password)?;

        // Verify token (TOTP or backup code)
        let secret = profile
            .two_factor_secret
            .as_ref()
            .ok_or_else(|| AppError::Internal("2FA enabled but no secret found".to_string()))?;

        let is_valid_totp = self.verify_totp(secret, &input.token)?;
        let is_valid_backup = if !is_valid_totp {
            self.verify_backup_code(&profile, &input.token)?
        } else {
            false
        };

        if !is_valid_totp && !is_valid_backup {
            return Err(AppError::Validation(
                "Invalid verification code or backup code".to_string(),
            ));
        }

        // Disable 2FA
        let mut active: user_profile::ActiveModel = profile.into();
        active.two_factor_secret = Set(None);
        active.two_factor_enabled = Set(false);
        active.two_factor_pending = Set(None);
        active.two_factor_backup_codes = Set(None);
        self.profile_repo.update(active).await?;

        Ok(())
    }

    /// Verify a 2FA token during login.
    pub async fn verify(&self, user_id: &str, token: &str) -> AppResult<bool> {
        let profile = self.profile_repo.get_by_user_id(user_id).await?;

        if !profile.two_factor_enabled {
            return Err(AppError::Validation(
                "Two-factor authentication is not enabled".to_string(),
            ));
        }

        let secret = profile
            .two_factor_secret
            .as_ref()
            .ok_or_else(|| AppError::Internal("2FA enabled but no secret found".to_string()))?;

        // Try TOTP first
        if self.verify_totp(secret, token)? {
            return Ok(true);
        }

        // Try backup code
        if self
            .verify_and_consume_backup_code(user_id, &profile, token)
            .await?
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Regenerate backup codes.
    pub async fn regenerate_backup_codes(
        &self,
        user_id: &str,
        password: &str,
    ) -> AppResult<Vec<String>> {
        let profile = self.profile_repo.get_by_user_id(user_id).await?;

        if !profile.two_factor_enabled {
            return Err(AppError::Validation(
                "Two-factor authentication is not enabled".to_string(),
            ));
        }

        // Verify password
        self.verify_password(&profile, password)?;

        // Generate new backup codes
        let (plain_codes, hashed_codes) = self.generate_backup_codes()?;

        // Update backup codes
        let mut active: user_profile::ActiveModel = profile.into();
        active.two_factor_backup_codes = Set(Some(serde_json::json!(hashed_codes)));
        self.profile_repo.update(active).await?;

        Ok(plain_codes)
    }

    /// Cancel pending 2FA setup.
    pub async fn cancel_setup(&self, user_id: &str) -> AppResult<()> {
        let profile = self.profile_repo.get_by_user_id(user_id).await?;

        if profile.two_factor_pending.is_none() {
            return Err(AppError::Validation(
                "No pending 2FA setup to cancel".to_string(),
            ));
        }

        let mut active: user_profile::ActiveModel = profile.into();
        active.two_factor_pending = Set(None);
        self.profile_repo.update(active).await?;

        Ok(())
    }

    // ==================== Helper Methods ====================

    fn verify_totp(&self, secret_base32: &str, token: &str) -> AppResult<bool> {
        let secret = Secret::Encoded(secret_base32.to_string());
        let secret_bytes = secret
            .to_bytes()
            .map_err(|e| AppError::Internal(format!("Invalid secret: {}", e)))?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            TOTP_DIGITS,
            TOTP_SKEW,
            TOTP_STEP,
            secret_bytes,
            None,
            String::new(),
        )
        .map_err(|e| AppError::Internal(format!("Failed to create TOTP: {}", e)))?;

        Ok(totp.check_current(token).unwrap_or(false))
    }

    fn verify_password(&self, profile: &user_profile::Model, password: &str) -> AppResult<()> {
        let hash = profile
            .password
            .as_ref()
            .ok_or_else(|| AppError::Validation("No password set for this account".to_string()))?;

        let parsed_hash = argon2::password_hash::PasswordHash::new(hash)
            .map_err(|_| AppError::Internal("Invalid password hash".to_string()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Validation("Invalid password".to_string()))?;

        Ok(())
    }

    fn generate_backup_codes(&self) -> AppResult<(Vec<String>, Vec<String>)> {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let argon2 = Argon2::default();

        let mut plain_codes = Vec::with_capacity(BACKUP_CODE_COUNT);
        let mut hashed_codes = Vec::with_capacity(BACKUP_CODE_COUNT);

        for _ in 0..BACKUP_CODE_COUNT {
            // Generate random digits
            let code: String = (0..BACKUP_CODE_LENGTH)
                .map(|_| rng.gen_range(0..10).to_string())
                .collect();

            // Hash the code
            let salt = SaltString::generate(&mut OsRng);
            let hash = argon2
                .hash_password(code.as_bytes(), &salt)
                .map_err(|e| AppError::Internal(format!("Failed to hash backup code: {}", e)))?
                .to_string();

            plain_codes.push(code);
            hashed_codes.push(hash);
        }

        Ok((plain_codes, hashed_codes))
    }

    fn verify_backup_code(&self, profile: &user_profile::Model, code: &str) -> AppResult<bool> {
        let hashed_codes: Vec<String> = profile
            .two_factor_backup_codes
            .as_ref()
            .and_then(|json| serde_json::from_value(json.clone()).ok())
            .unwrap_or_default();

        let argon2 = Argon2::default();

        for hash in &hashed_codes {
            if let Ok(parsed_hash) = argon2::password_hash::PasswordHash::new(hash) {
                if argon2
                    .verify_password(code.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn verify_and_consume_backup_code(
        &self,
        _user_id: &str,
        profile: &user_profile::Model,
        code: &str,
    ) -> AppResult<bool> {
        let mut hashed_codes: Vec<String> = profile
            .two_factor_backup_codes
            .as_ref()
            .and_then(|json| serde_json::from_value(json.clone()).ok())
            .unwrap_or_default();

        let argon2 = Argon2::default();

        let mut found_index = None;
        for (i, hash) in hashed_codes.iter().enumerate() {
            if let Ok(parsed_hash) = argon2::password_hash::PasswordHash::new(hash) {
                if argon2
                    .verify_password(code.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    found_index = Some(i);
                    break;
                }
            }
        }

        if let Some(index) = found_index {
            // Remove the used backup code
            hashed_codes.remove(index);

            // Update the profile
            let profile_clone = profile.clone();
            let mut active: user_profile::ActiveModel = profile_clone.into();
            active.two_factor_backup_codes = Set(Some(serde_json::json!(hashed_codes)));
            self.profile_repo.update(active).await?;

            return Ok(true);
        }

        Ok(false)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_totp_verification() {
        // This is a basic test to ensure the TOTP creation works
        let secret = Secret::generate_secret();
        let secret_bytes = secret.to_bytes().unwrap();

        let totp = TOTP::new(
            Algorithm::SHA1,
            TOTP_DIGITS,
            TOTP_SKEW,
            TOTP_STEP,
            secret_bytes.clone(),
            Some("TestIssuer".to_string()),
            "testuser".to_string(),
        )
        .unwrap();

        // Generate current token
        let token = totp.generate_current().unwrap();

        // Verify the token
        assert!(totp.check_current(&token).unwrap());
    }

    #[test]
    fn test_backup_code_generation() {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let code: String = (0..BACKUP_CODE_LENGTH)
            .map(|_| rng.gen_range(0..10).to_string())
            .collect();

        assert_eq!(code.len(), BACKUP_CODE_LENGTH);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }
}
