//! User service.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use misskey_common::{generate_rsa_keypair, AppError, AppResult, Config, IdGenerator};
use misskey_db::{
    entities::{user, user_keypair, user_profile},
    repositories::{NoteRepository, UserKeypairRepository, UserProfileRepository, UserRepository},
};
use sea_orm::Set;
use serde::Deserialize;
use validator::Validate;

/// Maximum number of notes that can be pinned to a user's profile.
const MAX_PINNED_NOTES: usize = 5;

/// User service for business logic.
#[derive(Clone)]
pub struct UserService {
    user_repo: UserRepository,
    profile_repo: UserProfileRepository,
    keypair_repo: UserKeypairRepository,
    note_repo: NoteRepository,
    id_gen: IdGenerator,
    server_url: String,
}

/// Input for creating a new user.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserInput {
    #[validate(length(min = 1, max = 128))]
    pub username: String,

    #[validate(length(min = 8, max = 128))]
    pub password: String,

    #[validate(length(max = 256))]
    pub name: Option<String>,
}

/// Input for updating a user.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserInput {
    #[validate(length(max = 256))]
    pub name: Option<String>,

    #[validate(length(max = 2048))]
    pub description: Option<String>,

    /// Avatar file ID (resolved to URL by API layer).
    pub avatar_id: Option<String>,

    /// Banner file ID (resolved to URL by API layer).
    pub banner_id: Option<String>,

    /// Avatar URL (set by API layer after resolving `avatar_id`).
    pub avatar_url: Option<String>,

    /// Banner URL (set by API layer after resolving `banner_id`).
    pub banner_url: Option<String>,

    pub is_bot: Option<bool>,
    pub is_cat: Option<bool>,
    pub is_locked: Option<bool>,
}

impl UserService {
    /// Create a new user service.
    #[must_use]
    pub fn new(
        user_repo: UserRepository,
        profile_repo: UserProfileRepository,
        keypair_repo: UserKeypairRepository,
        note_repo: NoteRepository,
        config: &Config,
    ) -> Self {
        Self {
            user_repo,
            profile_repo,
            keypair_repo,
            note_repo,
            id_gen: IdGenerator::new(),
            server_url: config.server.url.clone(),
        }
    }

    /// Create a new local user.
    pub async fn create(&self, input: CreateUserInput) -> AppResult<user::Model> {
        input.validate()?;

        // Check if username is taken
        if self
            .user_repo
            .find_by_username_and_host(&input.username, None)
            .await?
            .is_some()
        {
            return Err(AppError::BadRequest("Username already taken".to_string()));
        }

        // Hash password
        let password_hash = hash_password(&input.password)?;

        // Generate token and user ID
        let user_id = self.id_gen.generate();
        let token = self.id_gen.generate_token();

        // Create user
        let user_model = user::ActiveModel {
            id: Set(user_id.clone()),
            username: Set(input.username.clone()),
            username_lower: Set(input.username.to_lowercase()),
            host: Set(None),
            token: Set(Some(token)),
            name: Set(input.name),
            ..Default::default()
        };

        let user = self.user_repo.create(user_model).await?;

        // Create user profile with password hash
        let profile_model = user_profile::ActiveModel {
            user_id: Set(user_id.clone()),
            password: Set(Some(password_hash)),
            pinned_page_ids: Set(serde_json::json!([])),
            pinned_note_ids: Set(serde_json::json!([])),
            fields: Set(serde_json::json!([])),
            muted_words: Set(serde_json::json!([])),
            ..Default::default()
        };

        self.profile_repo.create(profile_model).await?;

        // Generate RSA keypair for ActivityPub
        let keypair = generate_rsa_keypair()?;
        let key_id = format!("{}/users/{}#main-key", self.server_url, user_id);

        let keypair_model = user_keypair::ActiveModel {
            user_id: Set(user_id),
            public_key: Set(keypair.public_key_pem),
            private_key: Set(keypair.private_key_pem),
            key_id: Set(key_id),
            ..Default::default()
        };

        self.keypair_repo.create(keypair_model).await?;

        Ok(user)
    }

    /// Get a user by ID.
    pub async fn get(&self, id: &str) -> AppResult<user::Model> {
        self.user_repo.get_by_id(id).await
    }

    /// Get a user by username.
    pub async fn get_by_username(
        &self,
        username: &str,
        host: Option<&str>,
    ) -> AppResult<user::Model> {
        self.user_repo
            .find_by_username_and_host(username, host)
            .await?
            .ok_or_else(|| AppError::UserNotFound(username.to_string()))
    }

    /// Find a local user by username.
    pub async fn find_local_by_username(&self, username: &str) -> AppResult<Option<user::Model>> {
        self.user_repo.find_by_username_and_host(username, None).await
    }

