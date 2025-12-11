//! Test utilities for database operations.
//!
//! Provides helpers for setting up and tearing down test databases.

use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use tracing::info;

/// Test database configuration.
#[derive(Debug, Clone)]
pub struct TestDbConfig {
    /// Database host.
    pub host: String,
    /// Database port.
    pub port: u16,
    /// Database username.
    pub username: String,
    /// Database password.
    pub password: String,
    /// Database name.
    pub database: String,
}

impl Default for TestDbConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("TEST_DB_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5433),
            username: std::env::var("TEST_DB_USER").unwrap_or_else(|_| "misskey_test".to_string()),
            password: std::env::var("TEST_DB_PASSWORD")
                .unwrap_or_else(|_| "misskey_test".to_string()),
            database: std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "misskey_test".to_string()),
        }
    }
}

impl TestDbConfig {
    /// Get the database URL.
    #[must_use]
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }

    /// Get URL for connecting to postgres database (for creating test DB).
    #[must_use]
    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.username, self.password, self.host, self.port
        )
    }
}

/// A test database context that manages the lifecycle of a test database.
pub struct TestDatabase {
    /// Database connection.
    pub conn: DatabaseConnection,
    /// Database configuration.
    pub config: TestDbConfig,
    #[allow(dead_code)]
    cleanup_on_drop: bool,
}

impl TestDatabase {
    /// Create a new test database with a unique name.
    ///
    /// This connects to the postgres database, creates a new test database,
    /// runs migrations, and returns a connection to the test database.
    pub async fn new() -> Result<Self, DbErr> {
        let config = TestDbConfig::default();
        Self::with_config(config).await
    }

    /// Create a new test database with custom configuration.
    pub async fn with_config(config: TestDbConfig) -> Result<Self, DbErr> {
        let conn = Database::connect(&config.database_url()).await?;

        info!(database = %config.database, "Connected to test database");

        Ok(Self {
            conn,
            config,
            cleanup_on_drop: false, // Set to true to clean tables on drop
        })
    }

    /// Create a unique test database (for parallel tests).
    pub async fn create_unique() -> Result<Self, DbErr> {
        let mut config = TestDbConfig::default();
        let unique_suffix = uuid::Uuid::new_v4().to_string().replace('-', "_");
        config.database = format!("misskey_test_{}", &unique_suffix[..8]);

        // Connect to postgres to create the database
        let postgres_conn = Database::connect(&config.postgres_url()).await?;

        let create_db = format!("CREATE DATABASE \"{}\"", config.database);
        postgres_conn
            .execute(Statement::from_string(DatabaseBackend::Postgres, create_db))
            .await?;

        postgres_conn.close().await?;

        // Connect to the new database
        let conn = Database::connect(&config.database_url()).await?;

        info!(database = %config.database, "Created unique test database");

        Ok(Self {
            conn,
            config,
            cleanup_on_drop: true,
        })
    }

    /// Get the database connection.
    #[must_use]
    pub const fn connection(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// Clean up all data in the test database (truncate all tables).
    pub async fn cleanup(&self) -> Result<(), DbErr> {
        // Get all table names
        let tables = self
            .conn
            .query_all(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public'".to_string(),
            ))
            .await?;

        // Truncate each table
        for row in tables {
            if let Ok(table_name) = row.try_get::<String>("", "tablename") {
                // Skip migration table
                if table_name == "seaql_migrations" {
                    continue;
                }

                let truncate = format!("TRUNCATE TABLE \"{table_name}\" CASCADE");
                self.conn
                    .execute(Statement::from_string(DatabaseBackend::Postgres, truncate))
                    .await?;
            }
        }

        info!("Cleaned up test database");
        Ok(())
    }

    /// Drop the test database (for unique databases).
    /// Note: This consumes self because it needs to close the connection.
    pub async fn drop_database(self) -> Result<(), DbErr> {
        // Close the connection first
        self.conn.close().await?;

        // Connect to postgres to drop the database
        let postgres_conn = Database::connect(&self.config.postgres_url()).await?;

        // Terminate all connections to the database
        let terminate = format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            self.config.database
        );
        postgres_conn
            .execute(Statement::from_string(DatabaseBackend::Postgres, terminate))
            .await
            .ok(); // Ignore errors

        let drop_db = format!("DROP DATABASE IF EXISTS \"{}\"", self.config.database);
        postgres_conn
            .execute(Statement::from_string(DatabaseBackend::Postgres, drop_db))
            .await?;

        postgres_conn.close().await?;

        info!(database = %self.config.database, "Dropped test database");
        Ok(())
    }

    /// Run a test with automatic cleanup.
    ///
    /// Example:
    /// ```ignore
    /// TestDatabase::run_test(|db| async {
    ///     let conn = db.connection();
    ///     // use connection...
    ///     Ok(())
    /// }).await?;
    /// ```
    pub async fn run_test<F, Fut, T>(f: F) -> Result<T, DbErr>
    where
        F: for<'a> FnOnce(&'a Self) -> Fut,
        Fut: std::future::Future<Output = Result<T, DbErr>>,
    {
        let db = Self::new().await?;
        let result = f(&db).await;
        db.cleanup().await?;
        result
    }
}

/// Test Redis configuration.
#[derive(Debug, Clone)]
pub struct TestRedisConfig {
    /// Redis host.
    pub host: String,
    /// Redis port.
    pub port: u16,
}

impl Default for TestRedisConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("TEST_REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("TEST_REDIS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(6380),
        }
    }
}

impl TestRedisConfig {
    /// Get the Redis URL.
    #[must_use]
    pub fn redis_url(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_db_config_default() {
        let config = TestDbConfig::default();
        assert_eq!(config.port, 5433);
        assert_eq!(config.database, "misskey_test");
    }

    #[test]
    fn test_db_config_url() {
        let config = TestDbConfig {
            host: "localhost".to_string(),
            port: 5433,
            username: "user".to_string(),
            password: "pass".to_string(),
            database: "testdb".to_string(),
        };
        assert_eq!(
            config.database_url(),
            "postgres://user:pass@localhost:5433/testdb"
        );
    }

    #[test]
    fn test_redis_config_default() {
        let config = TestRedisConfig::default();
        assert_eq!(config.port, 6380);
    }

    #[test]
    fn test_redis_config_url() {
        let config = TestRedisConfig {
            host: "localhost".to_string(),
            port: 6380,
        };
        assert_eq!(config.redis_url(), "redis://localhost:6380");
    }
}
