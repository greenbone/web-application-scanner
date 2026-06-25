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

async fn mount_records_to_scan(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/JSON/pscan/view/recordsToScan"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn get_passive_scan_records_to_scan_posts_to_zap_pscan_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/pscan/view/recordsToScan"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"recordsToScan\":\"42\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let records_to_scan = client
        .get_passive_scan_records_to_scan()
        .await
        .expect("recordsToScan should parse on success");

    assert_eq!(records_to_scan, 42);
}

#[tokio::test]
async fn get_passive_scan_records_to_scan_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    mount_records_to_scan(
        &server,
        ResponseTemplate::new(503).set_body_string("zap unavailable"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_passive_scan_records_to_scan()
        .await
        .expect_err("recordsToScan call should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_passive_scan_records_to_scan_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;

    mount_records_to_scan(
        &server,
        ResponseTemplate::new(200).set_body_string("{\"records\":\"42\"}"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_passive_scan_records_to_scan()
        .await
        .expect_err("recordsToScan call should fail when recordsToScan key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_passive_scan_records_to_scan_returns_unexpected_content_for_non_numeric_value() {
    let server = MockServer::start().await;

    mount_records_to_scan(
        &server,
        ResponseTemplate::new(200).set_body_string("{\"recordsToScan\":\"abc\"}"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_passive_scan_records_to_scan()
        .await
        .expect_err("recordsToScan call should fail for non-numeric content");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "recordsToScan");
            assert_eq!(content, "abc");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_passive_scan_records_to_scan_returns_unexpected_content_for_negative_value() {
    let server = MockServer::start().await;

    mount_records_to_scan(
        &server,
        ResponseTemplate::new(200).set_body_string("{\"recordsToScan\":\"-1\"}"),
    )
    .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_passive_scan_records_to_scan()
        .await
        .expect_err("recordsToScan call should fail for negative values");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "recordsToScan");
            assert_eq!(content, "-1");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}
