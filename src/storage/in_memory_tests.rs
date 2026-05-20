// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::api::dto::scans::{ResultType, Target};

fn make_scan(id: &str) -> ScanRecord {
    ScanRecord {
        id: id.to_string(),
        target: Target {
            hosts: vec!["192.168.0.1".to_string()],
            excluded_hosts: vec![],
            credentials: vec![],
        },
        scan_preferences: vec![],
        vts: vec![],
        status: ScanStatus::Stored,
        start_time: None,
        end_time: None,
    }
}

fn make_result(scan_id: &str) -> ResultRecord {
    ResultRecord {
        id: 0,
        scan_id: scan_id.to_string(),
        result_type: ResultType::Log,
        ip_address: None,
        hostname: None,
        oid: None,
        port: None,
        protocol: None,
        message: Some("test".to_string()),
        detail: None,
    }
}

#[tokio::test]
async fn create_and_get_scan() {
    let storage = InMemoryStorage::new();
    let scan = make_scan("scan-1");
    storage.create_scan(scan).await.unwrap();
    let retrieved = storage.get_scan("scan-1").await.unwrap();
    assert_eq!(retrieved.id, "scan-1");
    assert_eq!(retrieved.status, ScanStatus::Stored);
}

#[tokio::test]
async fn create_scan_duplicate_returns_error() {
    let storage = InMemoryStorage::new();
    storage.create_scan(make_scan("dup")).await.unwrap();
    let err = storage.create_scan(make_scan("dup")).await.unwrap_err();
    assert!(matches!(err, StorageError::AlreadyExists(_)));
}

#[tokio::test]
async fn get_missing_scan_returns_not_found() {
    let storage = InMemoryStorage::new();
    let err = storage.get_scan("missing").await.unwrap_err();
    assert!(matches!(err, StorageError::NotFound(_)));
}

#[tokio::test]
async fn update_status() {
    let storage = InMemoryStorage::new();
    storage.create_scan(make_scan("s")).await.unwrap();
    storage
        .update_scan_status("s", ScanStatus::Running)
        .await
        .unwrap();
    let scan = storage.get_scan("s").await.unwrap();
    assert_eq!(scan.status, ScanStatus::Running);
}

#[tokio::test]
async fn delete_scan_removes_results() {
    let storage = InMemoryStorage::new();
    storage.create_scan(make_scan("del")).await.unwrap();
    storage.add_result("del", make_result("del")).await.unwrap();
    storage.delete_scan("del").await.unwrap();
    let err = storage.get_scan("del").await.unwrap_err();
    assert!(matches!(err, StorageError::NotFound(_)));
}

#[tokio::test]
async fn delete_missing_scan_returns_not_found() {
    let storage = InMemoryStorage::new();
    let err = storage.delete_scan("none").await.unwrap_err();
    assert!(matches!(err, StorageError::NotFound(_)));
}

#[tokio::test]
async fn results_are_auto_indexed() {
    let storage = InMemoryStorage::new();
    storage.create_scan(make_scan("r")).await.unwrap();
    storage.add_result("r", make_result("r")).await.unwrap();
    storage.add_result("r", make_result("r")).await.unwrap();
    let results = storage.get_results("r", 0, None).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 0);
    assert_eq!(results[1].id, 1);
}

#[tokio::test]
async fn get_results_range() {
    let storage = InMemoryStorage::new();
    storage.create_scan(make_scan("rng")).await.unwrap();
    for _ in 0..5 {
        storage.add_result("rng", make_result("rng")).await.unwrap();
    }
    let results = storage.get_results("rng", 1, Some(3)).await.unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[2].id, 3);
}

#[tokio::test]
async fn get_result_by_id() {
    let storage = InMemoryStorage::new();
    storage.create_scan(make_scan("gr")).await.unwrap();
    storage.add_result("gr", make_result("gr")).await.unwrap();
    let r = storage.get_result("gr", 0).await.unwrap();
    assert_eq!(r.id, 0);
}

#[tokio::test]
async fn get_missing_result_returns_error() {
    let storage = InMemoryStorage::new();
    storage.create_scan(make_scan("m")).await.unwrap();
    let err = storage.get_result("m", 99).await.unwrap_err();
    assert!(matches!(err, StorageError::ResultNotFound(_, _)));
}
