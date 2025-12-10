//! Core business logic for misskey-rs.

pub mod services;

pub use services::*;

/// Generate a unique ID using ULID.
pub fn generate_id() -> String {
    ulid::Ulid::new().to_string().to_lowercase()
}
