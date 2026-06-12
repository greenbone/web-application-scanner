// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target URL validation for scan creation.

use crate::scan::ScanServiceError;

/// Validates a list of target URLs, normalizing each by trimming surrounding whitespace.
///
/// Returns the trimmed and validated URLs on success, or the first
/// [`ScanServiceError::InvalidUrl`] encountered.
pub fn validate_target_urls(hosts: &[String]) -> Result<Vec<String>, ScanServiceError> {
    hosts.iter().map(|raw| validate_target_url(raw)).collect()
}

/// Validates a single target URL.
///
/// The URL is trimmed of surrounding whitespace before validation.
/// Rejected when any of the following conditions are met:
/// - the trimmed URL contains embedded whitespace or control characters
/// - the URL cannot be parsed as an absolute URL
/// - the scheme is not `http` or `https`
/// - the URL contains a user-information component (`user:pass@host`)
/// - the URL contains a query string (`?...`)
/// - the URL contains a fragment (`#...`)
/// - the URL path contains dot segments (`.` or `..`)
///
/// Returns the trimmed URL string on success.
pub fn validate_target_url(raw: &str) -> Result<String, ScanServiceError> {
    let url = raw.trim();

    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ScanServiceError::InvalidUrl {
            value: raw.to_string(),
            reason: "URL contains whitespace or control characters".to_string(),
        });
    }

    let parsed = reqwest::Url::parse(url).map_err(|_| ScanServiceError::InvalidUrl {
        value: raw.to_string(),
        reason: "URL could not be parsed as an absolute URL".to_string(),
    })?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ScanServiceError::InvalidUrl {
            value: raw.to_string(),
            reason: "only HTTP and HTTPS URLs are accepted".to_string(),
        });
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ScanServiceError::InvalidUrl {
            value: raw.to_string(),
            reason: "URL must not contain user information".to_string(),
        });
    }

    if parsed.query().is_some() {
        return Err(ScanServiceError::InvalidUrl {
            value: raw.to_string(),
            reason: "URL must not contain a query string".to_string(),
        });
    }

    if parsed.fragment().is_some() {
        return Err(ScanServiceError::InvalidUrl {
            value: raw.to_string(),
            reason: "URL must not contain a fragment".to_string(),
        });
    }

    if has_dot_path_segments(url) {
        return Err(ScanServiceError::InvalidUrl {
            value: raw.to_string(),
            reason: "URL must not contain dot path segments".to_string(),
        });
    }

    Ok(url.to_string())
}

/// Returns `true` if the URL's path contains a segment that is exactly `.` or `..`.
///
/// Checks the raw URL string before any normalization performed by the URL parser.
fn has_dot_path_segments(url: &str) -> bool {
    // Find where the path starts: skip "scheme://authority"
    let path_start = url
        .find("://")
        .and_then(|i| url[i + 3..].find('/').map(|j| i + 3 + j))
        .unwrap_or(url.len());

    let path_and_rest = &url[path_start..];

    // Isolate the path before any query or fragment
    let path_end = path_and_rest
        .find('?')
        .or_else(|| path_and_rest.find('#'))
        .unwrap_or(path_and_rest.len());
    let path = &path_and_rest[..path_end];

    path.split('/').any(|seg| seg == "." || seg == "..")
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod validation_tests;
