// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    Json,
    body::to_bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    api::dto::scans::{
        HostInfo, HostScanningEntry, PreferencesResponse, ResultType, ScanAction,
        ScanActionRequest, ScanRequest, Target,
    },
    app::AppState,
    scan::{
        CreateScanRequest, Scan, ScanProgress, ScanResult, ScanService, ScanServiceError,
        ScanStatus, ScanStatusView,
    },
    storage::{ResultRecord, ScanRecord, ScanStorage, StorageError},
};

use super::{create_scan, get_scan, get_scan_status, progress_to_host_info, scan_action};

#[derive(Default)]
struct NullStorage;

#[async_trait]
impl ScanStorage for NullStorage {
    async fn create_scan(&self, _scan: ScanRecord) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn get_scan(&self, _id: &str) -> Result<ScanRecord, StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn list_non_terminal_scans(&self) -> Result<Vec<ScanRecord>, StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn update_scan_status(&self, _id: &str, _status: ScanStatus) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn transition_scan_status(
        &self,
        _id: &str,
        _expected: ScanStatus,
        _new_status: ScanStatus,
    ) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn update_scan_progress(
        &self,
        _id: &str,
        _progress: Option<serde_json::Value>,
    ) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn update_scan_context(
        &self,
        _id: &str,
        _context_name: Option<String>,
        _context_id: Option<String>,
    ) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn update_alert_cursor(
        &self,
        _id: &str,
        _alert_cursor: Option<i64>,
    ) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn update_scan_stop_requested(
        &self,
        _id: &str,
        _stop_requested: bool,
    ) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn delete_scan(&self, _id: &str) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn add_result(&self, _scan_id: &str, _result: ResultRecord) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn add_results(
        &self,
        _scan_id: &str,
        _results: Vec<ResultRecord>,
    ) -> Result<(), StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn get_result(&self, _scan_id: &str, _result_id: i64) -> Result<ResultRecord, StorageError> {
        panic!("scan handlers must not use storage directly")
    }

    async fn get_results(
        &self,
        _scan_id: &str,
        _start: usize,
        _end: Option<usize>,
    ) -> Result<Vec<ResultRecord>, StorageError> {
        panic!("scan handlers must not use storage directly")
    }
}

#[derive(Default)]
struct MockScanService {
    calls: Mutex<Vec<String>>,
}

impl MockScanService {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }
}

#[async_trait]
impl ScanService for MockScanService {
    async fn recover_interrupted_scans(&self) -> Result<(), ScanServiceError> {
        self.record("recover_interrupted_scans".to_string());
        Ok(())
    }

    async fn get_default_preferences(&self) -> Result<PreferencesResponse, ScanServiceError> {
        self.record("get_default_preferences".to_string());
        Ok(PreferencesResponse::default())
    }

    async fn create_scan(&self, _request: CreateScanRequest) -> Result<String, ScanServiceError> {
        self.record("create_scan".to_string());
        Ok("scan-from-service".to_string())
    }

    async fn get_scan(&self, id: &str) -> Result<Scan, ScanServiceError> {
        self.record(format!("get_scan:{id}"));
        Ok(Scan {
            id: id.to_string(),
            target: Target {
                hosts: vec!["https://example.test".to_string()],
                excluded_hosts: vec![],
                credentials: vec![],
            },
            scan_preferences: vec![],
            vts: vec![],
            status: ScanStatus::Stored,
            stop_requested: false,
            queued_time: None,
            start_time: None,
            end_time: None,
            context_name: None,
            context_id: None,
            alert_cursor: None,
            progress: None,
            interruption_reason: None,
        })
    }

    async fn get_scan_result(
        &self,
        id: &str,
        result_id: i64,
    ) -> Result<ScanResult, ScanServiceError> {
        self.record(format!("get_scan_result:{id}:{result_id}"));
        Ok(ScanResult {
            id: result_id,
            scan_id: id.to_string(),
            result_type: ResultType::Log,
            ip_address: None,
            hostname: Some("example.test".to_string()),
            oid: None,
            port: None,
            protocol: None,
            message: Some("ok".to_string()),
            detail: None,
        })
    }

    async fn get_scan_status(&self, id: &str) -> Result<ScanStatusView, ScanServiceError> {
        self.record(format!("get_scan_status:{id}"));
        Ok(ScanStatusView {
            status: ScanStatus::Running,
            start_time: Some(10),
            end_time: None,
            progress: None,
        })
    }

    async fn start_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        self.record(format!("start_scan:{id}"));
        Ok(())
    }

    async fn stop_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        self.record(format!("stop_scan:{id}"));
        Ok(())
    }

    async fn delete_scan(&self, id: &str) -> Result<(), ScanServiceError> {
        self.record(format!("delete_scan:{id}"));
        Ok(())
    }

    async fn get_results(
        &self,
        id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ScanResult>, ScanServiceError> {
        self.record(format!("get_results:{id}:{start}:{end:?}"));
        Ok(vec![])
    }
}

