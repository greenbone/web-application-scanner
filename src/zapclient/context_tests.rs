// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use reqwest::StatusCode;
use wiremock::{
    Mock,
    MockServer,
    ResponseTemplate,
    matchers::{body_string_contains, method, path},
};

use super::{ContextListResponse, ZapClient, ZapClientError};

const API_KEY: &str = "test-api-key";

#[tokio::test]
async fn context_list_posts_to_zap_context_list_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/JSON/context/view/contextList"))
        .and(body_string_contains(&format!("apikey={API_KEY}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{\"contextList\":[\"Default Context\",\"Test\"]}"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = ZapClient::new(server.uri(), API_KEY.to_string())
        .expect("client should be constructed");

    let response = client
        .context_list()
        .await
        .expect("context_list should return parsed response on success");

    assert_eq!(
        response,
        ContextListResponse {
            context_list: vec!["Default Context".to_string(), "Test".to_string()],
        }
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

    let client = ZapClient::new(server.uri(), API_KEY.to_string())
        .expect("client should be constructed");

    let error = client
        .context_list()
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

    let client = ZapClient::new(server.uri(), API_KEY.to_string())
        .expect("client should be constructed");

    let error = client
        .context_list()
        .await
        .expect_err("context_list should fail when contextList key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}