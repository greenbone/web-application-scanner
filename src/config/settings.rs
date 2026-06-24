// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use config::{Config, ConfigBuilder, ConfigError, Environment, builder::DefaultState};
use serde::Deserialize;
use std::path::Path;

/// Default log format (text format).
pub const DEFAULT_LOG_FORMAT: &str = "fmt";

/// Default log level (info messages and above).
pub const DEFAULT_LOG_LEVEL: &str = "info";

/// Default HTTP server port.
pub const DEFAULT_PORT: u16 = 8030;

/// Default storage backend (SQLite).
pub const DEFAULT_STORAGE_BACKEND: &str = "sqlite";

/// Default variable data directory.
pub const DEFAULT_VAR_DATA_DIR: &str = "/var/lib/greenbone-was";

/// Default SQLite database filename.
pub const DEFAULT_SQLITE_DATABASE_FILENAME: &str = "scans.db";

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

/// Default grace period added to scan-level AJAX spider timeout before forcing a stop request.
pub const DEFAULT_SCAN_AJAX_SPIDER_TIMEOUT_GRACE_PERIOD_SECONDS: u64 = 60;

/// Default time limit for waiting on phase status changes after stop requests.
pub const DEFAULT_SCAN_PHASE_STOP_STATUS_CHANGE_TIMEOUT_SECONDS: u64 = 60;

/// Default maximum number of retry attempts for transient failures.
pub const DEFAULT_SCAN_RETRY_MAX_RETRIES: u32 = 10;

/// Default maximum backoff delay between retries, in seconds.
pub const DEFAULT_SCAN_RETRY_MAX_DELAY_SECONDS: u64 = 60;

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
    /// Directory for variable runtime data, including the default SQLite database.
    pub var_data_dir: String,
    /// SQLite connection URL, either explicit or derived from `var_data_dir`.
    pub sqlite_url: Option<String>,
    /// Whether `sqlite_url` came from the explicit `GREENBONE_WAS_SQLITE_URL` override.
    pub sqlite_url_is_explicit: bool,
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
    /// Grace period in seconds added to scan-level AJAX spider timeout before issuing a stop.
    pub scan_ajax_spider_timeout_grace_period_seconds: u64,
    /// Time limit in seconds for waiting on scan phase status changes after stop requests.
    pub scan_phase_stop_status_change_timeout_seconds: u64,
    /// Maximum number of retry attempts for transient ZAP or storage failures.
    pub scan_retry_max_retries: u32,
    /// Maximum backoff delay between retry attempts, in seconds.
    pub scan_retry_max_delay_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSettings {
    log_format: String,
    log_level: String,
    port: u16,
    storage_backend: String,
    var_data_dir: String,
    sqlite_url: Option<String>,
    zap_base_url: String,
    zap_api_key: String,
    scan_worker_count: usize,
    scan_alert_poll_interval_seconds: u64,
    scan_stop_grace_period_seconds: u64,
    scan_ajax_spider_timeout_grace_period_seconds: u64,
    scan_phase_stop_status_change_timeout_seconds: u64,
    scan_retry_max_retries: u32,
    scan_retry_max_delay_seconds: u64,
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
            .set_default("var_data_dir", DEFAULT_VAR_DATA_DIR)?
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
            )?
            .set_default(
                "scan_ajax_spider_timeout_grace_period_seconds",
                DEFAULT_SCAN_AJAX_SPIDER_TIMEOUT_GRACE_PERIOD_SECONDS,
            )?
            .set_default(
                "scan_phase_stop_status_change_timeout_seconds",
                DEFAULT_SCAN_PHASE_STOP_STATUS_CHANGE_TIMEOUT_SECONDS,
            )?
            .set_default(
                "scan_retry_max_retries",
                DEFAULT_SCAN_RETRY_MAX_RETRIES as i64,
            )?
            .set_default(
                "scan_retry_max_delay_seconds",
                DEFAULT_SCAN_RETRY_MAX_DELAY_SECONDS,
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

        if raw.scan_phase_stop_status_change_timeout_seconds == 0 {
            return Err(ConfigError::Message(
                "scan_phase_stop_status_change_timeout_seconds must be greater than 0".to_string(),
            ));
        }

        if raw.scan_retry_max_delay_seconds == 0 {
            return Err(ConfigError::Message(
                "scan_retry_max_delay_seconds must be greater than 0".to_string(),
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

        let (sqlite_url, sqlite_url_is_explicit) = match raw.sqlite_url {
            Some(url) if url.is_empty() => {
                return Err(ConfigError::Message(
                    "GREENBONE_WAS_SQLITE_URL must not be empty".to_string(),
                ));
            }
            Some(url) if is_in_memory_sqlite_url(&url) => {
                return Err(ConfigError::Message(
                    "GREENBONE_WAS_SQLITE_URL must use a file-backed SQLite database; in-memory SQLite URLs are not supported for runtime configuration".to_string(),
                ));
            }
            Some(url) => (Some(url), true),
            None => (Some(default_sqlite_url(&raw.var_data_dir)), false),
        };

        Ok(Self {
            log_format: raw.log_format,
            log_level: raw.log_level,
            port: raw.port,
            storage_backend,
            var_data_dir: raw.var_data_dir,
            sqlite_url,
            sqlite_url_is_explicit,
            zap_base_url: raw.zap_base_url,
            zap_api_key: raw.zap_api_key,
            scan_worker_count: raw.scan_worker_count,
            scan_alert_poll_interval_seconds: raw.scan_alert_poll_interval_seconds,
            scan_stop_grace_period_seconds: raw.scan_stop_grace_period_seconds,
            scan_ajax_spider_timeout_grace_period_seconds: raw
                .scan_ajax_spider_timeout_grace_period_seconds,
            scan_phase_stop_status_change_timeout_seconds: raw
                .scan_phase_stop_status_change_timeout_seconds,
            scan_retry_max_retries: raw.scan_retry_max_retries,
            scan_retry_max_delay_seconds: raw.scan_retry_max_delay_seconds,
        })
    }
}

/// Build the default SQLite connection URL below `var_data_dir`.
pub fn default_sqlite_url(var_data_dir: &str) -> String {
    let db_path = Path::new(var_data_dir).join(DEFAULT_SQLITE_DATABASE_FILENAME);
    format!("sqlite:{}", db_path.display())
}

fn is_in_memory_sqlite_url(url: &str) -> bool {
    let lower_url = url.to_ascii_lowercase();

    lower_url == "sqlite::memory:"
        || lower_url
            .split(['?', '&'])
            .any(|part| part == "mode=memory")
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
