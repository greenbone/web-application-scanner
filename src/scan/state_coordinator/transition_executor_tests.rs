// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use tracing_test::traced_test;

use super::TransitionExecutor;
use crate::{
    api::dto::scans::Target,
    scan::ScanStatus,
    storage::{ScanRecord, StorageError, test_support::temporary_sqlite_storage},
};

fn make_scan(id: &str, status: ScanStatus) -> ScanRecord {
    ScanRecord {
        id: id.to_string(),
        target: Target {
            hosts: vec!["https://example.test".to_string()],
            excluded_hosts: vec![],
            credentials: vec![],
        },
        scan_preferences: vec![],
        vts: vec![],
        status,
        stop_requested: false,
        queued_time: None,
        start_time: None,
        end_time: None,
        context_name: None,
        context_id: None,
        alert_cursor: None,
        progress: None,
        interruption_reason: None,
    }
}

#[traced_test]
#[tokio::test]
async fn transition_status_compare_and_swap_success_emits_transition_telemetry() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    storage
        .create_scan(make_scan("scan-cas-ok", ScanStatus::Stored))
        .await
        .unwrap();

    let executor = TransitionExecutor::new(storage.clone());
    executor
        .transition_status("scan-cas-ok", ScanStatus::Stored, ScanStatus::Requested)
        .await
        .unwrap();

    let updated = storage.get_scan("scan-cas-ok").await.unwrap();
    assert_eq!(updated.status, ScanStatus::Requested);
    assert!(logs_contain("scan status transition"));
}

#[traced_test]
#[tokio::test]
async fn transition_status_returns_invalid_state_and_does_not_emit_telemetry_on_failed_write() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    storage
        .create_scan(make_scan("scan-cas-invalid", ScanStatus::Stored))
        .await
        .unwrap();

    let executor = TransitionExecutor::new(storage);
    let err = executor
        .transition_status("scan-cas-invalid", ScanStatus::Running, ScanStatus::Failed)
        .await
        .unwrap_err();

    assert!(matches!(err, StorageError::InvalidState));
    assert!(!logs_contain("scan status transition"));
}

#[traced_test]
#[tokio::test]
async fn transition_status_returns_not_found_and_does_not_emit_telemetry_on_failed_write() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let executor = TransitionExecutor::new(storage);

    let err = executor
        .transition_status("missing-scan", ScanStatus::Stored, ScanStatus::Requested)
        .await
        .unwrap_err();

    assert!(matches!(err, StorageError::NotFound(id) if id == "missing-scan"));
    assert!(!logs_contain("scan status transition"));
}

#[tokio::test]
async fn recover_interrupted_scans_marks_requested_and_running_as_failed() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    storage
        .create_scan(make_scan("scan-stored", ScanStatus::Stored))
        .await
        .unwrap();
    storage
        .create_scan(make_scan("scan-requested", ScanStatus::Requested))
        .await
        .unwrap();
    storage
        .create_scan(make_scan("scan-running", ScanStatus::Running))
        .await
        .unwrap();

    let executor = TransitionExecutor::new(storage.clone());
    executor.recover_interrupted_scans().await.unwrap();

    let stored = storage.get_scan("scan-stored").await.unwrap();
    let requested = storage.get_scan("scan-requested").await.unwrap();
    let running = storage.get_scan("scan-running").await.unwrap();

    assert_eq!(stored.status, ScanStatus::Stored);
    assert_eq!(requested.status, ScanStatus::Failed);
    assert_eq!(running.status, ScanStatus::Failed);
}
