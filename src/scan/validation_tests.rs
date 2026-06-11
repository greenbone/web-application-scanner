// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::scan::{ScanServiceError, validation::validate_target_url};

#[test]
fn accepts_http_url() {
    let result = validate_target_url("http://example.com/");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    assert_eq!(result.unwrap(), "http://example.com/");
}

#[test]
fn accepts_https_url() {
    let result = validate_target_url("https://example.com/path");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    assert_eq!(result.unwrap(), "https://example.com/path");
}

#[test]
fn accepts_url_with_port() {
    let result = validate_target_url("http://example.com:8080/");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    assert_eq!(result.unwrap(), "http://example.com:8080/");
}

#[test]
fn rejects_query_string() {
    let result = validate_target_url("https://example.com/path?query=1");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("query string")),
        "expected InvalidUrl with query-string reason, got {:?}",
        result
    );
}

#[test]
fn trims_surrounding_whitespace() {
    let result = validate_target_url("  http://example.com/  ");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    assert_eq!(result.unwrap(), "http://example.com/");
}

#[test]
fn rejects_embedded_whitespace() {
    let result = validate_target_url("http://example.com/path here");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("whitespace")),
        "expected InvalidUrl with whitespace reason, got {:?}",
        result
    );
}

#[test]
fn rejects_control_character() {
    let url = "http://example.com/\x00path";
    let result = validate_target_url(url);
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { .. })),
        "expected InvalidUrl, got {:?}",
        result
    );
}

#[test]
fn rejects_ftp_scheme() {
    let result = validate_target_url("ftp://example.com/file");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("HTTP")),
        "expected InvalidUrl with scheme reason, got {:?}",
        result
    );
}

#[test]
fn rejects_relative_url() {
    let result = validate_target_url("/relative/path");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { .. })),
        "expected InvalidUrl, got {:?}",
        result
    );
}

#[test]
fn rejects_non_url_string() {
    let result = validate_target_url("not a url at all");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { .. })),
        "expected InvalidUrl, got {:?}",
        result
    );
}

#[test]
fn rejects_user_info() {
    let result = validate_target_url("http://user:pass@example.com/");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("user information")),
        "expected InvalidUrl with user-info reason, got {:?}",
        result
    );
}

#[test]
fn rejects_username_only() {
    let result = validate_target_url("http://user@example.com/");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("user information")),
        "expected InvalidUrl with user-info reason, got {:?}",
        result
    );
}

#[test]
fn rejects_fragment() {
    let result = validate_target_url("http://example.com/#section");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("fragment")),
        "expected InvalidUrl with fragment reason, got {:?}",
        result
    );
}

#[test]
fn rejects_dot_segment() {
    let result = validate_target_url("http://example.com/./path");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("dot")),
        "expected InvalidUrl with dot-segment reason, got {:?}",
        result
    );
}

#[test]
fn rejects_dotdot_segment() {
    let result = validate_target_url("http://example.com/../path");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref reason, .. }) if reason.contains("dot")),
        "expected InvalidUrl with dot-segment reason, got {:?}",
        result
    );
}

#[test]
fn rejects_trailing_dotdot() {
    let result = validate_target_url("http://example.com/path/..");
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { .. })),
        "expected InvalidUrl, got {:?}",
        result
    );
}

#[test]
fn accepts_hidden_file_path() {
    // A path segment starting with dot but not exactly "." or ".." is allowed
    let result = validate_target_url("http://example.com/.hidden");
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    assert_eq!(result.unwrap(), "http://example.com/.hidden");
}

#[test]
fn preserves_original_value_in_error() {
    let raw = "  ftp://example.com/  ";
    let result = validate_target_url(raw);
    assert!(
        matches!(result, Err(ScanServiceError::InvalidUrl { ref value, .. }) if value == raw),
        "expected error to preserve original value, got {:?}",
        result
    );
}
