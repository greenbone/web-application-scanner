// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Logging setup and initialization.

use tracing_subscriber::{EnvFilter, fmt};

use crate::config::settings::Settings;

/// Initialize logging based on configuration settings.
///
/// Supports both text (fmt) and JSON output formats.
/// Log level filtering is controlled via the settings or RUST_LOG environment variable.
pub fn init_logging(settings: &Settings) {
    let env_filter = env_filter_from(&settings.log_level);

    match settings.log_format.as_str() {
        "json" => {
            fmt()
                .json()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_current_span(true)
                .with_span_list(true)
                .init();
        }
        _ => {
            fmt()
                .compact()
                .with_env_filter(env_filter)
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .init();
        }
    }
}

/// Initialize the log filter from the provided log level string.
fn env_filter_from(log_level: &str) -> EnvFilter {
    EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info"))
}
