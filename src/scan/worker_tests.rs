// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;
use tracing_test::traced_test;

use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, method, path},
};

use crate::{
    api::dto::scans::{ResultType, ScannerPreference, Target},
    scan::{
        CreateScanRequest, DefaultScanService, ScanRuntimeConfig, ScanService, ScanStatus,
        start_scan_runtime,
    },
    storage::{ScanStorage, test_support::temporary_sqlite_storage},
    zapclient::ZapClient,
};

fn make_request(host: &str) -> CreateScanRequest {
    CreateScanRequest {
        scan_id: None,
        target: Target {
            hosts: vec![host.to_string()],
            excluded_hosts: vec![],
            credentials: vec![],
        },
        scan_preferences: vec![ScannerPreference {
            id: "scan_mode".to_string(),
            value: "active".to_string(),
        }],
        vts: vec![],
    }
}

fn make_safe_mode_request(host: &str) -> CreateScanRequest {
    CreateScanRequest {
        scan_id: None,
        target: Target {
            hosts: vec![host.to_string()],
            excluded_hosts: vec![],
            credentials: vec![],
        },
        scan_preferences: vec![ScannerPreference {
            id: "scan_mode".to_string(),
            value: "safe".to_string(),
        }],
        vts: vec![],
    }
}

fn make_safe_mode_request_with_ajax_timeout(host: &str, timeout_seconds: u64) -> CreateScanRequest {
    CreateScanRequest {
        scan_id: None,
        target: Target {
            hosts: vec![host.to_string()],
            excluded_hosts: vec![],
            credentials: vec![],
        },
        scan_preferences: vec![
            ScannerPreference {
                id: "scan_mode".to_string(),
                value: "safe".to_string(),
            },
            ScannerPreference {
                id: "ajax_spider_timeout".to_string(),
                value: timeout_seconds.to_string(),
            },
        ],
        vts: vec![],
    }
}

async fn mount_ajax_spider_set_option_max_duration_ok(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/setOptionMaxDuration"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(server)
        .await;
}

