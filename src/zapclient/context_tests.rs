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

#[tokio::test]
async fn context_list_posts_to_zap_context_list_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/view/contextList"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{\"contextList\":[\"Default Context\",\"Test\"]}"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let response = client
        .get_context_list()
        .await
        .expect("context_list should return parsed response on success");

    assert_eq!(
        response,
        vec!["Default Context".to_string(), "Test".to_string()]
    );
}

#[tokio::test]
async fn context_list_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/view/contextList"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_context_list()
        .await
        .expect_err("context_list should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn context_list_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/view/contextList"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"contexts\":[]}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_context_list()
        .await
        .expect_err("context_list should fail when contextList key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn new_context_posts_to_zap_new_context_endpoint() {
    let server = MockServer::start().await;
    let context_name = "My Context";

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("contextName=My+Context"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"contextId\":\"7\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let context_id = client
        .new_context(context_name)
        .await
        .expect("new_context should return parsed context ID on success");

    assert_eq!(context_id, "7");
}

#[tokio::test]
async fn new_context_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .new_context("My Context")
        .await
        .expect_err("new_context should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn new_context_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/newContext"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"id\":\"7\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .new_context("My Context")
        .await
        .expect_err("new_context should fail when contextId key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn remove_context_posts_to_zap_remove_context_endpoint() {
    let server = MockServer::start().await;
    let context_name = "My Context";

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("contextName=My+Context"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    client
        .remove_context(context_name)
        .await
        .expect("remove_context should succeed when Result is OK");
}

#[tokio::test]
async fn remove_context_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .remove_context("My Context")
        .await
        .expect_err("remove_context should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn remove_context_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .remove_context("My Context")
        .await
        .expect_err("remove_context should fail when Result key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn remove_context_returns_unexpected_content_when_result_is_not_ok() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"FAIL\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .remove_context("My Context")
        .await
        .expect_err("remove_context should fail when Result is not OK");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "Result");
            assert_eq!(content, "FAIL");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn remove_context_rejects_lowercase_ok_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/removeContext"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"ok\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .remove_context("My Context")
        .await
        .expect_err("remove_context should fail when Result is not exact 'OK'");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "Result");
            assert_eq!(content, "ok");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn include_in_context_posts_to_zap_include_in_context_endpoint() {
    let server = MockServer::start().await;
    let context_name = "My Context";
    let regex = "https://example.com/.*";

    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("contextName=My+Context"))
        .and(body_string_contains("regex="))
        .and(body_string_contains("example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    client
        .include_in_context(context_name, regex)
        .await
        .expect("include_in_context should succeed when Result is OK");
}

#[tokio::test]
async fn include_in_context_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .include_in_context("My Context", "https://example.com/.*")
        .await
        .expect_err("include_in_context should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn include_in_context_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"OK\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .include_in_context("My Context", "https://example.com/.*")
        .await
        .expect_err("include_in_context should fail when Result key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn include_in_context_returns_unexpected_content_when_result_is_not_ok() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"FAIL\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .include_in_context("My Context", "https://example.com/.*")
        .await
        .expect_err("include_in_context should fail when Result is not OK");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "Result");
            assert_eq!(content, "FAIL");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}

#[tokio::test]
async fn include_in_context_rejects_lowercase_ok_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/action/includeInContext"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"Result\":\"ok\"}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .include_in_context("My Context", "https://example.com/.*")
        .await
        .expect_err("include_in_context should fail when Result is not exact 'OK'");

    match error {
        ZapClientError::UnexpectedContent { field, content } => {
            assert_eq!(field, "Result");
            assert_eq!(content, "ok");
        }
        other => panic!("expected UnexpectedContent error, got {other:?}"),
    }
}
