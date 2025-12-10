//! HTTP Signature implementation for `ActivityPub`.
//!
//! Implements draft-cavage-http-signatures for signing and verifying
//! `ActivityPub` requests.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use pkcs8::{DecodePrivateKey, DecodePublicKey};
use reqwest::header::{HeaderMap, HeaderValue};
use rsa::{
    pkcs1v15::{SigningKey, VerifyingKey},
    RsaPrivateKey, RsaPublicKey,
};
use sha2::{Digest, Sha256};
use signature::{SignatureEncoding, Signer, Verifier};
use std::collections::HashMap;
use tracing::{debug, warn};
use url::Url;

/// HTTP Signature error.
#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Missing header: {0}")]
    MissingHeader(String),
    #[error("Invalid signature header")]
    InvalidSignatureHeader,
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Signature expired: clock skew too large")]
    ExpiredSignature,
    #[error("Duplicate activity detected (replay attack)")]
    DuplicateActivity,
    #[error("Invalid date header format")]
    InvalidDateFormat,
}

/// HTTP Signature signer for outgoing requests.
pub struct HttpSigner {
    private_key: RsaPrivateKey,
    key_id: String,
}

impl HttpSigner {
    /// Create a new HTTP signer from a PEM-encoded private key.
    pub fn new(private_key_pem: &str, key_id: String) -> Result<Self, SignatureError> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .map_err(|e| SignatureError::InvalidPrivateKey(e.to_string()))?;

        Ok(Self {
            private_key,
            key_id,
        })
    }

    /// Sign an HTTP request and return the signature headers.
    pub fn sign_request(
        &self,
        method: &str,
        url: &Url,
        body: Option<&[u8]>,
        additional_headers: &HashMap<String, String>,
    ) -> Result<HeaderMap, SignatureError> {
        let mut headers = HeaderMap::new();

        // Parse URL components
        let host = url
            .host_str()
            .ok_or_else(|| SignatureError::InvalidUrl("No host in URL".to_string()))?;
        let path = url.path();
        let query = url.query().map_or(String::new(), |q| format!("?{q}"));
        let request_target = format!("{} {path}{query}", method.to_lowercase());

        // Generate date header
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        // Calculate digest if body present
        let digest = body.map(|b| {
            let hash = Sha256::digest(b);
            format!("SHA-256={}", BASE64.encode(hash))
        });

        // Build headers to sign
        let mut signed_headers = vec!["(request-target)", "host", "date"];
        if digest.is_some() {
            signed_headers.push("digest");
        }

        // Add any additional headers
        for key in additional_headers.keys() {
            if !signed_headers.contains(&key.as_str()) {
                signed_headers.push(key.as_str());
            }
        }

        // Build signing string
        let mut signing_parts = Vec::new();
        for header in &signed_headers {
            let value = match *header {
                "(request-target)" => request_target.clone(),
                "host" => host.to_string(),
                "date" => date.clone(),
                "digest" => digest.clone().unwrap_or_default(),
                h => additional_headers
                    .get(h)
                    .cloned()
                    .unwrap_or_default(),
            };
            signing_parts.push(format!("{header}: {value}"));
        }
        let signing_string = signing_parts.join("\n");

        debug!(signing_string = %signing_string, "Signing string");

        // Sign the string
        let signing_key = SigningKey::<Sha256>::new(self.private_key.clone());
        let signature_bytes = signing_key
            .try_sign(signing_string.as_bytes())
            .map_err(|e| SignatureError::SigningFailed(e.to_string()))?;
        let signature = BASE64.encode(signature_bytes.to_bytes());

        // Build signature header
        let signature_header = format!(
            "keyId=\"{}\",algorithm=\"rsa-sha256\",headers=\"{}\",signature=\"{}\"",
            self.key_id,
            signed_headers.join(" "),
            signature
        );

        // Add all headers
        headers.insert("Host", HeaderValue::from_str(host).unwrap());
        headers.insert("Date", HeaderValue::from_str(&date).unwrap());
        if let Some(ref d) = digest {
            headers.insert("Digest", HeaderValue::from_str(d).unwrap());
        }
        headers.insert("Signature", HeaderValue::from_str(&signature_header).unwrap());

        // Add additional headers
        for (key, value) in additional_headers {
            if let Ok(v) = HeaderValue::from_str(value)
                && let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                    headers.insert(name, v);
                }
        }

        Ok(headers)
    }
}

/// HTTP Signature verifier for incoming requests.
pub struct HttpVerifier;

