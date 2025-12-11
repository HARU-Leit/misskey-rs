//! Application configuration.

use serde::Deserialize;
use std::path::Path;

/// Application configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server configuration.
    pub server: ServerConfig,
    /// Database configuration.
    pub database: DatabaseConfig,
    /// Redis configuration.
    pub redis: RedisConfig,
    /// Federation configuration.
    pub federation: FederationConfig,
}

/// Server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Host to bind to.
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to bind to.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Public URL of this instance.
    pub url: String,
}

/// Database connection configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// `PostgreSQL` connection URL.
    pub url: String,
    /// Maximum number of connections in the pool.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Minimum number of connections in the pool.
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
}

/// Redis configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL.
    pub url: String,
    /// Key prefix for all Redis keys.
    #[serde(default = "default_redis_prefix")]
    pub prefix: String,
}

/// Federation configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct FederationConfig {
    /// Whether federation is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Instance name.
    pub instance_name: String,
    /// Instance description.
    #[serde(default)]
    pub instance_description: Option<String>,
    /// Instance maintainer name.
    #[serde(default)]
    pub maintainer_name: Option<String>,
    /// Instance maintainer email.
    #[serde(default)]
    pub maintainer_email: Option<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

const fn default_port() -> u16 {
    3000
}

const fn default_max_connections() -> u32 {
    100
}

const fn default_min_connections() -> u32 {
    5
}

fn default_redis_prefix() -> String {
    "misskey".to_string()
}

const fn default_true() -> bool {
    true
}

impl Config {
    /// Load configuration from files and environment variables.
    ///
    /// Configuration is loaded in the following order:
    /// 1. `config/default.toml`
    /// 2. `config/{environment}.toml` (based on `MISSKEY_ENV`)
    /// 3. Environment variables with `MISSKEY_` prefix
    pub fn load() -> Result<Self, config::ConfigError> {
        let env = std::env::var("MISSKEY_ENV").unwrap_or_else(|_| "development".to_string());

        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::File::with_name(&format!("config/{env}")).required(false))
            .add_source(
                config::Environment::with_prefix("MISSKEY")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }

    /// Load configuration from a specific file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .add_source(
                config::Environment::with_prefix("MISSKEY")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }
}
