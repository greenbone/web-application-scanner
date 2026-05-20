// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::config::settings;
use serial_test::serial;
use std::env;

use super::Settings;

fn clear_env() {
    unsafe {
        env::remove_var("GREENBONE_WAS_LOG_FORMAT");
        env::remove_var("GREENBONE_WAS_LOG_LEVEL");
        env::remove_var("GREENBONE_WAS_PORT");
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
}

#[test]
#[serial]
fn test_uses_env_overrides_when_set() {
    clear_env();
    unsafe {
        env::set_var("GREENBONE_WAS_LOG_FORMAT", "json");
        env::set_var("GREENBONE_WAS_LOG_LEVEL", "debug");
        env::set_var("GREENBONE_WAS_PORT", "8080");
    };

    let settings = Settings::load().expect("Failed to load settings");
    assert_eq!(settings.log_format, "json");
    assert_eq!(settings.log_level, "debug");
    assert_eq!(settings.port, 8080);
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
