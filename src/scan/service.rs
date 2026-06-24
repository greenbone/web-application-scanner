// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan orchestration facade used by API handlers.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::warn;
use uuid::Uuid;

use crate::{
    api::dto::scans::{
        PreferencesResponse, ScannerPreference, ScannerPreferenceMetadata, Target, Vt,
    },
    scan::{
        Scan, ScanResult, ScanRuntimeHandle, ScanServiceError, ScanStateCoordinator, ScanStatus,
        ScanStatusView,
        observability::{emit_scan_created, emit_scan_deleted},
        preferences::{
            AJAX_SPIDER_TIMEOUT_PREFERENCE_ID, SCAN_MODE_PREFERENCE_ID, preference_definitions,
        },
        validation::validate_target_urls,
    },
    storage::{StorageError, StorageHandle},
};

/// Shared handle for the scan service used in application state.
pub type ScanServiceHandle = Arc<dyn ScanService>;

/// Input payload for creating a scan.
#[derive(Debug, Clone)]
pub struct CreateScanRequest {
    pub scan_id: Option<String>,
    pub target: Target,
    pub scan_preferences: Vec<ScannerPreference>,
    pub vts: Vec<Vt>,
}

/// Internal scan orchestration commands used by transport handlers.
#[async_trait]
pub trait ScanService: Send + Sync {
    /// Recover scans left in a non-terminal, non-stored state from a previous service run.
    ///
    /// Called once at startup before the service begins accepting requests. Implementations
    /// should transition any `requested` or `running` scans to `failed`.
    async fn recover_interrupted_scans(&self) -> Result<(), ScanServiceError>;

    async fn get_default_preferences(&self) -> Result<PreferencesResponse, ScanServiceError>;

    async fn create_scan(&self, request: CreateScanRequest) -> Result<String, ScanServiceError>;

    async fn get_scan(&self, id: &str) -> Result<Scan, ScanServiceError>;

    async fn get_scan_result(
        &self,
        id: &str,
        result_id: i64,
    ) -> Result<ScanResult, ScanServiceError>;

    async fn get_scan_status(&self, id: &str) -> Result<ScanStatusView, ScanServiceError>;

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

    fn default_preferences_response() -> PreferencesResponse {
        PreferencesResponse(
            preference_definitions()
                .iter()
                .map(|pref| ScannerPreferenceMetadata {
                    id: pref.id.to_string(),
                    preference_type: pref.value_type.as_str().to_string(),
                    name: pref.name.to_string(),
                    description: pref.description.to_string(),
                    default_value: pref.default_value.to_string(),
                    values: if pref.allowed_values.is_empty() {
                        None
                    } else {
                        Some(pref.allowed_values.join(";"))
                    },
                })
                .collect(),
        )
    }

    fn resolve_scan_preferences(
        scan_preferences: Vec<ScannerPreference>,
    ) -> Result<Vec<ScannerPreference>, ScanServiceError> {
        let mut scan_mode = preference_definitions()
            .iter()
            .find(|p| p.id == SCAN_MODE_PREFERENCE_ID)
            .map(|p| p.default_value.to_string())
            .unwrap_or_else(|| "safe".to_string());

        let mut ajax_spider_timeout = preference_definitions()
            .iter()
            .find(|p| p.id == AJAX_SPIDER_TIMEOUT_PREFERENCE_ID)
            .map(|p| p.default_value.to_string())
            .unwrap_or_else(|| "0".to_string());

        let mut unknown_preferences: Vec<ScannerPreference> = Vec::new();

        for pref in scan_preferences {
            let value = pref.value.trim().to_string();
            match pref.id.as_str() {
                SCAN_MODE_PREFERENCE_ID => {
                    if value != "safe" && value != "active" {
                        return Err(ScanServiceError::InvalidPreference {
                            id: pref.id,
                            value,
                            reason: "allowed values are 'safe' and 'active'".to_string(),
                        });
                    }
                    scan_mode = value;
                }
                AJAX_SPIDER_TIMEOUT_PREFERENCE_ID => {
                    let parsed = value.parse::<i64>().map_err(|_| {
                        ScanServiceError::InvalidPreference {
                            id: pref.id.clone(),
                            value: value.clone(),
                            reason:
                                "value must be a non-negative integer in seconds (0 means unlimited)"
                                    .to_string(),
                        }
                    })?;
                    if parsed < 0 {
                        return Err(ScanServiceError::InvalidPreference {
                            id: pref.id,
                            value,
                            reason:
                                "value must be a non-negative integer in seconds (0 means unlimited)"
                                    .to_string(),
                        });
                    }
                    ajax_spider_timeout = parsed.to_string();
                }
                _ => {
                    warn!(
                        preference_id = %pref.id,
                        "unknown scan preference accepted and forwarded"
                    );
                    unknown_preferences.push(ScannerPreference { id: pref.id, value });
                }
            }
        }

        let mut resolved = vec![
            ScannerPreference {
                id: SCAN_MODE_PREFERENCE_ID.to_string(),
                value: scan_mode,
            },
            ScannerPreference {
                id: AJAX_SPIDER_TIMEOUT_PREFERENCE_ID.to_string(),
                value: ajax_spider_timeout,
            },
        ];
        resolved.extend(unknown_preferences);
        Ok(resolved)
    }
}

#[async_trait]
impl ScanService for DefaultScanService {
    async fn recover_interrupted_scans(&self) -> Result<(), ScanServiceError> {
        self.scan_state
            .recover_interrupted_scans()
            .await
            .map_err(ScanServiceError::Storage)
    }

    async fn get_default_preferences(&self) -> Result<PreferencesResponse, ScanServiceError> {
        Ok(Self::default_preferences_response())
    }

    async fn create_scan(&self, request: CreateScanRequest) -> Result<String, ScanServiceError> {
        let validated_hosts = validate_target_urls(&request.target.hosts)?;
        let resolved_scan_preferences = Self::resolve_scan_preferences(request.scan_preferences)?;
        let id = request
            .scan_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let scan = Scan {
            id: id.clone(),
            target: Target {
                hosts: validated_hosts,
                ..request.target
            },
            scan_preferences: resolved_scan_preferences,
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

    async fn get_scan_status(&self, id: &str) -> Result<ScanStatusView, ScanServiceError> {
        self.get_scan(id).await.map(|scan| scan.status_view())
    }

    async fn start_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        let scan_record = self
            .storage
            .get_scan(id)
            .await
            .map_err(Self::map_storage_err)?;

        let new_status = scan_record.status.start_command_transition().ok_or(
            ScanServiceError::InvalidTransition {
                from: scan_record.status,
                requested: ScanStatus::Requested,
            },
        )?;

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
                match self.scan_state.update_stop_requested(id, true).await {
                    Ok(()) => {}
                    Err(StorageError::InvalidState) => {
                        let latest_scan = self
                            .storage
                            .get_scan(id)
                            .await
                            .map_err(Self::map_storage_err)?;
                        return Err(ScanServiceError::InvalidTransition {
                            from: latest_scan.status,
                            requested: ScanStatus::Stopped,
                        });
                    }
                    Err(error) => return Err(Self::map_storage_err(error)),
                }

                if let Some(runtime) = &self.runtime {
                    runtime.request_stop(id.to_string()).await;
                }
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
