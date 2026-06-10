// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Coordinator for scan state persistence executors.

mod execution_state_executor;
mod transition_executor;

use self::{execution_state_executor::ExecutionStateExecutor, transition_executor::TransitionExecutor};

use crate::{
    scan::{ScanResult, ScanStatus},
    storage::{StorageError, StorageHandle},
};

/// Composes transition and execution-state persistence executors.
#[derive(Clone)]
pub struct ScanStateCoordinator {
    transition_executor: TransitionExecutor,
    execution_state_executor: ExecutionStateExecutor,
}

impl ScanStateCoordinator {
    pub fn new(storage: StorageHandle) -> Self {
        Self {
            transition_executor: TransitionExecutor::new(storage.clone()),
            execution_state_executor: ExecutionStateExecutor::new(storage),
        }
    }

    pub async fn transition_status(
        &self,
        scan_id: &str,
        from: ScanStatus,
        to: ScanStatus,
    ) -> Result<(), StorageError> {
        self.transition_executor
            .transition_status(scan_id, from, to)
            .await
    }

    pub async fn overwrite_status(
        &self,
        scan_id: &str,
        from: ScanStatus,
        to: ScanStatus,
    ) -> Result<(), StorageError> {
        self.transition_executor.overwrite_status(scan_id, from, to).await
    }

    pub async fn update_progress(
        &self,
        scan_id: &str,
        progress: Option<serde_json::Value>,
    ) -> Result<(), StorageError> {
        self.execution_state_executor
            .update_progress(scan_id, progress)
            .await
    }

    pub async fn update_context(
        &self,
        scan_id: &str,
        context_name: Option<String>,
        context_id: Option<String>,
    ) -> Result<(), StorageError> {
        self.execution_state_executor
            .update_context(scan_id, context_name, context_id)
            .await
    }

    pub async fn persist_alert_batch(
        &self,
        scan_id: &str,
        next_cursor: i64,
        results: Vec<ScanResult>,
    ) -> Result<(), StorageError> {
        self.execution_state_executor
            .persist_alert_batch(scan_id, next_cursor, results)
            .await
    }
}
