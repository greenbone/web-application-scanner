// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use reqwest::StatusCode;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, method, path},
};

use super::{AlertRiskLevel, ZapClient, ZapClientError};

const API_KEY: &str = "test-api-key";

#[tokio::test]
async fn get_alerts_gets_zap_alert_view_alerts_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("contextName=Default+Context"))
        .and(body_string_contains("baseurl=https%3A%2F%2Fexample.com"))
        .and(body_string_contains("start=10"))
        .and(body_string_contains("count=25"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "{\"alerts\":[{\"alertRef\":\"40012\",\"name\":\"Cross Site Scripting\",\"risk\":\"High\",\"description\":\"Reflected XSS detected\",\"url\":\"https://example.com/vuln\"}]}",
            ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let alerts = client
        .get_alerts(
            "Default Context",
            Some("https://example.com"),
            Some(10),
            Some(25),
        )
        .await
        .expect("get_alerts should return parsed alerts on success");

    assert_eq!(alerts.len(), 1);

    let first = &alerts[0];
    assert_eq!(first.alert_ref, "40012");
    assert_eq!(first.name, "Cross Site Scripting");
    assert_eq!(first.risk, AlertRiskLevel::High);
    assert_eq!(first.description, "Reflected XSS detected");
    assert_eq!(first.url, "https://example.com/vuln");
}

#[tokio::test]
async fn get_alerts_returns_unknown_risk_level_for_unrecognized_value() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "{\"alerts\":[{\"alertRef\":\"50001\",\"name\":\"Custom Risk\",\"risk\":\"Critical\",\"description\":\"Unknown risk value from API\",\"url\":\"https://example.com/custom\"}]}",
            ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let alerts = client
        .get_alerts("Default Context", None, None, None)
        .await
        .expect("get_alerts should deserialize unknown risk values as Unknown");

    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].risk, AlertRiskLevel::Unknown);
}

#[tokio::test]
async fn get_alerts_returns_unexpected_status_on_http_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_alerts("Default Context", None, None, None)
        .await
        .expect_err("get_alerts should fail on non-success status");

    match error {
        ZapClientError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "zap unavailable");
        }
        other => panic!("expected UnexpectedStatus error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_alerts_returns_parse_error_for_invalid_schema() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"alertItems\":[]}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let error = client
        .get_alerts("Default Context", None, None, None)
        .await
        .expect_err("get_alerts should fail when alerts key is missing");

    match error {
        ZapClientError::ParseResponse(_) => {}
        other => panic!("expected ParseResponse error, got {other:?}"),
    }
}

#[tokio::test]
async fn get_alerts_omits_optional_form_params_when_not_provided() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/JSON/alert/view/alerts"))
        .and(body_string_contains(format!("apikey={API_KEY}")))
        .and(body_string_contains("contextName=Default+Context"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"alerts\":[]}"))
        .expect(1)
        .mount(&server)
        .await;

    let client =
        ZapClient::new(server.uri(), API_KEY.to_string()).expect("client should be constructed");

    let alerts = client
        .get_alerts("Default Context", None, None, None)
        .await
        .expect("get_alerts should succeed without base_url");

    assert!(alerts.is_empty());
}
