// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Application-level error types.

use thiserror::Error;

/// Errors that can occur during application startup and operation.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to load settings: {0}")]
    Settings(#[from] config::ConfigError),

    #[error("failed to initialize storage: {0}")]
    Storage(String),

    #[error("failed to bind TCP listener: {0}")]
    Bind(#[source] std::io::Error),

    #[error("HTTP server failed: {0}")]
    Server(#[source] std::io::Error),
}
