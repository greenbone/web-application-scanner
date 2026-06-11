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

/// Default storage backend (SQLite).
pub const DEFAULT_STORAGE_BACKEND: &str = "sqlite";

/// In-memory SQLite connection URL.
pub const SQLITE_IN_MEMORY_URL: &str = "sqlite::memory:";

/// Default SQLite connection URL.
pub const DEFAULT_SQLITE_URL: &str = SQLITE_IN_MEMORY_URL;

/// Default ZAP API base URL.
pub const DEFAULT_ZAP_BASE_URL: &str = "http://127.0.0.1:8547";

/// Default ZAP API key.
pub const DEFAULT_ZAP_API_KEY: &str = "test-api-key";

/// Default number of concurrent scan workers.
pub const DEFAULT_SCAN_WORKER_COUNT: usize = 1;

/// Default alert polling interval in seconds.
pub const DEFAULT_SCAN_ALERT_POLL_INTERVAL_SECONDS: u64 = 10;

/// Default stop grace period in seconds.
pub const DEFAULT_SCAN_STOP_GRACE_PERIOD_SECONDS: u64 = 300;

/// Runtime selection of the storage backend.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageBackend {
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
    /// Maximum number of concurrently running scan workers.
    pub scan_worker_count: usize,
    /// Interval in seconds between alert polling attempts during active scans.
    pub scan_alert_poll_interval_seconds: u64,
    /// Grace period in seconds to wait for running scans to stop before forcing failure.
    pub scan_stop_grace_period_seconds: u64,
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
    scan_worker_count: usize,
    scan_alert_poll_interval_seconds: u64,
    scan_stop_grace_period_seconds: u64,
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
            .set_default("sqlite_url", DEFAULT_SQLITE_URL)?
            .set_default("zap_base_url", DEFAULT_ZAP_BASE_URL)?
            .set_default("zap_api_key", DEFAULT_ZAP_API_KEY)?
            .set_default("scan_worker_count", DEFAULT_SCAN_WORKER_COUNT as i64)?
            .set_default(
                "scan_alert_poll_interval_seconds",
                DEFAULT_SCAN_ALERT_POLL_INTERVAL_SECONDS,
            )?
            .set_default(
                "scan_stop_grace_period_seconds",
                DEFAULT_SCAN_STOP_GRACE_PERIOD_SECONDS,
            )
    }

    /// Validate and convert raw settings into typed `Settings`.
    fn from_raw(raw: RawSettings) -> Result<Self, ConfigError> {
        if raw.port == 0 {
            return Err(ConfigError::Message(
                "port must be between 1 and 65535".to_string(),
            ));
        }

        if raw.scan_worker_count == 0 {
            return Err(ConfigError::Message(
                "scan_worker_count must be greater than 0".to_string(),
            ));
        }

        if raw.scan_alert_poll_interval_seconds == 0 {
            return Err(ConfigError::Message(
                "scan_alert_poll_interval_seconds must be greater than 0".to_string(),
            ));
        }

        if raw.scan_stop_grace_period_seconds == 0 {
            return Err(ConfigError::Message(
                "scan_stop_grace_period_seconds must be greater than 0".to_string(),
            ));
        }

        let storage_backend = match raw.storage_backend.as_str() {
            "sqlite" => StorageBackend::Sqlite,
            other => {
                return Err(ConfigError::Message(format!(
                    "unknown storage backend '{}'; valid value is 'sqlite'",
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
            scan_worker_count: raw.scan_worker_count,
            scan_alert_poll_interval_seconds: raw.scan_alert_poll_interval_seconds,
            scan_stop_grace_period_seconds: raw.scan_stop_grace_period_seconds,
        })
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
