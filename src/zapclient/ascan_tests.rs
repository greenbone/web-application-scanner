// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use reqwest::StatusCode;
use wiremock::{
	Mock,
	MockServer,
	ResponseTemplate,
	matchers::{method, path, query_param, body_string_contains},
};

use super::{ZapClient, ZapClientError};

const API_KEY: &str = "test-api-key";

#[tokio::test]
async fn start_active_scan_posts_to_zap_ascan_scan_endpoint() {
	let server = MockServer::start().await;

	Mock::given(method("POST"))
		.and(path("/JSON/ascan/action/scan"))
		.and(body_string_contains(&format!("apikey={API_KEY}")))
		.and(body_string_contains("url=https%3A%2F%2Fexample.com"))
		.and(body_string_contains("recurse=true"))
		.and(body_string_contains("inScopeOnly=false"))
		.and(body_string_contains("contextId=3"))
		.respond_with(ResponseTemplate::new(200).set_body_string("{\"scan\":\"7\"}"))
		.expect(1)
		.mount(&server)
		.await;

	let client = ZapClient::new(server.uri(), API_KEY.to_string())
		.expect("client should be constructed");

	let scan_id = client
		.start_active_scan("3", "https://example.com", true, false)
		.await
		.expect("start_active_scan should return parsed scan id on success");

	assert_eq!(scan_id, "7");
}

#[tokio::test]
async fn start_active_scan_returns_unexpected_status_on_http_error() {
	let server = MockServer::start().await;

	Mock::given(method("POST"))
		.and(path("/JSON/ascan/action/scan"))
		.respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
		.expect(1)
		.mount(&server)
		.await;

	let client = ZapClient::new(server.uri(), API_KEY.to_string())
		.expect("client should be constructed");

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

	Mock::given(method("POST"))
		.and(path("/JSON/ascan/action/scan"))
		.respond_with(ResponseTemplate::new(200).set_body_string("{\"scanId\":\"7\"}"))
		.expect(1)
		.mount(&server)
		.await;

	let client = ZapClient::new(server.uri(), API_KEY.to_string())
		.expect("client should be constructed");

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

	Mock::given(method("GET"))
		.and(path("/JSON/ascan/view/status"))
		.and(query_param("apikey", API_KEY))
		.and(query_param("scanId", "7"))
		.respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":42}"))
		.expect(1)
		.mount(&server)
		.await;

	let client = ZapClient::new(server.uri(), API_KEY.to_string())
		.expect("client should be constructed");

	let status = client
		.get_active_scan_status("7")
		.await
		.expect("get_active_scan_status should return parsed status on success");

	assert_eq!(status, 42);
}

#[tokio::test]
async fn get_active_scan_status_returns_unexpected_status_on_http_error() {
	let server = MockServer::start().await;

	Mock::given(method("GET"))
		.and(path("/JSON/ascan/view/status"))
		.respond_with(ResponseTemplate::new(500).set_body_string("zap unavailable"))
		.expect(1)
		.mount(&server)
		.await;

	let client = ZapClient::new(server.uri(), API_KEY.to_string())
		.expect("client should be constructed");

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

	Mock::given(method("GET"))
		.and(path("/JSON/ascan/view/status"))
		.respond_with(ResponseTemplate::new(200).set_body_string("{\"progress\":42}"))
		.expect(1)
		.mount(&server)
		.await;

	let client = ZapClient::new(server.uri(), API_KEY.to_string())
		.expect("client should be constructed");

	let error = client
		.get_active_scan_status("7")
		.await
		.expect_err("get_active_scan_status should fail when status key is missing");

	match error {
		ZapClientError::ParseResponse(_) => {}
		other => panic!("expected ParseResponse error, got {other:?}"),
	}
}

