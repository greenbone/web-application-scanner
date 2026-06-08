// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Domain errors returned by scan orchestration services.

use thiserror::Error;

use crate::{scan::ScanStatus, storage::StorageError, zapclient::ZapClientError};

/// Errors returned by [`crate::scan::ScanService`] operations.
#[derive(Debug, Error)]
pub enum ScanServiceError {
    /// Requested lifecycle transition is not valid for the current state.
    #[error("invalid scan transition from '{from:?}' to '{requested:?}'")]
    InvalidTransition {
        from: ScanStatus,
        requested: ScanStatus,
    },

    /// Referenced scan does not exist.
    #[error("scan not found: {0}")]
    ScanNotFound(String),

    /// Target URL failed validation.
    #[error("invalid target url '{value}': {reason}")]
    InvalidUrl { value: String, reason: String },

    /// Storage backend failure.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// ZAP client failure.
    #[error(transparent)]
    ZapClient(#[from] ZapClientError),
}
