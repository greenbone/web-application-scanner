// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::api::dto::scans::ScanStatus;

use super::interface::{ResultRecord, ScanRecord, ScanStorage, StorageError};

struct InnerState {
    scans: HashMap<String, ScanRecord>,
    /// Results per scan ID, stored in insertion (ascending id) order.
    results: HashMap<String, Vec<ResultRecord>>,
}

/// Thread-safe in-memory storage backend backed by `RwLock`-protected maps.
#[derive(Clone)]
pub struct InMemoryStorage {
    state: Arc<RwLock<InnerState>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(InnerState {
                scans: HashMap::new(),
                results: HashMap::new(),
            })),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ScanStorage for InMemoryStorage {
    async fn create_scan(&self, scan: ScanRecord) -> Result<(), StorageError> {
        let mut state = self.state.write().await;
        if state.scans.contains_key(&scan.id) {
            return Err(StorageError::AlreadyExists(scan.id));
        }
        let id = scan.id.clone();
        state.scans.insert(id.clone(), scan);
        state.results.insert(id, Vec::new());
        Ok(())
    }

    async fn get_scan(&self, id: &str) -> Result<ScanRecord, StorageError> {
        let state = self.state.read().await;
        state
            .scans
            .get(id)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(id.to_string()))
    }

    async fn update_scan_status(&self, id: &str, status: ScanStatus) -> Result<(), StorageError> {
        let mut state = self.state.write().await;
        let scan = state
            .scans
            .get_mut(id)
            .ok_or_else(|| StorageError::NotFound(id.to_string()))?;
        scan.status = status;
        Ok(())
    }

    async fn delete_scan(&self, id: &str) -> Result<(), StorageError> {
        let mut state = self.state.write().await;
        if state.scans.remove(id).is_none() {
            return Err(StorageError::NotFound(id.to_string()));
        }
        state.results.remove(id);
        Ok(())
    }

    async fn add_result(&self, scan_id: &str, mut result: ResultRecord) -> Result<(), StorageError> {
        let mut state = self.state.write().await;
        if !state.scans.contains_key(scan_id) {
            return Err(StorageError::NotFound(scan_id.to_string()));
        }
        let results = state.results.entry(scan_id.to_string()).or_default();
        result.id = results.len() as i64;
        result.scan_id = scan_id.to_string();
        results.push(result);
        Ok(())
    }

    async fn get_result(&self, scan_id: &str, result_id: i64) -> Result<ResultRecord, StorageError> {
        let state = self.state.read().await;
        let results = state
            .results
            .get(scan_id)
            .ok_or_else(|| StorageError::NotFound(scan_id.to_string()))?;
        results
            .get(result_id as usize)
            .cloned()
            .ok_or_else(|| StorageError::ResultNotFound(scan_id.to_string(), result_id))
    }

    async fn get_results(
        &self,
        scan_id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ResultRecord>, StorageError> {
        let state = self.state.read().await;
        if !state.scans.contains_key(scan_id) {
            return Err(StorageError::NotFound(scan_id.to_string()));
        }
        let results = state.results.get(scan_id).map(Vec::as_slice).unwrap_or(&[]);
        let slice_start = start.min(results.len());
        let slice_end = match end {
            Some(e) => (e + 1).min(results.len()),
            None => results.len(),
        };
        Ok(results[slice_start..slice_end].to_vec())
    }
}

#[cfg(test)]
#[path = "in_memory_tests.rs"]
mod tests;
