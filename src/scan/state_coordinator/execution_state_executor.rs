// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runtime execution state persistence executor.

use crate::{
    scan::ScanResult,
    storage::{StorageError, StorageHandle},
};

/// Persists runtime execution state for worker/service paths.
#[derive(Clone)]
pub(super) struct ExecutionStateExecutor {
    storage: StorageHandle,
}

impl ExecutionStateExecutor {
    pub(super) fn new(storage: StorageHandle) -> Self {
        Self { storage }
    }

    pub(super) async fn update_progress(
        &self,
        scan_id: &str,
        progress: Option<serde_json::Value>,
    ) -> Result<(), StorageError> {
        self.storage.update_scan_progress(scan_id, progress).await
    }

    pub(super) async fn update_context(
        &self,
        scan_id: &str,
        context_name: Option<String>,
        context_id: Option<String>,
    ) -> Result<(), StorageError> {
        self.storage
            .update_scan_context(scan_id, context_name, context_id)
            .await
    }

    pub(super) async fn persist_alert_batch(
        &self,
        scan_id: &str,
        next_cursor: i64,
        results: Vec<ScanResult>,
    ) -> Result<(), StorageError> {
        let records = results.into_iter().map(Into::into).collect();
        self.storage.add_results(scan_id, records).await?;
        self.storage
            .update_alert_cursor(scan_id, Some(next_cursor))
            .await?;
        Ok(())
    }

    pub(super) async fn update_stop_requested(
        &self,
        scan_id: &str,
        stop_requested: bool,
    ) -> Result<(), StorageError> {
        self.storage
            .update_scan_stop_requested(scan_id, stop_requested)
            .await
    }
}

#[cfg(test)]
#[path = "execution_state_executor_tests.rs"]
mod execution_state_executor_tests;