fn make_state(service: Arc<MockScanService>) -> AppState {
    AppState::new(Arc::new(NullStorage), service)
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn make_progress(hosts: &[&str]) -> ScanProgress {
    let hosts: Vec<String> = hosts.iter().map(|s| s.to_string()).collect();
    ScanProgress::new(&hosts)
}

// ─── all targets pending ──────────────────────────────────────────────────────

#[test]
fn all_pending_targets_are_queued() {
    let progress = make_progress(&["http://a.example", "http://b.example"]);
    let info = progress_to_host_info(&progress);

    assert_eq!(info.all, 2);
    assert_eq!(info.queued, 2);
    assert_eq!(info.finished, 0);
    assert!(info.scanning.is_empty());
}

#[test]
fn alive_equals_all() {
    let progress = make_progress(&["http://a.example", "http://b.example"]);
    let info = progress_to_host_info(&progress);

    assert_eq!(info.alive, info.all);
}

#[test]
fn excluded_and_dead_are_zero() {
    let progress = make_progress(&["http://a.example"]);
    let info = progress_to_host_info(&progress);

    assert_eq!(info.excluded, 0);
    assert_eq!(info.dead, 0);
}

// ─── spider running ───────────────────────────────────────────────────────────

#[test]
fn spider_running_target_appears_in_scanning_with_progress_1() {
    let mut progress = make_progress(&["http://a.example"]);
    progress.mark_spider_running(0);

    let info = progress_to_host_info(&progress);

    assert_eq!(info.queued, 0);
    assert_eq!(info.finished, 0);
    assert_eq!(
        info.scanning,
        vec![HostScanningEntry {
            host: "http://a.example".to_string(),
            progress: 1,
        }]
    );
}

// ─── active scan running ──────────────────────────────────────────────────────

#[test]
fn active_scan_running_target_appears_in_scanning_with_formula_progress() {
    let mut progress = make_progress(&["http://a.example"]);
    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_running(0);
    progress.update_active_scan(0, 50);

    let info = progress_to_host_info(&progress);

    // floor(25 + 0.75 * 50) = 62
    assert_eq!(info.scanning.len(), 1);
    assert_eq!(info.scanning[0].host, "http://a.example");
    assert_eq!(info.scanning[0].progress, 62);
    assert_eq!(info.queued, 0);
    assert_eq!(info.finished, 0);
}

// ─── active scan done ─────────────────────────────────────────────────────────

#[test]
fn active_scan_done_target_counted_as_finished() {
    let mut progress = make_progress(&["http://a.example"]);
    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);

    let info = progress_to_host_info(&progress);

    assert_eq!(info.finished, 1);
    assert_eq!(info.queued, 0);
    assert!(info.scanning.is_empty());
}

// ─── mixed targets ────────────────────────────────────────────────────────────

#[test]
fn mixed_targets_populate_queued_scanning_and_finished_correctly() {
    let mut progress = make_progress(&[
        "http://pending.example",
        "http://spider.example",
        "http://active.example",
        "http://done.example",
    ]);

    // index 0: stays pending (queued)
    // index 1: spider running (scanning, progress = 1)
    progress.mark_spider_running(1);
    // index 2: spider done, active scan at 40% (scanning, progress = floor(25 + 30) = 55)
    progress.mark_spider_running(2);
    progress.mark_spider_done(2);
    progress.mark_active_scan_running(2);
    progress.update_active_scan(2, 40);
    // index 3: fully finished
    progress.mark_spider_running(3);
    progress.mark_spider_done(3);
    progress.mark_active_scan_done(3);

    let info = progress_to_host_info(&progress);

    assert_eq!(info.all, 4);
    assert_eq!(info.queued, 1);
    assert_eq!(info.finished, 1);
    assert_eq!(info.scanning.len(), 2);
    assert_eq!(info.alive, 4);

    let spider_entry = info
        .scanning
        .iter()
        .find(|e| e.host == "http://spider.example")
        .expect("spider.example should be in scanning");
    assert_eq!(spider_entry.progress, 1);

    let active_entry = info
        .scanning
        .iter()
        .find(|e| e.host == "http://active.example")
        .expect("active.example should be in scanning");
    assert_eq!(active_entry.progress, 55);
}

// ─── empty target list ────────────────────────────────────────────────────────

#[test]
fn empty_progress_produces_zeroed_host_info() {
    let progress = make_progress(&[]);
    let info = progress_to_host_info(&progress);

    assert_eq!(
        info,
        HostInfo {
            all: 0,
            excluded: 0,
            dead: 0,
            alive: 0,
            queued: 0,
            finished: 0,
            scanning: vec![],
        }
    );
}

#[tokio::test]
async fn get_scan_handler_uses_scan_service_facade() {
    let service = Arc::new(MockScanService::default());
    let response = get_scan(
        State(make_state(service.clone())),
        Path("scan-123".to_string()),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_text = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_text.contains("scan-123"));
    assert_eq!(service.calls(), vec!["get_scan:scan-123".to_string()]);
}

#[tokio::test]
async fn get_scan_status_handler_uses_scan_service_facade() {
    let service = Arc::new(MockScanService::default());
    let response = get_scan_status(
        State(make_state(service.clone())),
        Path("scan-789".to_string()),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(service.calls(), vec!["get_scan_status:scan-789".to_string()]);
}

#[tokio::test]
async fn scan_action_start_handler_uses_scan_service_facade() {
    let service = Arc::new(MockScanService::default());
    let response = scan_action(
        State(make_state(service.clone())),
        Path("scan-start".to_string()),
        Json(ScanActionRequest {
            action: ScanAction::Start,
        }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(service.calls(), vec!["start_scan:scan-start".to_string()]);
}

#[tokio::test]
async fn create_scan_handler_uses_scan_service_facade() {
    let service = Arc::new(MockScanService::default());
    let response = create_scan(
        State(make_state(service.clone())),
        Json(ScanRequest {
            target: Target {
                hosts: vec!["https://example.test".to_string()],
                excluded_hosts: vec![],
                credentials: vec![],
            },
            scan_preferences: vec![],
            vts: vec![],
        }),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(service.calls(), vec!["create_scan".to_string()]);
}
