// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lifecycle transition persistence executor.

use crate::{
    scan::{ScanStatus, observability::emit_status_transition},
    storage::{StorageError, StorageHandle},
};

/// Persists lifecycle transitions and emits transition telemetry.
#[derive(Clone)]
pub(super) struct TransitionExecutor {
    storage: StorageHandle,
}

impl TransitionExecutor {
    pub(super) fn new(storage: StorageHandle) -> Self {
        Self { storage }
    }

    pub(super) async fn transition_status(
        &self,
        scan_id: &str,
        from: ScanStatus,
        to: ScanStatus,
    ) -> Result<(), StorageError> {
        self.storage
            .transition_scan_status(scan_id, from, to)
            .await?;
        emit_status_transition(scan_id, from, to);
        Ok(())
    }

    pub(super) async fn overwrite_status(
        &self,
        scan_id: &str,
        from: ScanStatus,
        to: ScanStatus,
    ) -> Result<(), StorageError> {
        self.storage.update_scan_status(scan_id, to).await?;
        emit_status_transition(scan_id, from, to);
        Ok(())
    }
}
