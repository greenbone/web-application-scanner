// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use crate::{
    api::dto::scans::{ResultType, Target},
    config::settings::SQLITE_IN_MEMORY_URL,
    scan::{ScanResult, ScanStateCoordinator, ScanStatus},
    storage::{ScanRecord, ScanStorage, sqlite::SqliteStorage},
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

#[tokio::test]
async fn transition_status_delegates_to_transition_executor() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-delegate-transition", ScanStatus::Stored))
        .await
        .unwrap();
    let coordinator = ScanStateCoordinator::new(storage.clone());

    coordinator
        .transition_status(
            "scan-delegate-transition",
            ScanStatus::Stored,
            ScanStatus::Requested,
        )
        .await
        .unwrap();

    let updated = storage.get_scan("scan-delegate-transition").await.unwrap();
    assert_eq!(updated.status, ScanStatus::Requested);
}

#[tokio::test]
async fn persist_alert_batch_delegates_to_execution_state_executor() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    storage
        .create_scan(make_scan("scan-delegate-exec", ScanStatus::Running))
        .await
        .unwrap();
    let coordinator = ScanStateCoordinator::new(storage.clone());

    coordinator
        .persist_alert_batch(
            "scan-delegate-exec",
            1,
            vec![ScanResult {
                id: 0,
                scan_id: "scan-delegate-exec".to_string(),
                result_type: ResultType::Alarm,
                ip_address: Some("https://example.test/path".to_string()),
                hostname: Some("example.test".to_string()),
                oid: Some("10001".to_string()),
                port: Some(443),
                protocol: Some("tcp".to_string()),
                message: Some("finding".to_string()),
                detail: None,
            }],
        )
        .await
        .unwrap();

    let scan = storage.get_scan("scan-delegate-exec").await.unwrap();
    let results = storage.get_results("scan-delegate-exec", 0, None).await.unwrap();

    assert_eq!(scan.alert_cursor, Some(1));
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].result_type, ResultType::Alarm));
}
