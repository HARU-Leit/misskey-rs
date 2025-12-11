//! Cryptographic utilities for `ActivityPub` signatures.
//!
//! This module provides RSA key generation and parsing utilities used for
//! HTTP Signatures in `ActivityPub` federation.
//!
//! # Examples
//!
//! ```
//! use misskey_common::crypto::{generate_rsa_keypair, parse_private_key, parse_public_key};
//!
//! // Generate a new key pair
//! let keypair = generate_rsa_keypair().expect("Failed to generate keypair");
//!
//! // The keys are in PEM format
//! assert!(keypair.public_key_pem.contains("BEGIN PUBLIC KEY"));
//! assert!(keypair.private_key_pem.contains("BEGIN PRIVATE KEY"));
//!
//! // Parse the keys back
//! let _private = parse_private_key(&keypair.private_key_pem).expect("Failed to parse");
//! let _public = parse_public_key(&keypair.public_key_pem).expect("Failed to parse");
//! ```

use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding},
};

use crate::{AppError, AppResult};

/// RSA key pair for `ActivityPub` HTTP Signatures.
///
/// Contains both public and private keys in PEM format, suitable for
/// signing and verifying HTTP requests according to the HTTP Signatures
/// specification used by `ActivityPub`.
#[derive(Debug, Clone)]
pub struct RsaKeypair {
    /// Public key in PEM format (SPKI encoding).
    pub public_key_pem: String,
    /// Private key in PEM format (PKCS#8 encoding).
    pub private_key_pem: String,
}

/// Default RSA key size (2048 bits).
const RSA_KEY_SIZE: usize = 2048;

/// Generate a new RSA key pair for `ActivityPub` HTTP Signatures.
///
/// Creates a 2048-bit RSA key pair and returns both keys in PEM format.
/// The private key uses PKCS#8 encoding and the public key uses SPKI encoding.
///
/// # Examples
///
/// ```
/// use misskey_common::crypto::generate_rsa_keypair;
///
/// let keypair = generate_rsa_keypair().expect("Failed to generate keypair");
///
/// // Keys are ready to use for HTTP Signatures
/// assert!(keypair.public_key_pem.starts_with("-----BEGIN PUBLIC KEY-----"));
/// assert!(keypair.private_key_pem.starts_with("-----BEGIN PRIVATE KEY-----"));
/// ```
///
/// # Errors
///
/// Returns [`AppError::Internal`] if:
/// - RSA key generation fails (e.g., insufficient randomness)
/// - PEM encoding fails (should not happen with valid keys)
///
/// # Panics
///
/// This function does not panic under normal circumstances.
pub fn generate_rsa_keypair() -> AppResult<RsaKeypair> {
    let mut rng = rand::thread_rng();

    let private_key = RsaPrivateKey::new(&mut rng, RSA_KEY_SIZE)
        .map_err(|e| AppError::Internal(format!("Failed to generate RSA key: {e}")))?;

    let public_key = RsaPublicKey::from(&private_key);

    let private_key_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode private key: {e}")))?
        .to_string();

    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode public key: {e}")))?;

    Ok(RsaKeypair {
        public_key_pem,
        private_key_pem,
    })
}

/// Parse an RSA private key from PEM format.
///
/// Parses a PKCS#8 encoded private key in PEM format.
///
/// # Examples
///
/// ```
/// use misskey_common::crypto::{generate_rsa_keypair, parse_private_key};
///
/// let keypair = generate_rsa_keypair().expect("Failed to generate");
/// let private_key = parse_private_key(&keypair.private_key_pem)
///     .expect("Failed to parse private key");
/// ```
///
/// # Errors
///
/// Returns [`AppError::Internal`] if:
/// - The PEM format is invalid
/// - The key is not a valid PKCS#8 encoded RSA private key
pub fn parse_private_key(pem: &str) -> AppResult<RsaPrivateKey> {
    RsaPrivateKey::from_pkcs8_pem(pem)
        .map_err(|e| AppError::Internal(format!("Failed to parse private key: {e}")))
}

/// Parse an RSA public key from PEM format.
///
/// Parses a SPKI encoded public key in PEM format.
///
/// # Examples
///
/// ```
/// use misskey_common::crypto::{generate_rsa_keypair, parse_public_key};
///
/// let keypair = generate_rsa_keypair().expect("Failed to generate");
/// let public_key = parse_public_key(&keypair.public_key_pem)
///     .expect("Failed to parse public key");
/// ```
///
/// # Errors
///
/// Returns [`AppError::Internal`] if:
/// - The PEM format is invalid
/// - The key is not a valid SPKI encoded RSA public key
pub fn parse_public_key(pem: &str) -> AppResult<RsaPublicKey> {
    RsaPublicKey::from_public_key_pem(pem)
        .map_err(|e| AppError::Internal(format!("Failed to parse public key: {e}")))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = generate_rsa_keypair().unwrap();

        assert!(keypair.public_key_pem.contains("BEGIN PUBLIC KEY"));
        assert!(keypair.public_key_pem.contains("END PUBLIC KEY"));
        assert!(keypair.private_key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(keypair.private_key_pem.contains("END PRIVATE KEY"));
    }

    #[test]
    fn test_parse_generated_keys() {
        let keypair = generate_rsa_keypair().unwrap();

        // Should be able to parse the generated keys
        let _private = parse_private_key(&keypair.private_key_pem).unwrap();
        let _public = parse_public_key(&keypair.public_key_pem).unwrap();
    }
}