async fn mock_zap_server() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"100"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .and(body_string_contains("start=0"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"alerts":[{"alertRef":"10001","name":"Finding","risk":"Low","description":"detail","url":"https://example.test/app"}]}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .and(body_string_contains("start=1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_safe_mode_without_active_scan_requests() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"100"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_for_ajax_spider_timeout_enforcement() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/setOptionMaxDuration"))
        .and(body_string_contains("Integer=1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/stop"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"100"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_for_unlimited_ajax_spider_timeout() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/setOptionMaxDuration"))
        .and(body_string_contains("Integer=0"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/stop"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"100"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_for_default_ajax_spider_timeout() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/setOptionMaxDuration"))
        .and(body_string_contains("Integer=3600"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"100"}"#, "application/json"),
        )
        .expect(0)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_with_active_status_error() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(500).set_body_raw(r#"{"code":"internal"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_with_remove_context_error() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"100"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .and(body_string_contains("start=0"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"alerts":[{"alertRef":"10001","name":"Finding","risk":"Low","description":"detail","url":"https://example.test/app"}]}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .and(body_string_contains("start=1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(500).set_body_raw(r#"{"code":"internal"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_for_running_stop_in_active_scan() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"10"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/stop"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_for_running_stop_in_spider() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"running"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/stop"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_for_running_stop_in_active_stage_with_stop_failure() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"10"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/stop"))
        .respond_with(
            ResponseTemplate::new(500).set_body_raw(r#"{"code":"internal"}"#, "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn mock_zap_server_for_forced_stop_timeout() -> MockServer {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"contextId":"ctx-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    mount_ajax_spider_set_option_max_duration_ok(&server).await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"status":"stopped"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"scan":"active-1"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(200))
                .set_body_raw(r#"{"status":"10"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"alerts":[]}"#, "application/json"),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(r#"{"Result":"OK"}"#, "application/json"),
        )
        .mount(&server)
        .await;

    server
}

async fn wait_for_running(storage: &dyn ScanStorage, scan_id: &str) {
    for _ in 0..200 {
        let scan = storage.get_scan(scan_id).await.unwrap();
        if scan.status == ScanStatus::Running {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("scan did not reach running status");
}

async fn wait_for_status(storage: &dyn ScanStorage, scan_id: &str, expected: ScanStatus) {
    for _ in 0..200 {
        let scan = storage.get_scan(scan_id).await.unwrap();
        if scan.status == expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("scan did not reach expected status");
}

async fn wait_for_passive_running(storage: &dyn ScanStorage, scan_id: &str) {
    for _ in 0..200 {
        let scan = storage.get_scan(scan_id).await.unwrap();
        let passive_state = scan
            .progress
            .as_ref()
            .and_then(|progress| progress.pointer("/targets/0/passive_scan_state"))
            .and_then(serde_json::Value::as_str);
        if passive_state == Some("running") {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("scan did not reach passive running state");
}

async fn wait_for_request_path(server: &MockServer, expected_path: &str) {
    for _ in 0..200 {
        let seen = server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .any(|request| request.url.path() == expected_path);

        if seen {
            return;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    panic!("mock server did not receive expected request path: {expected_path}");
}

#[traced_test]
#[tokio::test]
async fn runtime_processes_requested_scan_to_succeeded_and_persists_alert_results() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(300),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Succeeded).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    let expected_context_name = format!("greenbone-was-{scan_id}");
    assert_eq!(scan.status, ScanStatus::Succeeded);
    assert_eq!(
        scan.context_name.as_deref(),
        Some(expected_context_name.as_str())
    );
    assert_eq!(scan.context_id.as_deref(), Some("ctx-1"));
    assert_eq!(scan.alert_cursor, Some(1));
    assert!(scan.progress.is_some());

    let results = storage.get_results(&scan_id, 0, None).await.unwrap();
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].result_type, ResultType::Alarm));
    assert_eq!(
        results[0].ip_address.as_deref(),
        Some("https://example.test")
    );
    assert_eq!(results[0].oid.as_deref(), Some("ZAP-10001"));
    assert_eq!(results[0].hostname.as_deref(), Some("example.test"));
    assert_eq!(results[0].port, Some(443));
    assert_eq!(results[0].protocol.as_deref(), Some("tcp"));
    assert!(logs_contain("scan status transition"));
    assert!(logs_contain("scan_queue_wait_seconds"));
}

#[traced_test]
#[tokio::test]
async fn runtime_skips_active_scan_when_scan_mode_is_safe() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_safe_mode_without_active_scan_requests().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(300),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_safe_mode_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Succeeded).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Succeeded);
    assert!(logs_contain("active scan skipped due to scan_mode=safe"));
}

#[traced_test]
#[tokio::test]
async fn runtime_sets_ajax_spider_timeout_option_and_continues_scan_flow() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_for_ajax_spider_timeout_enforcement().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(20),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(300),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_safe_mode_request_with_ajax_timeout(
            "https://example.test",
            1,
        ))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Succeeded).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Succeeded);
}

#[tokio::test]
async fn runtime_treats_zero_ajax_spider_timeout_as_unlimited() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_for_unlimited_ajax_spider_timeout().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(20),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(300),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_safe_mode_request_with_ajax_timeout(
            "https://example.test",
            0,
        ))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Succeeded).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Succeeded);
}

#[tokio::test]
async fn runtime_applies_default_ajax_spider_timeout_when_preference_is_omitted() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_for_default_ajax_spider_timeout().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(20),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(300),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_safe_mode_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Succeeded).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Succeeded);
}

#[tokio::test]
async fn runtime_transitions_running_scan_to_failed_on_worker_error() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_with_active_status_error().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(300),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Failed).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Failed);
}

#[tokio::test]
async fn runtime_keeps_succeeded_status_when_context_cleanup_fails() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_with_remove_context_error().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(300),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Succeeded).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Succeeded);
}

