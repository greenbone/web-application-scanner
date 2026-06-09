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
        status: ScanStatus::New,
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

fn make_scan_with_status(id: &str, status: ScanStatus) -> ScanRecord {
    let mut scan = make_scan(id);
    scan.status = status;
    scan
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
    assert_eq!(scan.status, ScanStatus::New);
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
async fn transition_scan_status_requires_expected_state() {
    let s = make_storage().await;
    s.create_scan(make_scan("cas")).await.unwrap();

    s.transition_scan_status("cas", ScanStatus::New, ScanStatus::Queued)
        .await
        .unwrap();

    let err = s
        .transition_scan_status("cas", ScanStatus::New, ScanStatus::Running)
        .await
        .unwrap_err();

    assert!(matches!(err, StorageError::InvalidState));
}

#[tokio::test]
async fn update_scan_context_progress_and_cursor_roundtrip() {
    let s = make_storage().await;
    s.create_scan(make_scan("meta")).await.unwrap();

    s.update_scan_context(
        "meta",
        Some("greenbone-was-meta".to_string()),
        Some("ctx-1".to_string()),
    )
    .await
    .unwrap();
    s.update_scan_progress(
        "meta",
        Some(serde_json::json!({"overall": 42, "targets": []})),
    )
    .await
    .unwrap();
    s.update_alert_cursor("meta", Some(7)).await.unwrap();

    let scan = s.get_scan("meta").await.unwrap();
    assert_eq!(scan.context_name.as_deref(), Some("greenbone-was-meta"));
    assert_eq!(scan.context_id.as_deref(), Some("ctx-1"));
    assert_eq!(scan.alert_cursor, Some(7));
    assert_eq!(
        scan.progress,
        Some(serde_json::json!({"overall": 42, "targets": []}))
    );
}

#[tokio::test]
async fn batch_results_persist_atomically() {
    let s = make_storage().await;
    s.create_scan(make_scan("batch")).await.unwrap();

    s.add_results("batch", vec![make_result("batch"), make_result("batch")])
        .await
        .unwrap();

    let results = s.get_results("batch", 0, None).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 0);
    assert_eq!(results[1].id, 1);
}

#[tokio::test]
async fn list_non_terminal_scans_excludes_terminal_states() {
    let s = make_storage().await;
    s.create_scan(make_scan_with_status("new", ScanStatus::New))
        .await
        .unwrap();
    s.create_scan(make_scan_with_status("done", ScanStatus::Done))
        .await
        .unwrap();

    let scans = s.list_non_terminal_scans().await.unwrap();
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].id, "new");
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
