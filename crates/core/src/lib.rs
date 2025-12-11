//! Core business logic for misskey-rs.
//!
//! This crate contains the domain services implementing Misskey's core functionality:
//!
//! - **User management**: Registration, profiles, follows
//! - **Notes**: Creation, deletion, timelines
//! - **Reactions**: Emoji reactions on notes
//! - **Notifications**: User notifications
//! - **Messaging**: Direct messages
//! - **Drive**: File management
//! - **Moderation**: User reports, blocking, muting
//! - **ActivityPub delivery**: Federation activity dispatch
//!
//! All services are designed for dependency injection and support both
//! real database connections and mock implementations for testing.

pub mod services;

pub use services::*;

/// Generate a unique ID using ULID.
pub fn generate_id() -> String {
    ulid::Ulid::new().to_string().to_lowercase()
}
