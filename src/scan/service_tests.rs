// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;
use tracing_test::traced_test;

use crate::{
    api::dto::scans::{ResultType, Target},
    config::settings::SQLITE_IN_MEMORY_URL,
    scan::{CreateScanRequest, DefaultScanService, ScanService, ScanServiceError, ScanStatus},
    storage::{ResultRecord, ScanRecord, ScanStorage, StorageError, sqlite::SqliteStorage},
};

fn make_request() -> CreateScanRequest {
    CreateScanRequest {
        target: Target {
            hosts: vec!["https://example.test".to_string()],
            excluded_hosts: vec![],
            credentials: vec![],
        },
        scan_preferences: vec![],
        vts: vec![],
    }
}

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
async fn create_scan_persists_stored_status() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    let service = DefaultScanService::new_storage_only(storage.clone());

    let scan_id = service.create_scan(make_request()).await.unwrap();
    let persisted = storage.get_scan(&scan_id).await.unwrap();

    assert_eq!(persisted.id, scan_id);
    assert_eq!(persisted.status, ScanStatus::Stored);
    assert!(logs_contain("scan created"));
}

#[traced_test]
#[tokio::test]
async fn start_scan_transitions_stored_to_requested() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("start-scan", ScanStatus::Stored))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage.clone());

    service.start_scan("start-scan").await.unwrap();

    let persisted = storage.get_scan("start-scan").await.unwrap();
    assert_eq!(persisted.status, ScanStatus::Requested);
    assert!(logs_contain("scan status transition"));
}

#[tokio::test]
async fn start_scan_returns_invalid_transition_for_non_stored_scans() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("done-scan", ScanStatus::Succeeded))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage);

    let err = service.start_scan("done-scan").await.unwrap_err();

    assert!(matches!(
        err,
        ScanServiceError::InvalidTransition {
            from: ScanStatus::Succeeded,
            requested: ScanStatus::Requested,
        }
    ));
}

#[tokio::test]
async fn stop_scan_transitions_requested_to_stopped() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("queued-scan", ScanStatus::Requested))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage.clone());

    service.stop_scan("queued-scan").await.unwrap();

    let persisted = storage.get_scan("queued-scan").await.unwrap();
    assert_eq!(persisted.status, ScanStatus::Stopped);
}

#[tokio::test]
async fn stop_scan_marks_running_scan_as_stop_requested() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("running-scan", ScanStatus::Running))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage.clone());

    service.stop_scan("running-scan").await.unwrap();

    let persisted = storage.get_scan("running-scan").await.unwrap();
    assert_eq!(persisted.status, ScanStatus::Running);
    assert!(persisted.stop_requested);
}

#[tokio::test]
async fn delete_scan_rejects_non_deletable_states() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("queued-delete", ScanStatus::Requested))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage);

    let err = service.delete_scan("queued-delete").await.unwrap_err();

    assert!(matches!(
        err,
        ScanServiceError::InvalidTransition {
            from: ScanStatus::Requested,
            requested: ScanStatus::Requested,
        }
    ));
}

#[traced_test]
#[tokio::test]
async fn delete_scan_emits_info_log() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("done-delete", ScanStatus::Succeeded))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage);

    service.delete_scan("done-delete").await.unwrap();

    assert!(logs_contain("scan deleted"));
}

#[tokio::test]
async fn get_scan_returns_full_scan_record() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-read", ScanStatus::Stored))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage);

    let scan = service.get_scan("scan-read").await.unwrap();

    assert_eq!(scan.id, "scan-read");
    assert_eq!(scan.status, ScanStatus::Stored);
    assert_eq!(scan.target.hosts, vec!["https://example.test".to_string()]);
}

#[tokio::test]
async fn get_scan_maps_missing_scan_to_scan_not_found() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    let service = DefaultScanService::new_storage_only(storage);

    let err = service.get_scan("missing-scan").await.unwrap_err();

    assert!(matches!(err, ScanServiceError::ScanNotFound(id) if id == "missing-scan"));
}

#[tokio::test]
async fn get_scan_result_returns_persisted_result() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-result", ScanStatus::Stored))
        .await
        .unwrap();
    storage
        .add_result(
            "scan-result",
            ResultRecord {
                id: 0,
                scan_id: "scan-result".to_string(),
                result_type: ResultType::Log,
                ip_address: None,
                hostname: Some("example.test".to_string()),
                oid: None,
                port: None,
                protocol: None,
                message: Some("ok".to_string()),
                detail: None,
            },
        )
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage);

    let result = service.get_scan_result("scan-result", 0).await.unwrap();

    assert_eq!(result.id, 0);
    assert_eq!(result.hostname.as_deref(), Some("example.test"));
}

#[tokio::test]
async fn get_scan_result_for_missing_index_forwards_storage_error() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-result-miss", ScanStatus::Stored))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage);

    let err = service
        .get_scan_result("scan-result-miss", 5)
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ScanServiceError::Storage(StorageError::ResultNotFound(id, 5)) if id == "scan-result-miss"
    ));
}

#[tokio::test]
async fn get_scan_status_returns_status_and_timestamps() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    let mut scan = make_scan("scan-status", ScanStatus::Running);
    scan.start_time = Some(100);
    scan.end_time = Some(120);
    storage.create_scan(scan).await.unwrap();
    let service = DefaultScanService::new_storage_only(storage);

    let status_view = service.get_scan_status("scan-status").await.unwrap();

    assert_eq!(status_view.status, ScanStatus::Running);
    assert_eq!(status_view.start_time, Some(100));
    assert_eq!(status_view.end_time, Some(120));
}

#[tokio::test]
async fn recover_interrupted_scans_transitions_non_terminal_runtime_states_to_failed() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-stored-recovery", ScanStatus::Stored))
        .await
        .unwrap();
    storage
        .create_scan(make_scan("scan-requested-recovery", ScanStatus::Requested))
        .await
        .unwrap();
    storage
        .create_scan(make_scan("scan-running-recovery", ScanStatus::Running))
        .await
        .unwrap();
    let service = DefaultScanService::new_storage_only(storage.clone());

    service.recover_interrupted_scans().await.unwrap();

    let stored = storage.get_scan("scan-stored-recovery").await.unwrap();
    let requested = storage.get_scan("scan-requested-recovery").await.unwrap();
    let running = storage.get_scan("scan-running-recovery").await.unwrap();

    assert_eq!(stored.status, ScanStatus::Stored);
    assert_eq!(requested.status, ScanStatus::Failed);
    assert_eq!(running.status, ScanStatus::Failed);
}