#[tokio::test]
async fn runtime_with_multiple_workers_processes_multiple_scans() {
    // --- Skip this test until scan worker concurrency is supported ---

    /*
        let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
        let server = mock_zap_server().await;
        let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
        let runtime = start_scan_runtime(
            storage.clone(),
            zap_client,
            ScanRuntimeConfig {
                worker_count: 2,
                alert_poll_interval: Duration::from_millis(1),
                scan_poll_interval: Duration::from_millis(1),
                alert_page_size: 100,
            },
        );
        let service = DefaultScanService::new(storage.clone(), runtime);

        let scan_id_1 = service
            .create_scan(make_request("https://example.test"))
            .await
            .unwrap();
        let scan_id_2 = service
            .create_scan(make_request("https://example-two.test"))
            .await
            .unwrap();

        service.start_scan(&scan_id_1).await.unwrap();
        service.start_scan(&scan_id_2).await.unwrap();

        wait_for_status(storage.as_ref(), &scan_id_1, ScanStatus::Succeeded).await;
        wait_for_status(storage.as_ref(), &scan_id_2, ScanStatus::Succeeded).await;

        let scan_1 = storage.get_scan(&scan_id_1).await.unwrap();
        let scan_2 = storage.get_scan(&scan_id_2).await.unwrap();
        assert_eq!(scan_1.status, ScanStatus::Succeeded);
        assert_eq!(scan_2.status, ScanStatus::Succeeded);
    */
}

#[tokio::test]
async fn runtime_stop_running_scan_in_active_stage_transitions_to_stopped_and_clears_stop_requested()
 {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_for_running_stop_in_active_scan().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(5),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_running(storage.as_ref(), &scan_id).await;
    wait_for_request_path(&server, "/JSON/ascan/action/scan").await;

    service.stop_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Stopped).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Stopped);
    assert!(!scan.stop_requested);
}

#[tokio::test]
async fn runtime_stop_running_scan_in_spider_stage_transitions_to_stopped_and_clears_stop_requested()
 {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_for_running_stop_in_spider().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(5),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_running(storage.as_ref(), &scan_id).await;
    wait_for_request_path(&server, "/JSON/ajaxSpider/view/status").await;

    service.stop_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Stopped).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Stopped);
    assert!(!scan.stop_requested);
}

#[tokio::test]
async fn runtime_stop_running_scan_in_passive_stage_transitions_to_stopped_and_clears_stop_requested()
 {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_secs(2),
            stop_grace_period: Duration::from_secs(5),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_running(storage.as_ref(), &scan_id).await;
    wait_for_passive_running(storage.as_ref(), &scan_id).await;

    service.stop_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Stopped).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Stopped);
    assert!(!scan.stop_requested);
}

#[tokio::test]
async fn runtime_stop_running_scan_fails_when_zap_stop_fails_non_transiently() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_for_running_stop_in_active_stage_with_stop_failure().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_secs(5),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_running(storage.as_ref(), &scan_id).await;
    wait_for_request_path(&server, "/JSON/ascan/action/scan").await;

    service.stop_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Failed).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Failed);
}

#[tokio::test]
async fn runtime_forces_failed_when_stop_grace_period_expires() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let server = mock_zap_server_for_forced_stop_timeout().await;
    let zap_client = ZapClient::new(server.uri(), "test-api-key".to_string()).unwrap();
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: 1,
            alert_poll_interval: Duration::from_millis(1),
            scan_poll_interval: Duration::from_millis(1),
            alert_page_size: 100,
            passive_scan_placeholder_duration: Duration::from_millis(1),
            stop_grace_period: Duration::from_millis(50),
            ..ScanRuntimeConfig::default()
        },
    );
    let service = DefaultScanService::new(storage.clone(), runtime);

    let scan_id = service
        .create_scan(make_request("https://example.test"))
        .await
        .unwrap();

    service.start_scan(&scan_id).await.unwrap();
    wait_for_running(storage.as_ref(), &scan_id).await;
    // Wait until active scan status polling is in-flight before requesting stop,
    // so grace-period failure handling is exercised deterministically.
    wait_for_request_path(&server, "/JSON/ascan/view/status").await;

    service.stop_scan(&scan_id).await.unwrap();
    wait_for_status(storage.as_ref(), &scan_id, ScanStatus::Failed).await;

    let scan = storage.get_scan(&scan_id).await.unwrap();
    assert_eq!(scan.status, ScanStatus::Failed);
}
