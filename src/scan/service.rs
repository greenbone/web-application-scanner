// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan orchestration facade used by API handlers.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    api::dto::scans::{PreferencesResponse, ScannerPreference, Target, Vt},
    scan::{
        Scan, ScanResult, ScanRuntimeHandle, ScanServiceError, ScanStateCoordinator, ScanStatus,
        ScanStatusView,
        observability::{emit_scan_created, emit_scan_deleted},
    },
    storage::{StorageError, StorageHandle},
};

/// Shared handle for the scan service used in application state.
pub type ScanServiceHandle = Arc<dyn ScanService>;

/// Input payload for creating a scan.
#[derive(Debug, Clone)]
pub struct CreateScanRequest {
    pub target: Target,
    pub scan_preferences: Vec<ScannerPreference>,
    pub vts: Vec<Vt>,
}

/// Internal scan orchestration commands used by transport handlers.
#[async_trait]
pub trait ScanService: Send + Sync {
    async fn get_default_preferences(&self) -> Result<PreferencesResponse, ScanServiceError>;

    async fn create_scan(&self, request: CreateScanRequest) -> Result<String, ScanServiceError>;

    async fn get_scan(&self, id: &str) -> Result<Scan, ScanServiceError>;

    async fn get_scan_result(
        &self,
        id: &str,
        result_id: i64,
    ) -> Result<ScanResult, ScanServiceError>;

    async fn get_scan_status(
        &self,
        id: &str,
    ) -> Result<ScanStatusView, ScanServiceError>;

    async fn start_scan(&self, id: &str) -> Result<(), ScanServiceError>;

    async fn stop_scan(&self, id: &str) -> Result<(), ScanServiceError>;

    async fn delete_scan(&self, id: &str) -> Result<(), ScanServiceError>;

    async fn get_results(
        &self,
        id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ScanResult>, ScanServiceError>;
}

/// Default scan service implementation backed by the configured storage.
#[derive(Clone)]
pub struct DefaultScanService {
    storage: StorageHandle,
    scan_state: ScanStateCoordinator,
    runtime: Option<ScanRuntimeHandle>,
}

impl DefaultScanService {
    pub fn new_storage_only(storage: StorageHandle) -> Self {
        Self {
            scan_state: ScanStateCoordinator::new(storage.clone()),
            storage,
            runtime: None,
        }
    }

    pub fn new(storage: StorageHandle, runtime: ScanRuntimeHandle) -> Self {
        Self {
            scan_state: ScanStateCoordinator::new(storage.clone()),
            storage,
            runtime: Some(runtime),
        }
    }

    fn map_storage_err(err: StorageError) -> ScanServiceError {
        match err {
            StorageError::NotFound(id) => ScanServiceError::ScanNotFound(id),
            other => ScanServiceError::Storage(other),
        }
    }
}

#[async_trait]
impl ScanService for DefaultScanService {
    async fn get_default_preferences(&self) -> Result<PreferencesResponse, ScanServiceError> {
        Ok(PreferencesResponse::default())
    }

    async fn create_scan(&self, request: CreateScanRequest) -> Result<String, ScanServiceError> {
        let id = Uuid::new_v4().to_string();
        let scan = Scan {
            id: id.clone(),
            target: request.target,
            scan_preferences: request.scan_preferences,
            vts: request.vts,
            status: ScanStatus::Stored,
            stop_requested: false,
            queued_time: None,
            start_time: None,
            end_time: None,
            context_name: None,
            context_id: None,
            alert_cursor: None,
            progress: None,
            interruption_reason: None,
        };

        self.storage
            .create_scan(scan.into())
            .await
            .map_err(Self::map_storage_err)?;

        emit_scan_created(&id);

        Ok(id)
    }

    async fn get_scan(&self, id: &str) -> Result<Scan, ScanServiceError> {
        self.storage
            .get_scan(id)
            .await
            .map(Into::into)
            .map_err(Self::map_storage_err)
    }

    async fn get_scan_result(
        &self,
        id: &str,
        result_id: i64,
    ) -> Result<ScanResult, ScanServiceError> {
        self.storage
            .get_result(id, result_id)
            .await
            .map(Into::into)
            .map_err(Self::map_storage_err)
    }

    async fn get_scan_status(
        &self,
        id: &str,
    ) -> Result<ScanStatusView, ScanServiceError> {
        self.get_scan(id).await.map(|scan| scan.status_view())
    }

    async fn start_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        let scan_record = self
            .storage
            .get_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        let new_status =
            scan_record.status
                .start_command_transition()
                .ok_or(ScanServiceError::InvalidTransition {
                    from: scan_record.status,
                    requested: ScanStatus::Requested,
                })?;

        self.scan_state
            .transition_status(id, scan_record.status, new_status)
            .await
            .map_err(Self::map_storage_err)?;

        if let Some(runtime) = &self.runtime {
            runtime.enqueue(id.to_string()).await;
        }

        Ok(())
    }

    async fn stop_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        let scan_record = self
            .storage
            .get_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        match scan_record.status {
            ScanStatus::Requested => {
                if let Some(runtime) = &self.runtime {
                    runtime.remove_queued(id).await;
                }

                self.scan_state
                    .transition_status(id, scan_record.status, ScanStatus::Stopped)
                    .await
                    .map_err(Self::map_storage_err)?;
            }
            ScanStatus::Running => {
                self.storage
                    .update_scan_stop_requested(id, true)
                    .await
                    .map_err(Self::map_storage_err)?;
            }
            _ => {
                return Err(ScanServiceError::InvalidTransition {
                    from: scan_record.status,
                    requested: ScanStatus::Stopped,
                });
            }
        }

        Ok(())
    }

    async fn delete_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        let scan_record = self
            .storage
            .get_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        if !scan_record.status.can_delete() {
            return Err(ScanServiceError::InvalidTransition {
                from: scan_record.status,
                requested: scan_record.status,
            });
        }

        self.storage
            .delete_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        emit_scan_deleted(id);

        Ok(())
    }

    async fn get_results(
        &self,
        id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ScanResult>, ScanServiceError> {
        self.storage
            .get_results(id, start, end)
            .await
            .map(|results| results.into_iter().map(Into::into).collect())
            .map_err(Self::map_storage_err)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
