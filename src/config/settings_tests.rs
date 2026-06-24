// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::config::settings;
use serial_test::serial;
use std::env;

use super::{Settings, StorageBackend};

fn clear_env() {
    unsafe {
        env::remove_var("GREENBONE_WAS_LOG_FORMAT");
        env::remove_var("GREENBONE_WAS_LOG_LEVEL");
        env::remove_var("GREENBONE_WAS_PORT");
        env::remove_var("GREENBONE_WAS_STORAGE_BACKEND");
        env::remove_var("GREENBONE_WAS_VAR_DATA_DIR");
        env::remove_var("GREENBONE_WAS_SQLITE_URL");
        env::remove_var("GREENBONE_WAS_ZAP_BASE_URL");
        env::remove_var("GREENBONE_WAS_ZAP_API_KEY");
        env::remove_var("GREENBONE_WAS_SCAN_WORKER_COUNT");
        env::remove_var("GREENBONE_WAS_SCAN_ALERT_POLL_INTERVAL_SECONDS");
        env::remove_var("GREENBONE_WAS_SCAN_STOP_GRACE_PERIOD_SECONDS");
        env::remove_var("GREENBONE_WAS_SCAN_AJAX_SPIDER_TIMEOUT_GRACE_PERIOD_SECONDS");
        env::remove_var("GREENBONE_WAS_SCAN_RETRY_MAX_RETRIES");
        env::remove_var("GREENBONE_WAS_SCAN_RETRY_MAX_DELAY_SECONDS");
    }
}

#[test]
#[serial]
fn test_uses_defaults_when_env_is_unset() {
    clear_env();

    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.log_format, settings::DEFAULT_LOG_FORMAT);
    assert_eq!(settings.log_level, settings::DEFAULT_LOG_LEVEL);
    assert_eq!(settings.port, settings::DEFAULT_PORT);
    assert_eq!(settings.storage_backend, StorageBackend::Sqlite);
    assert_eq!(settings.var_data_dir, settings::DEFAULT_VAR_DATA_DIR);
    assert_eq!(
        settings.sqlite_url.as_deref(),
        Some(settings::default_sqlite_url(settings::DEFAULT_VAR_DATA_DIR).as_str())
    );
    assert!(!settings.sqlite_url_is_explicit);
    assert_eq!(settings.zap_base_url, settings::DEFAULT_ZAP_BASE_URL);
    assert_eq!(settings.zap_api_key, settings::DEFAULT_ZAP_API_KEY);
    assert_eq!(
        settings.scan_worker_count,
        settings::DEFAULT_SCAN_WORKER_COUNT
    );
    assert_eq!(
        settings.scan_alert_poll_interval_seconds,
        settings::DEFAULT_SCAN_ALERT_POLL_INTERVAL_SECONDS
    );
    assert_eq!(
        settings.scan_stop_grace_period_seconds,
        settings::DEFAULT_SCAN_STOP_GRACE_PERIOD_SECONDS
    );
    assert_eq!(
        settings.scan_ajax_spider_timeout_grace_period_seconds,
        settings::DEFAULT_SCAN_AJAX_SPIDER_TIMEOUT_GRACE_PERIOD_SECONDS
    );
    assert_eq!(
        settings.scan_retry_max_retries,
        settings::DEFAULT_SCAN_RETRY_MAX_RETRIES
    );
    assert_eq!(
        settings.scan_retry_max_delay_seconds,
        settings::DEFAULT_SCAN_RETRY_MAX_DELAY_SECONDS
    );
}

#[test]
#[serial]
fn test_uses_env_overrides_when_set() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_LOG_FORMAT", "json");
        env::set_var("GREENBONE_WAS_LOG_LEVEL", "debug");
        env::set_var("GREENBONE_WAS_PORT", "8080");
        env::set_var("GREENBONE_WAS_STORAGE_BACKEND", "sqlite");
        env::set_var("GREENBONE_WAS_VAR_DATA_DIR", "/tmp/greenbone-was");
        env::set_var("GREENBONE_WAS_SQLITE_URL", "sqlite:scans.db");
        env::set_var("GREENBONE_WAS_ZAP_BASE_URL", "http://127.0.0.1:8081");
        env::set_var("GREENBONE_WAS_ZAP_API_KEY", "non-default-api-key");
        env::set_var("GREENBONE_WAS_SCAN_WORKER_COUNT", "3");
        env::set_var("GREENBONE_WAS_SCAN_ALERT_POLL_INTERVAL_SECONDS", "15");
        env::set_var("GREENBONE_WAS_SCAN_STOP_GRACE_PERIOD_SECONDS", "120");
        env::set_var(
            "GREENBONE_WAS_SCAN_AJAX_SPIDER_TIMEOUT_GRACE_PERIOD_SECONDS",
            "45",
        );
        env::set_var("GREENBONE_WAS_SCAN_RETRY_MAX_RETRIES", "7");
        env::set_var("GREENBONE_WAS_SCAN_RETRY_MAX_DELAY_SECONDS", "45");
    };

    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.log_format, "json");
    assert_eq!(settings.log_level, "debug");
    assert_eq!(settings.port, 8080);
    assert_eq!(settings.storage_backend, StorageBackend::Sqlite);
    assert_eq!(settings.var_data_dir, "/tmp/greenbone-was");
    assert_eq!(settings.sqlite_url, Some("sqlite:scans.db".to_string()));
    assert!(settings.sqlite_url_is_explicit);
    assert_eq!(settings.zap_base_url, "http://127.0.0.1:8081");
    assert_eq!(settings.zap_api_key, "non-default-api-key");
    assert_eq!(settings.scan_worker_count, 3);
    assert_eq!(settings.scan_alert_poll_interval_seconds, 15);
    assert_eq!(settings.scan_stop_grace_period_seconds, 120);
    assert_eq!(settings.scan_ajax_spider_timeout_grace_period_seconds, 45);
    assert_eq!(settings.scan_retry_max_retries, 7);
    assert_eq!(settings.scan_retry_max_delay_seconds, 45);
}