impl HttpVerifier {
    /// Parse the Signature header into components.
    pub fn parse_signature_header(header: &str) -> Result<SignatureComponents, SignatureError> {
        let mut key_id = None;
        let mut algorithm = None;
        let mut headers_list = None;
        let mut signature = None;

        // Parse key="value" pairs
        for part in header.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let value = value.trim_matches('"');
                match key {
                    "keyId" => key_id = Some(value.to_string()),
                    "algorithm" => algorithm = Some(value.to_string()),
                    "headers" => headers_list = Some(value.to_string()),
                    "signature" => signature = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Ok(SignatureComponents {
            key_id: key_id.ok_or(SignatureError::InvalidSignatureHeader)?,
            algorithm: algorithm.unwrap_or_else(|| "rsa-sha256".to_string()),
            headers: headers_list
                .unwrap_or_else(|| "date".to_string())
                .split(' ')
                .map(String::from)
                .collect(),
            signature: signature.ok_or(SignatureError::InvalidSignatureHeader)?,
        })
    }

    /// Verify an HTTP signature using the given public key.
    pub fn verify(
        public_key_pem: &str,
        components: &SignatureComponents,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
    ) -> Result<bool, SignatureError> {
        let public_key = RsaPublicKey::from_public_key_pem(public_key_pem)
            .map_err(|e| SignatureError::InvalidPublicKey(e.to_string()))?;

        // Build signing string from headers
        let mut signing_parts = Vec::new();
        for header in &components.headers {
            let value = match header.as_str() {
                "(request-target)" => format!("{} {path}", method.to_lowercase()),
                h => headers
                    .get(h)
                    .ok_or_else(|| SignatureError::MissingHeader(h.to_string()))?
                    .clone(),
            };
            signing_parts.push(format!("{header}: {value}"));
        }
        let signing_string = signing_parts.join("\n");

        debug!(signing_string = %signing_string, "Verifying signing string");

        // Decode and verify signature
        let signature_bytes = BASE64
            .decode(&components.signature)
            .map_err(|e| SignatureError::VerificationFailed(e.to_string()))?;

        let verifying_key = VerifyingKey::<Sha256>::new(public_key);
        let signature = rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
            .map_err(|e| SignatureError::VerificationFailed(e.to_string()))?;

        match verifying_key.verify(signing_string.as_bytes(), &signature) {
            Ok(()) => Ok(true),
            Err(e) => {
                warn!(error = %e, "Signature verification failed");
                Ok(false)
            }
        }
    }
}

/// Parsed signature header components.
#[derive(Debug, Clone)]
pub struct SignatureComponents {
    pub key_id: String,
    pub algorithm: String,
    pub headers: Vec<String>,
    pub signature: String,
}

/// Calculate SHA-256 digest of a body.
#[must_use] 
pub fn calculate_digest(body: &[u8]) -> String {
    let hash = Sha256::digest(body);
    format!("SHA-256={}", BASE64.encode(hash))
}

/// Verify that a digest header matches the body.
#[must_use] 
pub fn verify_digest(body: &[u8], digest_header: &str) -> bool {
    let expected = calculate_digest(body);
    expected == digest_header
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_keypair() -> (String, String) {
        use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};

        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public_key = RsaPublicKey::from(&private_key);

        let private_pem = private_key.to_pkcs8_pem(LineEnding::LF).unwrap();
        let public_pem = public_key.to_public_key_pem(LineEnding::LF).unwrap();

        (private_pem.to_string(), public_pem)
    }

    #[test]
    fn test_sign_and_verify() {
        let (private_pem, public_pem) = generate_test_keypair();

        let signer = HttpSigner::new(&private_pem, "https://example.com/users/test#main-key".to_string()).unwrap();

        let url = Url::parse("https://remote.example/inbox").unwrap();
        let body = b"{\"type\":\"Create\"}";

        let headers = signer
            .sign_request("POST", &url, Some(body), &HashMap::new())
            .unwrap();

        // Extract signature header
        let sig_header = headers.get("Signature").unwrap().to_str().unwrap();
        let components = HttpVerifier::parse_signature_header(sig_header).unwrap();

        // Build headers map for verification
        let mut verify_headers = HashMap::new();
        verify_headers.insert("host".to_string(), "remote.example".to_string());
        verify_headers.insert(
            "date".to_string(),
            headers.get("Date").unwrap().to_str().unwrap().to_string(),
        );
        verify_headers.insert(
            "digest".to_string(),
            headers.get("Digest").unwrap().to_str().unwrap().to_string(),
        );

        let result = HttpVerifier::verify(
            &public_pem,
            &components,
            "POST",
            "/inbox",
            &verify_headers,
        )
        .unwrap();

        assert!(result);
    }

    #[test]
    fn test_parse_signature_header() {
        let header = r#"keyId="https://example.com/users/test#main-key",algorithm="rsa-sha256",headers="(request-target) host date digest",signature="abc123==""#;

        let components = HttpVerifier::parse_signature_header(header).unwrap();

        assert_eq!(
            components.key_id,
            "https://example.com/users/test#main-key"
        );
        assert_eq!(components.algorithm, "rsa-sha256");
        assert_eq!(
            components.headers,
            vec!["(request-target)", "host", "date", "digest"]
        );
        assert_eq!(components.signature, "abc123==");
    }

    #[test]
    fn test_calculate_digest() {
        let body = b"hello world";
        let digest = calculate_digest(body);
        assert!(digest.starts_with("SHA-256="));
    }

    #[test]
    fn test_verify_digest() {
        let body = b"hello world";
        let digest = calculate_digest(body);
        assert!(verify_digest(body, &digest));
        assert!(!verify_digest(b"wrong body", &digest));
    }
}
