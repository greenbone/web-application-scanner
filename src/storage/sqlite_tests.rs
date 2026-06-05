// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::api::dto::scans::ResultType;
use crate::config::settings::SQLITE_IN_MEMORY_URL;

async fn make_storage() -> SqliteStorage {
    SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap()
}

fn make_scan(id: &str) -> ScanRecord {
    ScanRecord {
        id: id.to_string(),
        target: Target {
            hosts: vec!["10.0.0.1".to_string()],
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
        result_type: ResultType::Alarm,
        ip_address: Some("10.0.0.1".to_string()),
        hostname: None,
        oid: Some("1.3.6.1".to_string()),
        port: Some(80),
        protocol: Some("tcp".to_string()),
        message: Some("found".to_string()),
        detail: None,
    }
}

#[tokio::test]
async fn create_and_get_scan() {
    let s = make_storage().await;
    s.create_scan(make_scan("s1")).await.unwrap();
    let scan = s.get_scan("s1").await.unwrap();
    assert_eq!(scan.id, "s1");
    assert_eq!(scan.status, ScanStatus::Stored);
    assert_eq!(scan.target.hosts, vec!["10.0.0.1"]);
}

#[tokio::test]
async fn duplicate_scan_returns_already_exists() {
    let s = make_storage().await;
    s.create_scan(make_scan("dup")).await.unwrap();
    let err = s.create_scan(make_scan("dup")).await.unwrap_err();
    assert!(matches!(err, StorageError::AlreadyExists(_)));
}

#[tokio::test]
async fn get_missing_scan_returns_not_found() {
    let s = make_storage().await;
    assert!(matches!(
        s.get_scan("missing").await.unwrap_err(),
        StorageError::NotFound(_)
    ));
}

#[tokio::test]
async fn update_and_read_status() {
    let s = make_storage().await;
    s.create_scan(make_scan("st")).await.unwrap();
    s.update_scan_status("st", ScanStatus::Running)
        .await
        .unwrap();
    let scan = s.get_scan("st").await.unwrap();
    assert_eq!(scan.status, ScanStatus::Running);
}

#[tokio::test]
async fn delete_removes_scan_and_results() {
    let s = make_storage().await;
    s.create_scan(make_scan("del")).await.unwrap();
    s.add_result("del", make_result("del")).await.unwrap();
    s.delete_scan("del").await.unwrap();
    assert!(matches!(
        s.get_scan("del").await.unwrap_err(),
        StorageError::NotFound(_)
    ));
}

#[tokio::test]
async fn results_auto_increment() {
    let s = make_storage().await;
    s.create_scan(make_scan("ri")).await.unwrap();
    s.add_result("ri", make_result("ri")).await.unwrap();
    s.add_result("ri", make_result("ri")).await.unwrap();
    let results = s.get_results("ri", 0, None).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 0);
    assert_eq!(results[1].id, 1);
}

#[tokio::test]
async fn results_range_query() {
    let s = make_storage().await;
    s.create_scan(make_scan("rq")).await.unwrap();
    for _ in 0..5 {
        s.add_result("rq", make_result("rq")).await.unwrap();
    }
    let results = s.get_results("rq", 1, Some(3)).await.unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[2].id, 3);
}