#[test]
#[serial]
fn test_rejects_invalid_port() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_PORT", "0");
    };

    let result = Settings::load();
    assert!(result.is_err(), "Expected error for invalid port");
    let err = result.err().unwrap();
    assert!(err.to_string().contains("port must be between 1 and 65535"));
}

#[test]
#[serial]
fn test_storage_backend_defaults_to_sqlite() {
    clear_env();
    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.storage_backend, StorageBackend::Sqlite);
}

#[test]
#[serial]
fn test_sqlite_backend_with_url() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_STORAGE_BACKEND", "sqlite");
        env::set_var("GREENBONE_WAS_SQLITE_URL", "sqlite:/tmp/test.db");
    }
    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.storage_backend, StorageBackend::Sqlite);
    assert_eq!(settings.sqlite_url.as_deref(), Some("sqlite:/tmp/test.db"));
    assert!(settings.sqlite_url_is_explicit);
}

#[test]
#[serial]
fn test_var_data_dir_changes_derived_sqlite_url() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_VAR_DATA_DIR", "/tmp/greenbone-was-data");
    }

    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.var_data_dir, "/tmp/greenbone-was-data");
    assert_eq!(
        settings.sqlite_url.as_deref(),
        Some("sqlite:/tmp/greenbone-was-data/scans.db")
    );
    assert!(!settings.sqlite_url_is_explicit);
}

#[test]
#[serial]
fn test_explicit_sqlite_url_overrides_var_data_dir_default() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_VAR_DATA_DIR", "/tmp/greenbone-was-data");
        env::set_var("GREENBONE_WAS_SQLITE_URL", "sqlite:/custom/scans.db");
    }

    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.var_data_dir, "/tmp/greenbone-was-data");
    assert_eq!(
        settings.sqlite_url.as_deref(),
        Some("sqlite:/custom/scans.db")
    );
    assert!(settings.sqlite_url_is_explicit);
}

#[test]
#[serial]
fn test_explicit_sqlite_url_matching_default_stays_explicit() {
    clear_env();
    unsafe {
        env::set_var(
            "GREENBONE_WAS_SQLITE_URL",
            settings::default_sqlite_url(settings::DEFAULT_VAR_DATA_DIR),
        );
    }

    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(
        settings.sqlite_url.as_deref(),
        Some(settings::default_sqlite_url(settings::DEFAULT_VAR_DATA_DIR).as_str())
    );
    assert!(settings.sqlite_url_is_explicit);
}

#[test]
#[serial]
fn test_sqlite_backend_with_empty_explicit_url_is_error() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_STORAGE_BACKEND", "sqlite");
        env::set_var("GREENBONE_WAS_SQLITE_URL", "");
    }
    let result = Settings::load();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("GREENBONE_WAS_SQLITE_URL"));
}

#[test]
#[serial]
fn test_in_memory_sqlite_url_is_rejected() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_SQLITE_URL", "sqlite::memory:");
    }

    let result = Settings::load();

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("file-backed SQLite database"));
}

#[test]
#[serial]
fn test_sqlite_url_with_memory_mode_is_rejected() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_SQLITE_URL", "sqlite:scans.db?mode=memory");
    }

    let result = Settings::load();

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("file-backed SQLite database"));
}

#[test]
#[serial]
fn test_unknown_storage_backend_is_error() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_STORAGE_BACKEND", "redis");
    }
    let result = Settings::load();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("unknown storage backend"));
}

#[test]
#[serial]
fn test_zero_scan_worker_count_is_error() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_SCAN_WORKER_COUNT", "0");
    }

    let result = Settings::load();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("scan_worker_count"));
}

#[test]
#[serial]
fn test_zero_scan_alert_poll_interval_is_error() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_SCAN_ALERT_POLL_INTERVAL_SECONDS", "0");
    }

    let result = Settings::load();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("scan_alert_poll_interval_seconds"));
}

#[test]
#[serial]
fn test_zero_scan_stop_grace_period_is_error() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_SCAN_STOP_GRACE_PERIOD_SECONDS", "0");
    }

    let result = Settings::load();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("scan_stop_grace_period_seconds"));
}