    /// Authenticate a user by token.
    pub async fn authenticate_by_token(&self, token: &str) -> AppResult<user::Model> {
        self.user_repo
            .find_by_token(token)
            .await?
            .ok_or(AppError::Unauthorized)
    }

    /// Authenticate a user by username and password.
    pub async fn authenticate(&self, username: &str, password: &str) -> AppResult<user::Model> {
        // Find user by username (local users only)
        let user = self
            .user_repo
            .find_by_username_and_host(username, None)
            .await?
            .ok_or(AppError::Unauthorized)?;

        // Get user profile to check password
        let profile = self
            .profile_repo
            .find_by_user_id(&user.id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        // Verify password
        let password_hash = profile.password.ok_or(AppError::Unauthorized)?;
        if !verify_password(password, &password_hash)? {
            return Err(AppError::Unauthorized);
        }

        Ok(user)
    }

    /// Regenerate a user's authentication token.
    pub async fn regenerate_token(&self, user_id: &str) -> AppResult<String> {
        let user = self.user_repo.get_by_id(user_id).await?;
        let new_token = self.id_gen.generate_token();

        let mut active: user::ActiveModel = user.into();
        active.token = Set(Some(new_token.clone()));
        active.updated_at = Set(Some(chrono::Utc::now().into()));

        self.user_repo.update(active).await?;

        Ok(new_token)
    }

    /// Update a user.
    pub async fn update(&self, id: &str, input: UpdateUserInput) -> AppResult<user::Model> {
        input.validate()?;

        let user = self.user_repo.get_by_id(id).await?;
        let mut active: user::ActiveModel = user.into();

        if let Some(name) = input.name {
            active.name = Set(Some(name));
        }
        if let Some(description) = input.description {
            active.description = Set(Some(description));
        }
        if let Some(avatar_url) = input.avatar_url {
            active.avatar_url = Set(Some(avatar_url));
        }
        if let Some(banner_url) = input.banner_url {
            active.banner_url = Set(Some(banner_url));
        }
        if let Some(is_bot) = input.is_bot {
            active.is_bot = Set(is_bot);
        }
        if let Some(is_cat) = input.is_cat {
            active.is_cat = Set(is_cat);
        }
        if let Some(is_locked) = input.is_locked {
            active.is_locked = Set(is_locked);
        }

        active.updated_at = Set(Some(chrono::Utc::now().into()));

        self.user_repo.update(active).await
    }

    /// Search users by username or display name.
    pub async fn search_users(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
        local_only: bool,
    ) -> AppResult<Vec<user::Model>> {
        self.user_repo.search(query, limit, offset, local_only).await
    }

    /// Get pinned note IDs for a user.
    pub async fn get_pinned_note_ids(&self, user_id: &str) -> AppResult<Vec<String>> {
        self.profile_repo.get_pinned_note_ids(user_id).await
    }

    /// Pin a note to the user's profile.
    pub async fn pin_note(&self, user_id: &str, note_id: &str) -> AppResult<Vec<String>> {
        // Verify the note exists
        let note = self.note_repo.get_by_id(note_id).await?;

        // Verify the note belongs to the user
        if note.user_id != user_id {
            return Err(AppError::Forbidden(
                "You can only pin your own notes".to_string(),
            ));
        }

        // Verify visibility allows pinning (public or home)
        let visibility = format!("{:?}", note.visibility).to_lowercase();
        if visibility != "public" && visibility != "home" {
            return Err(AppError::BadRequest(
                "Only public or home notes can be pinned".to_string(),
            ));
        }

        // Pin the note
        self.profile_repo
            .pin_note(user_id, note_id, MAX_PINNED_NOTES)
            .await
    }

    /// Unpin a note from the user's profile.
    pub async fn unpin_note(&self, user_id: &str, note_id: &str) -> AppResult<Vec<String>> {
        self.profile_repo.unpin_note(user_id, note_id).await
    }

    /// Reorder pinned notes.
    pub async fn reorder_pinned_notes(
        &self,
        user_id: &str,
        note_ids: Vec<String>,
    ) -> AppResult<()> {
        self.profile_repo
            .reorder_pinned_notes(user_id, note_ids)
            .await
    }
}

/// Hash a password using Argon2.
fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {e}")))
}

