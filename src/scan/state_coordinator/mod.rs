// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Coordinator for scan state persistence executors.

mod execution_state_executor;
mod transition_executor;

use self::{
    execution_state_executor::ExecutionStateExecutor, transition_executor::TransitionExecutor,
};

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
        self.transition_executor
            .overwrite_status(scan_id, from, to)
            .await
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

    pub async fn update_stop_requested(
        &self,
        scan_id: &str,
        stop_requested: bool,
    ) -> Result<(), StorageError> {
        self.execution_state_executor
            .update_stop_requested(scan_id, stop_requested)
            .await
    }

    /// Recover scans left in a non-terminal, non-stored state from a previous service run.
    ///
    /// Any scan found in `requested` or `running` status at startup is assumed to have been
    /// interrupted by a crash. Such scans are transitioned directly to `failed`. Scans in
    /// `stored` status are left untouched.
    pub async fn recover_interrupted_scans(&self) -> Result<(), StorageError> {
        self.transition_executor.recover_interrupted_scans().await
    }
}

/// Scan state coordinator wrapper that automatically retries operations on transient failures.
///
/// Wraps a [`ScanStateCoordinator`] and applies exponential backoff retry logic to all API calls.
/// Storage transient errors (backend connection issues) are automatically retried; permanent
/// errors (invalid state transitions) fail immediately.
#[derive(Clone)]
pub struct RetryingScanStateCoordinator {
    inner: ScanStateCoordinator,
    max_retries: u32,
    max_delay: std::time::Duration,
}

impl RetryingScanStateCoordinator {
    /// Create a new retrying coordinator from an existing state coordinator.
    pub fn new(
        coordinator: ScanStateCoordinator,
        max_retries: u32,
        max_delay: std::time::Duration,
    ) -> Self {
        Self {
            inner: coordinator,
            max_retries,
            max_delay,
        }
    }

    /// Transition a scan from one status to another (only if current status matches).
    pub async fn transition_status(
        &self,
        scan_id: &str,
        from: ScanStatus,
        to: ScanStatus,
    ) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();

        crate::scan::retry::with_retry(
            move || {
                let inner = inner.clone();
                let id = id.clone();
                async move { inner.transition_status(&id, from, to).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Overwrite a scan status (regardless of current status).
    pub async fn overwrite_status(
        &self,
        scan_id: &str,
        from: ScanStatus,
        to: ScanStatus,
    ) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();

        crate::scan::retry::with_retry(
            move || {
                let inner = inner.clone();
                let id = id.clone();
                async move { inner.overwrite_status(&id, from, to).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Update scan progress.
    pub async fn update_progress(
        &self,
        scan_id: &str,
        progress: Option<serde_json::Value>,
    ) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();

        crate::scan::retry::with_retry(
            move || {
                let inner = inner.clone();
                let id = id.clone();
                let pv = progress.clone();
                async move { inner.update_progress(&id, pv).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Update scan context information.
    pub async fn update_context(
        &self,
        scan_id: &str,
        context_name: Option<String>,
        context_id: Option<String>,
    ) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();
        let cn = context_name.clone();
        let ci = context_id.clone();

        crate::scan::retry::with_retry(
            move || {
                let inner = inner.clone();
                let id = id.clone();
                let cn = cn.clone();
                let ci = ci.clone();
                async move { inner.update_context(&id, cn, ci).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Persist a batch of scan alerts.
    pub async fn persist_alert_batch(
        &self,
        scan_id: &str,
        next_cursor: i64,
        results: Vec<crate::scan::ScanResult>,
    ) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();

        crate::scan::retry::with_retry(
            move || {
                let inner = inner.clone();
                let id = id.clone();
                let res = results.clone();
                async move { inner.persist_alert_batch(&id, next_cursor, res).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Update the stop_requested flag.
    pub async fn update_stop_requested(
        &self,
        scan_id: &str,
        stop_requested: bool,
    ) -> Result<(), StorageError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();

        crate::scan::retry::with_retry(
            move || {
                let inner = inner.clone();
                let id = id.clone();
                async move { inner.update_stop_requested(&id, stop_requested).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Get a reference to the inner [`ScanStateCoordinator`] for direct access when retries are not desired.
    pub fn inner(&self) -> &ScanStateCoordinator {
        &self.inner
    }
}
