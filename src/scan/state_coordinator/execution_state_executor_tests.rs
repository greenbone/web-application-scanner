// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::ExecutionStateExecutor;
use crate::{
    api::dto::scans::ResultType,
    scan::{ScanResult, ScanStatus},
    storage::{ResultRecord, ScanRecord, ScanStorage, StorageError},
};

#[derive(Default)]
struct RecordingStorage {
    calls: Mutex<Vec<&'static str>>,
    add_results_error: Mutex<Option<StorageError>>,
}

impl RecordingStorage {
    fn with_add_results_error(error: StorageError) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            add_results_error: Mutex::new(Some(error)),
        }
    }

    fn calls(&self) -> Vec<&'static str> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl ScanStorage for RecordingStorage {
    async fn create_scan(&self, _scan: ScanRecord) -> Result<(), StorageError> {
        panic!("not used by this test");
    }

    async fn get_scan(&self, _id: &str) -> Result<ScanRecord, StorageError> {
        panic!("not used by this test");
    }

    async fn list_non_terminal_scans(&self) -> Result<Vec<ScanRecord>, StorageError> {
        panic!("not used by this test");
    }

    async fn update_scan_status(&self, _id: &str, _status: ScanStatus) -> Result<(), StorageError> {
        panic!("not used by this test");
    }

    async fn transition_scan_status(
        &self,
        _id: &str,
        _expected: ScanStatus,
        _new_status: ScanStatus,
    ) -> Result<(), StorageError> {
        panic!("not used by this test");
    }

    async fn update_scan_progress(
        &self,
        _id: &str,
        _progress: Option<serde_json::Value>,
    ) -> Result<(), StorageError> {
        self.calls.lock().unwrap().push("update_scan_progress");
        Ok(())
    }

    async fn update_scan_context(
        &self,
        _id: &str,
        _context_name: Option<String>,
        _context_id: Option<String>,
    ) -> Result<(), StorageError> {
        panic!("not used by this test");
    }

    async fn update_alert_cursor(
        &self,
        _id: &str,
        _alert_cursor: Option<i64>,
    ) -> Result<(), StorageError> {
        self.calls.lock().unwrap().push("update_alert_cursor");
        Ok(())
    }

    async fn update_scan_stop_requested(
        &self,
        _id: &str,
        _stop_requested: bool,
    ) -> Result<(), StorageError> {
        panic!("not used by this test");
    }

    async fn delete_scan(&self, _id: &str) -> Result<(), StorageError> {
        panic!("not used by this test");
    }

    async fn add_result(&self, _scan_id: &str, _result: ResultRecord) -> Result<(), StorageError> {
        panic!("not used by this test");
    }

    async fn add_results(
        &self,
        _scan_id: &str,
        _results: Vec<ResultRecord>,
    ) -> Result<(), StorageError> {
        self.calls.lock().unwrap().push("add_results");
        if let Some(err) = self.add_results_error.lock().unwrap().take() {
            return Err(err);
        }
        Ok(())
    }

    async fn get_result(&self, _scan_id: &str, _result_id: i64) -> Result<ResultRecord, StorageError> {
        panic!("not used by this test");
    }

    async fn get_results(
        &self,
        _scan_id: &str,
        _start: usize,
        _end: Option<usize>,
    ) -> Result<Vec<ResultRecord>, StorageError> {
        panic!("not used by this test");
    }
}

fn make_result() -> ScanResult {
    ScanResult {
        id: 0,
        scan_id: "scan-1".to_string(),
        result_type: ResultType::Alarm,
        ip_address: Some("https://example.test".to_string()),
        hostname: Some("example.test".to_string()),
        oid: Some("1".to_string()),
        port: Some(443),
        protocol: Some("tcp".to_string()),
        message: Some("message".to_string()),
        detail: None,
    }
}

#[tokio::test]
async fn persist_alert_batch_writes_results_before_updating_cursor() {
    let storage = Arc::new(RecordingStorage::default());
    let executor = ExecutionStateExecutor::new(storage.clone());

    executor
        .persist_alert_batch("scan-1", 8, vec![make_result()])
        .await
        .unwrap();

    let calls = storage.calls();
    assert_eq!(calls, vec!["add_results", "update_alert_cursor"]);
}

#[tokio::test]
async fn persist_alert_batch_does_not_advance_cursor_when_result_batch_write_fails() {
    let storage = Arc::new(RecordingStorage::with_add_results_error(
        StorageError::Backend("boom".to_string()),
    ));
    let executor = ExecutionStateExecutor::new(storage.clone());

    let err = executor
        .persist_alert_batch("scan-1", 9, vec![make_result()])
        .await
        .unwrap_err();

    assert!(matches!(err, StorageError::Backend(message) if message == "boom"));

    let calls = storage.calls();
    assert_eq!(calls, vec!["add_results"]);
}

#[tokio::test]
async fn update_progress_routes_to_storage_progress_update() {
    let storage = Arc::new(RecordingStorage::default());
    let executor = ExecutionStateExecutor::new(storage.clone());

    executor
        .update_progress("scan-progress", Some(serde_json::json!({ "stage": "spider" })))
        .await
        .unwrap();

    let calls = storage.calls();
    assert_eq!(calls, vec!["update_scan_progress"]);
}
