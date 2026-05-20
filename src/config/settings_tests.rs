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
        env::remove_var("GREENBONE_WAS_SQLITE_URL");
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
    assert_eq!(settings.storage_backend, StorageBackend::InMemory);
    assert!(settings.sqlite_url.is_none());
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
        env::set_var("GREENBONE_WAS_SQLITE_URL", "sqlite:scans.db");
    };

    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.log_format, "json");
    assert_eq!(settings.log_level, "debug");
    assert_eq!(settings.port, 8080);
    assert_eq!(settings.storage_backend, StorageBackend::Sqlite);
    assert_eq!(settings.sqlite_url, Some("sqlite:scans.db".to_string()));
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
fn test_storage_backend_defaults_to_inmemory() {
    clear_env();
    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.storage_backend, StorageBackend::InMemory);
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
}

#[test]
#[serial]
fn test_sqlite_backend_without_url_is_error() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_STORAGE_BACKEND", "sqlite");
    }
    let result = Settings::load();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(err.to_string().contains("GREENBONE_WAS_SQLITE_URL"));
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
