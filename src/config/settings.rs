// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use config::{Config, ConfigBuilder, ConfigError, Environment, builder::DefaultState};
use serde::Deserialize;

/// Default log format (text format).
pub const DEFAULT_LOG_FORMAT: &str = "fmt";

/// Default log level (info messages and above).
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// Default HTTP server port.
pub const DEFAULT_PORT: u16 = 8030;

/// Default storage backend (in-memory).
pub const DEFAULT_STORAGE_BACKEND: &str = "inmemory";

/// Default ZAP API base URL.
pub const DEFAULT_ZAP_BASE_URL: &str = "http://127.0.0.1:8547";

/// Default ZAP API key.
pub const DEFAULT_ZAP_API_KEY: &str = "test-api-key";

/// Runtime selection of the storage backend.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageBackend {
    InMemory,
    Sqlite,
}

#[derive(Debug, Clone)]
pub struct Settings {
    /// Log format to use at runtime (e.g. "fmt" or "json").
    pub log_format: String,
    /// Log level to filter log messages at runtime (e.g. "info", "debug", "error").
    pub log_level: String,
    /// Port to listen on for incoming HTTP requests.
    pub port: u16,
    /// Which storage backend to use at runtime.
    pub storage_backend: StorageBackend,
    /// SQLite connection URL (e.g. `sqlite:scans.db`).
    /// Required when `storage_backend` is [`StorageBackend::Sqlite`].
    pub sqlite_url: Option<String>,
    /// Base URL for the ZAP HTTP API.
    pub zap_base_url: String,
    /// API key used for authenticated ZAP API calls.
    pub zap_api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSettings {
    log_format: String,
    log_level: String,
    port: u16,
    storage_backend: String,
    sqlite_url: String,
    zap_base_url: String,
    zap_api_key: String,
}

impl Settings {
    /// Load settings from environment variables and `.env` file.
    ///
    /// Environment variables prefixed with `GREENBONE_WAS_` override defaults.
    /// If no `.env` file exists, falls back to defaults and environment variables.
    pub fn load() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();

        let cfg: Config = Self::config_builder()?
            .add_source(Environment::with_prefix("GREENBONE_WAS"))
            .build()?;

        let raw_settings: RawSettings = cfg.try_deserialize::<RawSettings>()?;
        Self::from_raw(raw_settings)
    }

    /// Create a configuration builder with default values.
    fn config_builder() -> Result<ConfigBuilder<DefaultState>, ConfigError> {
        Config::builder()
            .set_default("log_format", "fmt")?
            .set_default("log_level", "info")?
            .set_default("port", 8030)?
            .set_default("storage_backend", DEFAULT_STORAGE_BACKEND)?
            .set_default("sqlite_url", "")?
            .set_default("zap_base_url", DEFAULT_ZAP_BASE_URL)?
            .set_default("zap_api_key", DEFAULT_ZAP_API_KEY)
    }

    /// Validate and convert raw settings into typed `Settings`.
    fn from_raw(raw: RawSettings) -> Result<Self, ConfigError> {
        if raw.port == 0 {
            return Err(ConfigError::Message(
                "port must be between 1 and 65535".to_string(),
            ));
        }

        let storage_backend = match raw.storage_backend.as_str() {
            "inmemory" => StorageBackend::InMemory,
            "sqlite" => StorageBackend::Sqlite,
            other => {
                return Err(ConfigError::Message(format!(
                    "unknown storage backend '{}'; valid values are 'inmemory' and 'sqlite'",
                    other
                )));
            }
        };

        let sqlite_url = if raw.sqlite_url.is_empty() {
            None
        } else {
            Some(raw.sqlite_url)
        };

        if storage_backend == StorageBackend::Sqlite && sqlite_url.is_none() {
            return Err(ConfigError::Message(
                "GREENBONE_WAS_SQLITE_URL is required when storage backend is 'sqlite'".to_string(),
            ));
        }

        Ok(Self {
            log_format: raw.log_format,
            log_level: raw.log_level,
            port: raw.port,
            storage_backend,
            sqlite_url,
            zap_base_url: raw.zap_base_url,
            zap_api_key: raw.zap_api_key,
        })
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
