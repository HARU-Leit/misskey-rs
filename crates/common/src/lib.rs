//! Common utilities and shared types for misskey-rs.
//!
//! This crate provides foundational components used across all misskey-rs crates:
//!
//! - **Configuration**: Application settings via [`Config`]
//! - **Error handling**: Unified error types via [`AppError`] and [`AppResult`]
//! - **Cryptography**: RSA key generation for ActivityPub signatures
//! - **HTTP Signatures**: Implementation of HTTP Signatures for federation
//! - **ID Generation**: ULID-based unique identifiers via [`IdGenerator`]
//! - **Metrics**: Performance monitoring via [`Metrics`]
//! - **Storage**: File storage backends (local, S3-compatible)
//! - **URL Preview**: Link preview fetching for rich embeds
//!
//! # Example
//!
//! ```no_run
//! use misskey_common::{Config, IdGenerator, AppResult};
//!
//! fn example() -> AppResult<()> {
//!     let config = Config::load()?;
//!     let id_gen = IdGenerator::new();
//!     let id = id_gen.generate();
//!     println!("Generated ID: {}", id);
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod crypto;
pub mod error;
pub mod http_signature;
pub mod id;
pub mod metrics;
pub mod storage;
pub mod url_preview;

pub use config::Config;
pub use crypto::{generate_rsa_keypair, RsaKeypair};
pub use error::{AppError, AppResult};
pub use http_signature::{
    build_signature_string, calculate_digest, sign_request, verify_signature, HttpSignature,
};
pub use id::IdGenerator;
pub use metrics::{get_metrics, Metrics, MetricsSnapshot, Timer};
pub use storage::{generate_storage_key, LocalStorage, StorageBackend, StorageConfig, UploadedFile};
pub use url_preview::{fetch_preview, UrlPreview, UrlPreviewConfig};
