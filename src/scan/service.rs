// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan orchestration facade used by API handlers.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    api::dto::scans::{ScannerPreference, Target, Vt},
    scan::{ScanServiceError, ScanStatus},
    storage::{ResultRecord, ScanRecord, StorageError, StorageHandle},
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
    async fn create_scan(&self, request: CreateScanRequest) -> Result<String, ScanServiceError>;

    async fn start_scan(&self, id: &str) -> Result<(), ScanServiceError>;

    async fn stop_scan(&self, id: &str) -> Result<(), ScanServiceError>;

    async fn delete_scan(&self, id: &str) -> Result<(), ScanServiceError>;

    async fn get_results(
        &self,
        id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ResultRecord>, ScanServiceError>;
}

/// Default scan service implementation backed by the configured storage.
#[derive(Clone)]
pub struct DefaultScanService {
    storage: StorageHandle,
}

impl DefaultScanService {
    pub fn new(storage: StorageHandle) -> Self {
        Self { storage }
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
    async fn create_scan(&self, request: CreateScanRequest) -> Result<String, ScanServiceError> {
        let id = Uuid::new_v4().to_string();
        let scan = ScanRecord {
            id: id.clone(),
            target: request.target,
            scan_preferences: request.scan_preferences,
            vts: request.vts,
            status: ScanStatus::New,
            start_time: None,
            end_time: None,
        };

        self.storage
            .create_scan(scan)
            .await
            .map_err(Self::map_storage_err)?;

        Ok(id)
    }

    async fn start_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        let scan = self
            .storage
            .get_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        let new_status = scan
            .status
            .start_command_transition()
            .ok_or(ScanServiceError::InvalidTransition {
                from: scan.status,
                requested: ScanStatus::Queued,
            })?;

        self.storage
            .update_scan_status(id, new_status)
            .await
            .map_err(Self::map_storage_err)
    }

    async fn stop_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        let scan = self
            .storage
            .get_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        let requested = if scan.status == ScanStatus::Running {
            ScanStatus::StopRequested
        } else {
            ScanStatus::Stopped
        };

        let new_status = scan
            .status
            .stop_command_transition()
            .ok_or(ScanServiceError::InvalidTransition {
                from: scan.status,
                requested,
            })?;

        self.storage
            .update_scan_status(id, new_status)
            .await
            .map_err(Self::map_storage_err)
    }

    async fn delete_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        let scan = self
            .storage
            .get_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        if !scan.status.can_delete() {
            return Err(ScanServiceError::InvalidTransition {
                from: scan.status,
                requested: scan.status,
            });
        }

        self.storage.delete_scan(id).await.map_err(Self::map_storage_err)
    }

    async fn get_results(
        &self,
        id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ResultRecord>, ScanServiceError> {
        self.storage
            .get_results(id, start, end)
            .await
            .map_err(Self::map_storage_err)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
