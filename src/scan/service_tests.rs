// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

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
        start_time: None,
        end_time: None,
    }
}

#[tokio::test]
async fn create_scan_persists_new_status() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    let service = DefaultScanService::new(storage.clone());

    let scan_id = service.create_scan(make_request()).await.unwrap();
    let persisted = storage.get_scan(&scan_id).await.unwrap();

    assert_eq!(persisted.id, scan_id);
    assert_eq!(persisted.status, ScanStatus::New);
}

#[tokio::test]
async fn start_scan_transitions_new_to_queued() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("start-scan", ScanStatus::New))
        .await
        .unwrap();
    let service = DefaultScanService::new(storage.clone());

    service.start_scan("start-scan").await.unwrap();

    let persisted = storage.get_scan("start-scan").await.unwrap();
    assert_eq!(persisted.status, ScanStatus::Queued);
}

#[tokio::test]
async fn start_scan_returns_invalid_transition_for_non_new_scans() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("done-scan", ScanStatus::Done))
        .await
        .unwrap();
    let service = DefaultScanService::new(storage);

    let err = service.start_scan("done-scan").await.unwrap_err();

    assert!(matches!(
        err,
        ScanServiceError::InvalidTransition {
            from: ScanStatus::Done,
            requested: ScanStatus::Queued,
        }
    ));
}

#[tokio::test]
async fn stop_scan_transitions_queued_to_stopped() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("queued-scan", ScanStatus::Queued))
        .await
        .unwrap();
    let service = DefaultScanService::new(storage.clone());

    service.stop_scan("queued-scan").await.unwrap();

    let persisted = storage.get_scan("queued-scan").await.unwrap();
    assert_eq!(persisted.status, ScanStatus::Stopped);
}

#[tokio::test]
async fn stop_scan_transitions_running_to_stop_requested() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("running-scan", ScanStatus::Running))
        .await
        .unwrap();
    let service = DefaultScanService::new(storage.clone());

    service.stop_scan("running-scan").await.unwrap();

    let persisted = storage.get_scan("running-scan").await.unwrap();
    assert_eq!(persisted.status, ScanStatus::StopRequested);
}

#[tokio::test]
async fn delete_scan_rejects_non_deletable_states() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("queued-delete", ScanStatus::Queued))
        .await
        .unwrap();
    let service = DefaultScanService::new(storage);

    let err = service.delete_scan("queued-delete").await.unwrap_err();

    assert!(matches!(
        err,
        ScanServiceError::InvalidTransition {
            from: ScanStatus::Queued,
            requested: ScanStatus::Queued,
        }
    ));
}

#[tokio::test]
async fn get_scan_returns_full_scan_record() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-read", ScanStatus::New))
        .await
        .unwrap();
    let service = DefaultScanService::new(storage);

    let scan = service.get_scan("scan-read").await.unwrap();

    assert_eq!(scan.id, "scan-read");
    assert_eq!(scan.status, ScanStatus::New);
    assert_eq!(scan.target.hosts, vec!["https://example.test".to_string()]);
}

#[tokio::test]
async fn get_scan_maps_missing_scan_to_scan_not_found() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    let service = DefaultScanService::new(storage);

    let err = service.get_scan("missing-scan").await.unwrap_err();

    assert!(matches!(err, ScanServiceError::ScanNotFound(id) if id == "missing-scan"));
}

#[tokio::test]
async fn get_scan_result_returns_persisted_result() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-result", ScanStatus::New))
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
    let service = DefaultScanService::new(storage);

    let result = service.get_scan_result("scan-result", 0).await.unwrap();

    assert_eq!(result.id, 0);
    assert_eq!(result.hostname.as_deref(), Some("example.test"));
}

#[tokio::test]
async fn get_scan_result_for_missing_index_forwards_storage_error() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-result-miss", ScanStatus::New))
        .await
        .unwrap();
    let service = DefaultScanService::new(storage);

    let err = service.get_scan_result("scan-result-miss", 5).await.unwrap_err();

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
    let service = DefaultScanService::new(storage);

    let (status, start_time, end_time) = service.get_scan_status("scan-status").await.unwrap();

    assert_eq!(status, ScanStatus::Running);
    assert_eq!(start_time, Some(100));
    assert_eq!(end_time, Some(120));
}
