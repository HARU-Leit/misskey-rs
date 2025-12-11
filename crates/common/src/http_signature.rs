//! HTTP Signature utilities for `ActivityPub`.
//!
//! Implements HTTP Signatures as used by `ActivityPub` for request authentication.
//! See: <https://datatracker.ietf.org/doc/html/draft-cavage-http-signatures>

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1v15::{Signature, SigningKey, VerifyingKey},
    pkcs8::DecodePublicKey,
    sha2::Sha256,
    signature::{SignatureEncoding, Signer, Verifier},
};
use sha2::{Digest, Sha256 as Sha256Hasher};
use std::collections::HashMap;

use crate::{AppError, AppResult};

/// Parsed HTTP Signature header.
#[derive(Debug, Clone)]
pub struct HttpSignature {
    /// Key ID (typically the actor's public key URL)
    pub key_id: String,
    /// Algorithm used (typically "rsa-sha256")
    pub algorithm: String,
    /// Headers included in the signature
    pub headers: Vec<String>,
    /// The signature itself (base64 encoded)
    pub signature: String,
}

impl HttpSignature {
    /// Parse an HTTP Signature header value.
    ///
    /// Format: `keyId="...",algorithm="...",headers="...",signature="..."`
    pub fn parse(header: &str) -> AppResult<Self> {
        let mut key_id = None;
        let mut algorithm = None;
        let mut headers = None;
        let mut signature = None;

        // Parse key="value" pairs
        for part in header.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let value = value.trim_matches('"');
                match key.trim() {
                    "keyId" => key_id = Some(value.to_string()),
                    "algorithm" => algorithm = Some(value.to_string()),
                    "headers" => headers = Some(value.to_string()),
                    "signature" => signature = Some(value.to_string()),
                    _ => {} // Ignore unknown fields
                }
            }
        }

        Ok(Self {
            key_id: key_id.ok_or_else(|| AppError::BadRequest("Missing keyId".to_string()))?,
            algorithm: algorithm.unwrap_or_else(|| "rsa-sha256".to_string()),
            headers: headers
                .unwrap_or_else(|| "date".to_string())
                .split(' ')
                .map(std::string::ToString::to_string)
                .collect(),
            signature: signature
                .ok_or_else(|| AppError::BadRequest("Missing signature".to_string()))?,
        })
    }
}

/// Build the signature string from request components.
///
/// This creates the string that needs to be signed/verified.
pub fn build_signature_string(
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    signed_headers: &[String],
) -> AppResult<String> {
    let mut parts = Vec::new();

    for header_name in signed_headers {
        let value = if header_name == "(request-target)" {
            format!("{} {}", method.to_lowercase(), path)
        } else {
            headers
                .get(&header_name.to_lowercase())
                .cloned()
                .ok_or_else(|| AppError::BadRequest(format!("Missing header: {header_name}")))?
        };

        parts.push(format!("{header_name}: {value}"));
    }

    Ok(parts.join("\n"))
}

/// Verify an HTTP Signature.
///
/// # Arguments
/// * `signature` - The parsed HTTP signature
/// * `public_key_pem` - The public key in PEM format
/// * `method` - HTTP method (GET, POST, etc.)
/// * `path` - Request path
/// * `headers` - Request headers
pub fn verify_signature(
    signature: &HttpSignature,
    public_key_pem: &str,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
) -> AppResult<bool> {
    // Build the signature string
    let sig_string = build_signature_string(method, path, headers, &signature.headers)?;

    // Decode the signature
    let sig_bytes = BASE64
        .decode(&signature.signature)
        .map_err(|e| AppError::BadRequest(format!("Invalid signature encoding: {e}")))?;

    // Parse the public key
    let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)
        .map_err(|e| AppError::Internal(format!("Invalid public key: {e}")))?;

    // Verify the signature
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    let signature_obj = Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| AppError::BadRequest(format!("Invalid signature format: {e}")))?;

    Ok(verifying_key
        .verify(sig_string.as_bytes(), &signature_obj)
        .is_ok())
}

/// Sign an HTTP request.
///
/// # Arguments
/// * `private_key` - The RSA private key
/// * `key_id` - The key ID (public key URL)
/// * `method` - HTTP method
/// * `path` - Request path
/// * `headers` - Headers to include in signature
pub fn sign_request(
    private_key: &RsaPrivateKey,
    key_id: &str,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    signed_header_names: &[&str],
) -> AppResult<String> {
    // Build signature string
    let header_names: Vec<String> = signed_header_names
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let sig_string = build_signature_string(method, path, headers, &header_names)?;

    // Sign
    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let signature = signing_key.sign(sig_string.as_bytes());
    let sig_base64 = BASE64.encode(signature.to_bytes());

    // Build header value
    Ok(format!(
        r#"keyId="{}",algorithm="rsa-sha256",headers="{}",signature="{}""#,
        key_id,
        signed_header_names.join(" "),
        sig_base64
    ))
}

/// Calculate SHA-256 digest of a body.
#[must_use]
pub fn calculate_digest(body: &[u8]) -> String {
    let mut hasher = Sha256Hasher::new();
    hasher.update(body);
    let hash = hasher.finalize();
    format!("SHA-256={}", BASE64.encode(hash))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::generate_rsa_keypair;

    #[test]
    fn test_parse_signature_header() {
        let header = r#"keyId="https://example.com/users/test#main-key",algorithm="rsa-sha256",headers="(request-target) host date digest",signature="abc123==""#;

        let sig = HttpSignature::parse(header).unwrap();

        assert_eq!(sig.key_id, "https://example.com/users/test#main-key");
        assert_eq!(sig.algorithm, "rsa-sha256");
        assert_eq!(
            sig.headers,
            vec!["(request-target)", "host", "date", "digest"]
        );
        assert_eq!(sig.signature, "abc123==");
    }

    #[test]
    fn test_build_signature_string() {
        let mut headers = HashMap::new();
        headers.insert("host".to_string(), "example.com".to_string());
        headers.insert(
            "date".to_string(),
            "Sun, 06 Nov 1994 08:49:37 GMT".to_string(),
        );

        let signed_headers = vec![
            "(request-target)".to_string(),
            "host".to_string(),
            "date".to_string(),
        ];

        let sig_string =
            build_signature_string("POST", "/inbox", &headers, &signed_headers).unwrap();

        assert!(sig_string.contains("(request-target): post /inbox"));
        assert!(sig_string.contains("host: example.com"));
        assert!(sig_string.contains("date: Sun, 06 Nov 1994 08:49:37 GMT"));
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = generate_rsa_keypair().unwrap();
        let private_key = crate::crypto::parse_private_key(&keypair.private_key_pem).unwrap();

        let mut headers = HashMap::new();
        headers.insert("host".to_string(), "example.com".to_string());
        headers.insert(
            "date".to_string(),
            "Sun, 06 Nov 1994 08:49:37 GMT".to_string(),
        );

        let signed_header_names = &["(request-target)", "host", "date"];

        // Sign
        let sig_header = sign_request(
            &private_key,
            "https://example.com/users/test#main-key",
            "POST",
            "/inbox",
            &headers,
            signed_header_names,
        )
        .unwrap();

        // Parse and verify
        let parsed_sig = HttpSignature::parse(&sig_header).unwrap();
        let is_valid = verify_signature(
            &parsed_sig,
            &keypair.public_key_pem,
            "POST",
            "/inbox",
            &headers,
        )
        .unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_calculate_digest() {
        let body = b"hello world";
        let digest = calculate_digest(body);

        assert!(digest.starts_with("SHA-256="));
    }
}
