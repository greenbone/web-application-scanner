// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use reqwest::StatusCode;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, method, path},
};

use super::{ZapClient, ZapClientError};

const API_KEY: &str = "test-api-key";

async fn mount_ascan_scan(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

async fn mount_ascan_status(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

async fn mount_ascan_stop(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/stop"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn start_active_scan_posts_to_zap_ascan_scan_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/scan"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("url=https%3A%2F%2Fexample.com"))
        .and(body_string_contains("recurse=true"))
        .and(body_string_contains("inScopeOnly=false"))
        .and(body_string_contains("contextId=3"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"scan\":\"7\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let scan_id = client
        .start_active_scan("3", "https://example.com", true, false)
        .await
        .expect("start_active_scan should return parsed scan id on success");

    assert_eq!(scan_id, "7");
}

#[tokio::test]
async fn start_active_scan_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    mount_ascan_scan(
        &server,
        ResponseTemplate::new(500).set_body_string("zap unavailable"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .start_active_scan("3", "https://example.com", false, true)
        .await
        .expect_err("start_active_scan should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn start_active_scan_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;

    mount_ascan_scan(
        &server,
        ResponseTemplate::new(200).set_body_string("{\"scanId\":\"7\"}"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .start_active_scan("3", "https://example.com", true, true)
        .await
        .expect_err("start_active_scan should fail when scan key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_active_scan_status_gets_zap_ascan_status_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/view/status"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("scanId=7"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"42\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let status = client
        .get_active_scan_status("7")
        .await
        .expect("get_active_scan_status should return parsed status on success");

    assert_eq!(status, 42);
}

#[tokio::test]
async fn get_active_scan_status_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    mount_ascan_status(
        &server,
        ResponseTemplate::new(500).set_body_string("zap unavailable"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_active_scan_status("7")
        .await
        .expect_err("get_active_scan_status should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_active_scan_status_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;

    mount_ascan_status(
        &server,
        ResponseTemplate::new(200).set_body_string("{\"progress\":42}"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_active_scan_status("7")
        .await
        .expect_err("get_active_scan_status should fail when status key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_active_scan_status_returns_unexpected_content_for_out_of_range_status() {
    let server = MockServer::start().await;

    mount_ascan_status(
        &server,
        ResponseTemplate::new(200).set_body_string("{\"status\":\"101\"}"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_active_scan_status("7")
        .await
        .expect_err("get_active_scan_status should fail when status is out of range");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "status");
            assert_eq!(content, "101");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn stop_active_scan_posts_to_zap_ascan_stop_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ascan/action/stop"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("scanId=7"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    client
        .stop_active_scan("7")
        .await
        .expect("stop_active_scan should succeed when Result is OK");
}

#[tokio::test]
async fn stop_active_scan_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    mount_ascan_stop(
        &server,
        ResponseTemplate::new(500).set_body_string("zap unavailable"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .stop_active_scan("7")
        .await
        .expect_err("stop_active_scan should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}
