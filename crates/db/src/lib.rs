//! Database layer for misskey-rs.
//!
//! This crate provides the persistence layer using `SeaORM` with `PostgreSQL`:
//!
//! - **Entities**: Database models in [`entities`]
//! - **Migrations**: Schema migrations in [`migrations`]
//! - **Repositories**: Data access patterns in [`repositories`]
//! - **Test utilities**: Mock database support in [`test_utils`]
//!
//! # Example
//!
//! ```no_run
//! use misskey_db::{init, migrate};
//! use misskey_common::Config;
//!
//! async fn setup_database() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::load()?;
//!     let db = init(&config).await?;
//!     migrate(&db).await?;
//!     Ok(())
//! }
//! ```

pub mod entities;
pub mod migrations;
pub mod repositories;
pub mod test_utils;

use misskey_common::{AppError, Config};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::log::LevelFilter;

/// Initialize database connection.
pub async fn init(config: &Config) -> Result<DatabaseConnection, AppError> {
    let mut opt = ConnectOptions::new(&config.database.url);

    opt.max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(true)
        .sqlx_logging_level(LevelFilter::Debug);

    Database::connect(opt)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

/// Run pending migrations.
pub async fn migrate(db: &DatabaseConnection) -> Result<(), AppError> {
    use sea_orm_migration::MigratorTrait;
    migrations::Migrator::up(db, None)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}
