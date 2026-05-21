// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::ZapClient;
use super::ZapClientError;

#[test]
fn new_returns_error_for_empty_base_url() {
    let result = ZapClient::new("".to_string(), "some_api_key".to_string());
    assert!(matches!(result, Err(ZapClientError::MissingSetting(_))));
}

#[test]
fn new_returns_error_for_empty_api_key() {
    let result = ZapClient::new("http://example.com".to_string(), "".to_string());
    assert!(matches!(result, Err(ZapClientError::MissingSetting(_))));
}

#[test]
fn new_returns_error_for_invalid_base_url() {
    let result = ZapClient::new("invalid_url".to_string(), "some_api_key".to_string());
    assert!(matches!(result, Err(ZapClientError::InvalidBaseUrl(_))));
}

#[test]
fn new_creates_client_with_valid_inputs() {
    let result = ZapClient::new("http://example.com".to_string(), "some_api_key".to_string());
    assert!(result.is_ok());
    let client = result.unwrap();
    assert_eq!(client.base_url.as_str(), "http://example.com/");
    assert_eq!(client.api_key, "some_api_key");
}