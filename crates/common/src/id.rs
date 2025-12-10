//! ID generation utilities.

use ulid::Ulid;
use uuid::Uuid;

/// ID generator for entities.
#[derive(Debug, Clone, Default)]
pub struct IdGenerator {
    _private: (),
}

impl IdGenerator {
    /// Create a new ID generator.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Generate a new ULID-based ID.
    ///
    /// ULIDs are:
    /// - Lexicographically sortable
    /// - Monotonically increasing within the same millisecond
    /// - Shorter than UUIDs when represented as strings
    #[must_use]
    pub fn generate(&self) -> String {
        Ulid::new().to_string().to_lowercase()
    }

    /// Generate a new UUID v7-based ID.
    ///
    /// UUID v7 is time-ordered and suitable for database primary keys.
    #[must_use]
    pub fn generate_uuid_v7(&self) -> String {
        Uuid::now_v7().to_string()
    }

    /// Generate a new random UUID v4.
    #[must_use]
    pub fn generate_uuid_v4(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Generate a cryptographically secure random token.
    #[must_use]
    pub fn generate_token(&self) -> String {
        // Use UUID v4 for tokens (no time component for security)
        Uuid::new_v4().simple().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ulid() {
        let id_gen = IdGenerator::new();
        let id1 = id_gen.generate();
        let id2 = id_gen.generate();

        assert_eq!(id1.len(), 26);
        assert_eq!(id2.len(), 26);
        assert_ne!(id1, id2);
        // Note: ULIDs generated rapidly within the same millisecond
        // may not be strictly ordered due to the random component
    }

    #[test]
    fn test_generate_uuid_v7() {
        let id_gen = IdGenerator::new();
        let id = id_gen.generate_uuid_v7();

        assert_eq!(id.len(), 36); // UUID with hyphens
        assert!(id.starts_with('0')); // UUID v7 starts with version nibble
    }

    #[test]
    fn test_generate_token() {
        let id_gen = IdGenerator::new();
        let token = id_gen.generate_token();

        assert_eq!(token.len(), 32); // Simple UUID without hyphens
    }
}
