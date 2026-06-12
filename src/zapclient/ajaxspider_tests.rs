// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use reqwest::StatusCode;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, method, path},
};

use super::{AjaxSpiderStatus, ZapClient, ZapClientError};

const API_KEY: &str = "test-api-key";

#[tokio::test]
async fn start_ajax_spider_scan_posts_to_zap_ajax_scan_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("url=https%3A%2F%2Fexample.com"))
        .and(body_string_contains("inScope=true"))
        .and(body_string_contains("contextName=Default+Context"))
        .and(body_string_contains("subtreeOnly=false"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    client
        .start_ajax_spider_scan("Default Context", "https://example.com", true, false)
        .await
        .expect("start_ajax_spider_scan should succeed when Result is OK");
}

#[tokio::test]
async fn start_ajax_spider_scan_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .start_ajax_spider_scan("Default Context", "https://example.com", false, true)
        .await
        .expect_err("start_ajax_spider_scan should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn start_ajax_spider_scan_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .start_ajax_spider_scan("Default Context", "https://example.com", true, true)
        .await
        .expect_err("start_ajax_spider_scan should fail when Result key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn start_ajax_spider_scan_returns_unexpected_content_when_result_is_not_ok() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"FAIL\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .start_ajax_spider_scan("Default Context", "https://example.com", true, false)
        .await
        .expect_err("start_ajax_spider_scan should fail when Result is not OK");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "Result");
            assert_eq!(content, "FAIL");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_ajax_spider_status_posts_to_zap_ajax_status_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"running\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let status = client
        .get_ajax_spider_status()
        .await
        .expect("get_ajax_spider_status should return parsed status on success");

    assert_eq!(status, AjaxSpiderStatus::Running);
}

#[tokio::test]
async fn get_ajax_spider_status_accepts_lowercase_stopped() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"stopped\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let status = client
        .get_ajax_spider_status()
        .await
        .expect("get_ajax_spider_status should accept lowercase status values");

    assert_eq!(status, AjaxSpiderStatus::Stopped);
}

#[tokio::test]
async fn get_ajax_spider_status_rejects_uppercase_status() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"Stopped\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_ajax_spider_status()
        .await
        .expect_err("get_ajax_spider_status should fail on non-lowercase status values");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "status");
            assert_eq!(content, "Stopped");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_ajax_spider_status_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_ajax_spider_status()
        .await
        .expect_err("get_ajax_spider_status should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_ajax_spider_status_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"progress\":\"100\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_ajax_spider_status()
        .await
        .expect_err("get_ajax_spider_status should fail when status key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_ajax_spider_status_returns_unexpected_content_for_unknown_status() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/view/status"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"100\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_ajax_spider_status()
        .await
        .expect_err("get_ajax_spider_status should fail on unknown status values");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "status");
            assert_eq!(content, "100");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn stop_ajax_spider_scan_posts_to_zap_ajax_stop_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/stop"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    client
        .stop_ajax_spider_scan()
        .await
        .expect("stop_ajax_spider_scan should succeed when Result is OK");
}

#[tokio::test]
async fn stop_ajax_spider_scan_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/ajaxSpider/action/stop"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .stop_ajax_spider_scan()
        .await
        .expect_err("stop_ajax_spider_scan should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}
