//! Common utilities and shared types for misskey-rs.
//!
//! This crate provides foundational components used across all misskey-rs crates:
//!
//! - **Configuration**: Application settings via [`Config`]
//! - **Error handling**: Unified error types via [`AppError`] and [`AppResult`]
//! - **Cryptography**: RSA key generation for `ActivityPub` signatures
//! - **HTTP Signatures**: Implementation of HTTP Signatures for federation
//! - **ID Generation**: ULID-based unique identifiers via [`IdGenerator`]
//! - **Metrics**: Performance monitoring via [`Metrics`]
//! - **Storage**: File storage backends (local, S3-compatible)
//! - **URL Preview**: Link preview fetching for rich embeds
//! - **URL Preview Cache**: Redis-backed caching for URL previews
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
pub mod url_preview_cache;

pub use config::Config;
pub use crypto::{RsaKeypair, generate_rsa_keypair};
pub use error::{AppError, AppResult};
pub use http_signature::{
    HttpSignature, build_signature_string, calculate_digest, sign_request, verify_signature,
};
pub use id::IdGenerator;
pub use metrics::{Metrics, MetricsSnapshot, Timer, get_metrics};
pub use storage::{
    LocalStorage, StorageBackend, StorageConfig, UploadedFile, generate_storage_key,
};
pub use url_preview::{UrlPreview, UrlPreviewConfig, fetch_preview};
pub use url_preview_cache::{UrlPreviewCache, UrlPreviewCacheError};
