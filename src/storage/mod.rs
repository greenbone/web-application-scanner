// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod in_memory;
pub mod interface;
pub mod sqlite;

use std::sync::Arc;

pub use interface::{ResultRecord, ScanRecord, ScanStorage, StorageError, parse_range};

/// Shared, cloneable handle to the active storage backend.
///
/// `Arc<dyn ScanStorage>` implements `Clone` and satisfies Axum's `State`
/// requirements when wrapped in [`crate::app::AppState`].
pub type StorageHandle = Arc<dyn ScanStorage>;
