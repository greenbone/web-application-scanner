// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lifecycle transition persistence executor.

use tracing::warn;

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

    /// Transition all `requested` and `running` scans to `failed`.
    ///
    /// Called once at startup to recover scans that were interrupted by a crash.
    /// Scans in `stored` status are left untouched.
    pub(super) async fn recover_interrupted_scans(&self) -> Result<(), StorageError> {
        let non_terminal = self.storage.list_non_terminal_scans().await?;

        for scan in non_terminal {
            match scan.status {
                ScanStatus::Requested | ScanStatus::Running => {
                    warn!(
                        scan_id = %scan.id,
                        status = ?scan.status,
                        "startup recovery: transitioning interrupted scan to failed"
                    );
                    self.storage
                        .update_scan_status(&scan.id, ScanStatus::Failed)
                        .await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