/// Verify a password against a hash.
fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| AppError::Internal(format!("Invalid hash: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use misskey_common::config::{DatabaseConfig, FederationConfig, RedisConfig, ServerConfig};
    use sea_orm::{DatabaseBackend, MockDatabase};
    use std::sync::Arc;

    fn create_test_config() -> Config {
        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                url: "https://example.com".to_string(),
            },
            database: DatabaseConfig {
                url: "postgres://localhost/test".to_string(),
                max_connections: 10,
                min_connections: 1,
            },
            redis: RedisConfig {
                url: "redis://localhost".to_string(),
                prefix: "mk:".to_string(),
            },
            federation: FederationConfig {
                enabled: true,
                instance_name: "Test Instance".to_string(),
                instance_description: Some("A test instance".to_string()),
                maintainer_name: None,
                maintainer_email: None,
            },
        }
    }

    fn create_test_user(id: &str, username: &str) -> user::Model {
        user::Model {
            id: id.to_string(),
            username: username.to_string(),
            username_lower: username.to_lowercase(),
            host: None,
            name: Some("Test User".to_string()),
            description: None,
            avatar_url: None,
            banner_url: None,
            is_bot: false,
            is_cat: false,
            is_locked: false,
            is_suspended: false,
            is_silenced: false,
            is_admin: false,
            is_moderator: false,
            followers_count: 0,
            following_count: 0,
            notes_count: 0,
            inbox: None,
            shared_inbox: None,
            featured: None,
            uri: None,
            last_fetched_at: None,
            token: Some("test_token".to_string()),
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    // Unit tests for password functions
    #[test]
    fn test_hash_password() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        assert!(hash.starts_with("$argon2"));
        assert!(hash.len() > 50);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        let result = verify_password("wrong_password", &hash).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let result = verify_password("test", "invalid_hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_password_different_each_time() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Different salts should produce different hashes
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    fn create_test_service(user_db: Arc<sea_orm::DatabaseConnection>, profile_db: Arc<sea_orm::DatabaseConnection>, keypair_db: Arc<sea_orm::DatabaseConnection>, note_db: Arc<sea_orm::DatabaseConnection>) -> UserService {
        let user_repo = UserRepository::new(user_db);
        let profile_repo = UserProfileRepository::new(profile_db);
        let keypair_repo = UserKeypairRepository::new(keypair_db);
        let note_repo = NoteRepository::new(note_db);
        let config = create_test_config();
        UserService::new(user_repo, profile_repo, keypair_repo, note_repo, &config)
    }

    // Service tests
    #[tokio::test]
    async fn test_get_user_not_found() {
        let user_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<user::Model>::new()])
                .into_connection(),
        );
        let profile_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let keypair_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let service = create_test_service(user_db, profile_db, keypair_db, note_db);

        let result = service.get("nonexistent").await;
        assert!(result.is_err());
        match result {
            Err(AppError::UserNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected UserNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_by_username_not_found() {
        let user_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<user::Model>::new()])
                .into_connection(),
        );
        let profile_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let keypair_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let service = create_test_service(user_db, profile_db, keypair_db, note_db);

        let result = service.get_by_username("nobody", None).await;
        assert!(result.is_err());
        match result {
            Err(AppError::UserNotFound(name)) => assert_eq!(name, "nobody"),
            _ => panic!("Expected UserNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_authenticate_by_token_found() {
        let user = create_test_user("user1", "testuser");

        let user_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[user.clone()]])
                .into_connection(),
        );
        let profile_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let keypair_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let service = create_test_service(user_db, profile_db, keypair_db, note_db);

        let result = service.authenticate_by_token("test_token").await.unwrap();
        assert_eq!(result.id, "user1");
    }

    #[tokio::test]
    async fn test_authenticate_by_token_not_found() {
        let user_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<user::Model>::new()])
                .into_connection(),
        );
        let profile_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let keypair_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let service = create_test_service(user_db, profile_db, keypair_db, note_db);

        let result = service.authenticate_by_token("invalid").await;
        assert!(result.is_err());
        match result {
            Err(AppError::Unauthorized) => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[tokio::test]
    async fn test_create_user_input_validation() {
        // Test username too long
        let input = CreateUserInput {
            username: "a".repeat(200),
            password: "password123".to_string(),
            name: None,
        };
        assert!(input.validate().is_err());

        // Test password too short
        let input = CreateUserInput {
            username: "testuser".to_string(),
            password: "short".to_string(),
            name: None,
        };
        assert!(input.validate().is_err());

        // Test valid input
        let input = CreateUserInput {
            username: "testuser".to_string(),
            password: "password123".to_string(),
            name: Some("Test User".to_string()),
        };
        assert!(input.validate().is_ok());
    }

    #[tokio::test]
    async fn test_update_user_input_validation() {
        // Test description too long
        let input = UpdateUserInput {
            name: None,
            description: Some("a".repeat(3000)),
            avatar_id: None,
            banner_id: None,
            avatar_url: None,
            banner_url: None,
            is_bot: None,
            is_cat: None,
            is_locked: None,
        };
        assert!(input.validate().is_err());

        // Test valid input
        let input = UpdateUserInput {
            name: Some("New Name".to_string()),
            description: Some("Bio".to_string()),
            avatar_id: Some("file123".to_string()),
            banner_id: Some("file456".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            banner_url: Some("https://example.com/banner.png".to_string()),
            is_bot: Some(false),
            is_cat: Some(true),
            is_locked: Some(false),
        };
        assert!(input.validate().is_ok());
    }
}
