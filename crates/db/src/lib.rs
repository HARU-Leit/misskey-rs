//! Database layer for misskey-rs.
//!
//! This crate provides the persistence layer using `SeaORM` with `PostgreSQL`:
//!
//! - **Entities**: Database models in [`entities`]
//! - **Migrations**: Schema migrations in [`migrations`]
//! - **Repositories**: Data access patterns in [`repositories`]
//! - **Test utilities**: Mock database support in [`test_utils`]
//! - **Read Replicas**: Automatic read/write splitting via [`DatabasePool`]
//!
//! # Example
//!
//! ```no_run
//! use misskey_db::{init, init_pool, migrate};
//! use misskey_common::Config;
//!
//! async fn setup_database() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::load()?;
//!     // Simple setup (single connection)
//!     let db = init(&config).await?;
//!     migrate(&db).await?;
//!
//!     // Advanced setup (with read replicas)
//!     let pool = init_pool(&config).await?;
//!     let writer = pool.writer();
//!     let reader = pool.reader(); // Automatically load-balanced across replicas
//!     Ok(())
//! }
//! ```

pub mod entities;
pub mod migrations;
pub mod repositories;
pub mod test_utils;

use misskey_common::{AppError, Config};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tracing::{info, log::LevelFilter, warn};

/// Database pool with optional read replica support.
///
/// When read replicas are configured, read operations can be distributed
/// across replicas using round-robin load balancing, while write operations
/// always go to the primary connection.
///
/// This struct is cheaply clonable via `Arc`.
#[derive(Clone)]
pub struct DatabasePool {
    inner: Arc<DatabasePoolInner>,
}

/// Inner state of the database pool.
struct DatabasePoolInner {
    /// Primary connection (for writes and reads when no replicas)
    primary: DatabaseConnection,
    /// Read replica connections (empty if no replicas configured)
    replicas: Vec<DatabaseConnection>,
    /// Round-robin counter for replica selection
    replica_counter: AtomicUsize,
}

impl DatabasePool {
    /// Create a new database pool with only a primary connection.
    #[must_use]
    pub fn new(primary: DatabaseConnection) -> Self {
        Self {
            inner: Arc::new(DatabasePoolInner {
                primary,
                replicas: Vec::new(),
                replica_counter: AtomicUsize::new(0),
            }),
        }
    }

    /// Create a new database pool with read replicas.
    #[must_use]
    pub fn with_replicas(primary: DatabaseConnection, replicas: Vec<DatabaseConnection>) -> Self {
        Self {
            inner: Arc::new(DatabasePoolInner {
                primary,
                replicas,
                replica_counter: AtomicUsize::new(0),
            }),
        }
    }

    /// Get the writer connection (always the primary).
    ///
    /// Use this for INSERT, UPDATE, DELETE operations and any queries
    /// that require strong consistency.
    #[must_use]
    pub fn writer(&self) -> &DatabaseConnection {
        &self.inner.primary
    }

    /// Get a reader connection.
    ///
    /// If read replicas are configured, returns one using round-robin.
    /// Otherwise, returns the primary connection.
    ///
    /// Use this for SELECT queries that can tolerate slight replication lag.
    #[must_use]
    pub fn reader(&self) -> &DatabaseConnection {
        if self.inner.replicas.is_empty() {
            return &self.inner.primary;
        }

        let index =
            self.inner.replica_counter.fetch_add(1, Ordering::Relaxed) % self.inner.replicas.len();
        &self.inner.replicas[index]
    }

    /// Get the primary connection directly.
    ///
    /// Alias for `writer()` for backward compatibility.
    #[must_use]
    pub fn primary(&self) -> &DatabaseConnection {
        &self.inner.primary
    }

    /// Check if read replicas are configured.
    #[must_use]
    pub fn has_replicas(&self) -> bool {
        !self.inner.replicas.is_empty()
    }

    /// Get the number of read replicas.
    #[must_use]
    pub fn replica_count(&self) -> usize {
        self.inner.replicas.len()
    }

    /// Get all replica connections (for health checks, etc.).
    #[must_use]
    pub fn replicas(&self) -> &[DatabaseConnection] {
        &self.inner.replicas
    }
}

/// Create connection options with standard settings.
fn create_connect_options(url: &str, max_conns: u32, min_conns: u32) -> ConnectOptions {
    let mut opt = ConnectOptions::new(url);
    opt.max_connections(max_conns)
        .min_connections(min_conns)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(true)
        .sqlx_logging_level(LevelFilter::Debug);
    opt
}

/// Initialize database connection (single primary).
///
/// For read replica support, use [`init_pool`] instead.
pub async fn init(config: &Config) -> Result<DatabaseConnection, AppError> {
    let opt = create_connect_options(
        &config.database.url,
        config.database.max_connections,
        config.database.min_connections,
    );

    Database::connect(opt)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

/// Initialize database pool with optional read replicas.
///
/// If `read_replicas` is configured in the database config, connections
/// to those replicas will be established. Failed replica connections are
/// logged as warnings but don't prevent startup.
pub async fn init_pool(config: &Config) -> Result<DatabasePool, AppError> {
    // Connect to primary
    let primary_opt = create_connect_options(
        &config.database.url,
        config.database.max_connections,
        config.database.min_connections,
    );

    let primary = Database::connect(primary_opt)
        .await
        .map_err(|e| AppError::Database(format!("Failed to connect to primary: {e}")))?;

    info!("Connected to primary database");

    // Connect to replicas (if configured)
    let mut replicas = Vec::new();
    let replica_urls = &config.database.read_replicas;

    if !replica_urls.is_empty() {
        // Distribute connections evenly across replicas
        let conns_per_replica = config.database.max_connections / (replica_urls.len() as u32 + 1);
        let min_per_replica = config.database.min_connections / (replica_urls.len() as u32 + 1);

        for (i, url) in replica_urls.iter().enumerate() {
            let replica_opt =
                create_connect_options(url, conns_per_replica.max(1), min_per_replica.max(1));

            match Database::connect(replica_opt).await {
                Ok(conn) => {
                    info!(replica = i, "Connected to read replica");
                    replicas.push(conn);
                }
                Err(e) => {
                    warn!(replica = i, error = %e, "Failed to connect to read replica, skipping");
                }
            }
        }

        if replicas.is_empty() {
            warn!("No read replicas connected, all reads will use primary");
        } else {
            info!(count = replicas.len(), "Read replicas initialized");
        }
    }

    Ok(DatabasePool::with_replicas(primary, replicas))
}

/// Run pending migrations (always on primary).
pub async fn migrate(db: &DatabaseConnection) -> Result<(), AppError> {
    use sea_orm_migration::MigratorTrait;
    migrations::Migrator::up(db, None)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

/// Run pending migrations on a database pool.
///
/// Migrations always run on the primary/writer connection.
pub async fn migrate_pool(pool: &DatabasePool) -> Result<(), AppError> {
    migrate(pool.writer()).await
}
