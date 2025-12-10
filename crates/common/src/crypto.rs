//! Cryptographic utilities for `ActivityPub` signatures.

use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};

use crate::{AppError, AppResult};

/// RSA key pair for `ActivityPub` HTTP Signatures.
#[derive(Debug, Clone)]
pub struct RsaKeypair {
    /// Public key in PEM format.
    pub public_key_pem: String,
    /// Private key in PEM format.
    pub private_key_pem: String,
}

/// Default RSA key size (2048 bits).
const RSA_KEY_SIZE: usize = 2048;

/// Generate a new RSA key pair.
///
/// # Returns
/// A new RSA key pair with public and private keys in PEM format.
///
/// # Errors
/// Returns an error if key generation fails.
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

/// Parse a private key from PEM format.
pub fn parse_private_key(pem: &str) -> AppResult<RsaPrivateKey> {
    RsaPrivateKey::from_pkcs8_pem(pem)
        .map_err(|e| AppError::Internal(format!("Failed to parse private key: {e}")))
}

/// Parse a public key from PEM format.
pub fn parse_public_key(pem: &str) -> AppResult<RsaPublicKey> {
    RsaPublicKey::from_public_key_pem(pem)
        .map_err(|e| AppError::Internal(format!("Failed to parse public key: {e}")))
}

#[cfg(test)]
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
