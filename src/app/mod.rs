// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Application-wide state and context.

pub mod error;

use crate::scan::ScanServiceHandle;
use crate::storage::StorageHandle;

/// Application state injected into every Axum route handler.
///
/// Contains references to all services and dependencies needed by the API handlers.
#[derive(Clone)]
pub struct AppState {
    /// Handle to the configured storage backend.
    pub storage: StorageHandle,
    /// Handle to the internal scan orchestration service.
    pub scan_service: ScanServiceHandle,
}

impl AppState {
    /// Create a new application state with the given storage backend.
    pub fn new(storage: StorageHandle, scan_service: ScanServiceHandle) -> Self {
        Self {
            storage,
            scan_service,
        }
    }
}
