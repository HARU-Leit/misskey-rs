//! Database integration tests.
//!
//! These tests require a running `PostgreSQL` instance.
//! Run with: `cargo test --test db_integration -- --ignored`
//!
//! Setup test database:
//!   docker-compose -f docker-compose.test.yml up -d test-db
//!
//! Environment variables:
//!   `TEST_DB_HOST` (default: localhost)
//!   `TEST_DB_PORT` (default: 5433)
//!   `TEST_DB_USER` (default: `misskey_test`)
//!   `TEST_DB_PASSWORD` (default: `misskey_test`)
//!   `TEST_DB_NAME` (default: `misskey_test`)

#![allow(clippy::unwrap_used)]

use misskey_db::test_utils::{TestDatabase, TestDbConfig, TestRedisConfig};

#[tokio::test]
#[ignore = "requires running PostgreSQL instance"]
async fn test_database_connection() {
    let config = TestDbConfig::default();
    let result = TestDatabase::with_config(config).await;
    assert!(result.is_ok(), "Failed to connect: {:?}", result.err());
}

#[tokio::test]
#[ignore = "requires running PostgreSQL instance"]
async fn test_database_cleanup() {
    let db = TestDatabase::new().await.expect("Failed to connect");
    let result = db.cleanup().await;
    assert!(result.is_ok(), "Cleanup failed: {:?}", result.err());
}

#[tokio::test]
#[ignore = "requires running PostgreSQL instance"]
async fn test_execute_query() {
    let db = TestDatabase::new().await.expect("Failed to connect");

    // Connection should be valid
    use sea_orm::ConnectionTrait;
    let result = db
        .connection()
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1".to_string(),
        ))
        .await;

    assert!(result.is_ok(), "Query failed: {:?}", result.err());
}

#[test]
fn test_config_from_env() {
    // Test that default config is valid
    let config = TestDbConfig::default();
    assert!(!config.host.is_empty());
    assert!(config.port > 0);
    assert!(!config.username.is_empty());
    assert!(!config.database.is_empty());
}

#[test]
fn test_redis_config_from_env() {
    let config = TestRedisConfig::default();
    assert!(!config.host.is_empty());
    assert!(config.port > 0);
}

#[test]
fn test_database_url_format() {
    let config = TestDbConfig {
        host: "testhost".to_string(),
        port: 5432,
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        database: "testdb".to_string(),
    };

    let url = config.database_url();
    assert!(url.starts_with("postgres://"));
    assert!(url.contains("testhost"));
    assert!(url.contains("5432"));
    assert!(url.contains("testuser"));
    assert!(url.contains("testdb"));
}

#[test]
fn test_postgres_url_format() {
    let config = TestDbConfig::default();
    let url = config.postgres_url();
    assert!(url.ends_with("/postgres"));
}
