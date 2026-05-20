// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod error;

use crate::storage::StorageHandle;

/// Application state injected into every Axum route handler.
#[derive(Clone)]
pub struct AppState {
    pub storage: StorageHandle,
}

impl AppState {
    pub fn new(storage: StorageHandle) -> Self {
        Self { storage }
    }
}
