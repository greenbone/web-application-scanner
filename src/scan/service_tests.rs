// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use crate::{
    api::dto::scans::Target,
    config::settings::SQLITE_IN_MEMORY_URL,
    scan::{CreateScanRequest, DefaultScanService, ScanService, ScanServiceError, ScanStatus},
    storage::{ScanRecord, ScanStorage, sqlite::SqliteStorage},
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
