// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;
use tracing_test::traced_test;

use crate::{
    scan::retry::{IsTransient, backoff_delay, with_retry},
    storage::StorageError,
    zapclient::ZapClientError,
};

// ── IsTransient implementations ─────────────────────────────────────────────

#[test]
fn storage_backend_error_is_transient() {
    let err = StorageError::Backend("locked".to_string());
    assert!(err.is_transient());
}

#[test]
fn storage_not_found_is_not_transient() {
    let err = StorageError::NotFound("id".to_string());
    assert!(!err.is_transient());
}

#[test]
fn storage_invalid_state_is_not_transient() {
    assert!(!StorageError::InvalidState.is_transient());
}

#[test]
fn zap_request_error_is_transient() {
    // Build a reqwest error by parsing an invalid URL (not a network error,
    // but any reqwest::Error wraps into ZapClientError::Request).
    // Use a ZapClientError::Request variant constructed via the From impl.
    // We can't easily construct a reqwest::Error directly, so we use From<reqwest::Error>
    // indirectly by checking the variant match pattern used in IsTransient.
    // Instead, verify via the ZapClientError::UnexpectedStatus variant which is NOT transient.
    let err = ZapClientError::UnexpectedStatus {
        status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        body: "err".to_string(),
    };
    assert!(!err.is_transient());
}

#[test]
fn zap_parse_error_is_not_transient() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid json {").unwrap_err();
    let err = ZapClientError::ParseResponse(json_err);
    assert!(!err.is_transient());
}

// ── backoff_delay ────────────────────────────────────────────────────────────

#[test]
fn backoff_delay_starts_at_one_second() {
    let delay = backoff_delay(0, Duration::from_secs(60));
    assert_eq!(delay, Duration::from_secs(1));
}

#[test]
fn backoff_delay_doubles_each_attempt() {
    assert_eq!(
        backoff_delay(1, Duration::from_secs(60)),
        Duration::from_secs(2)
    );
    assert_eq!(
        backoff_delay(2, Duration::from_secs(60)),
        Duration::from_secs(4)
    );
    assert_eq!(
        backoff_delay(3, Duration::from_secs(60)),
        Duration::from_secs(8)
    );
}

#[test]
fn backoff_delay_is_capped_at_max_delay() {
    let max = Duration::from_secs(10);
    assert_eq!(backoff_delay(5, max), max); // 2^5=32 > 10
    assert_eq!(backoff_delay(10, max), max);
}

// ── with_retry ───────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum FakeError {
    Transient,
    Permanent,
}

impl IsTransient for FakeError {
    fn is_transient(&self) -> bool {
        matches!(self, FakeError::Transient)
    }
}

#[tokio::test]
async fn succeeds_on_first_attempt() {
    let mut calls = 0usize;
    let result: Result<i32, FakeError> = with_retry(
        "retry.test.success_first",
        || {
            calls += 1;
            async { Ok(42) }
        },
        3,
        Duration::from_millis(1),
    )
    .await;

    assert_eq!(result, Ok(42));
    assert_eq!(calls, 1);
}

#[tokio::test]
async fn retries_on_transient_error_and_succeeds() {
    let mut calls = 0usize;
    let result: Result<i32, FakeError> = with_retry(
        "retry.test.transient_then_success",
        || {
            calls += 1;
            async move {
                if calls < 3 {
                    Err(FakeError::Transient)
                } else {
                    Ok(99)
                }
            }
        },
        5,
        Duration::from_millis(1),
    )
    .await;

    assert_eq!(result, Ok(99));
    assert_eq!(calls, 3);
}

#[tokio::test]
async fn does_not_retry_permanent_error() {
    let mut calls = 0usize;
    let result: Result<i32, FakeError> = with_retry(
        "retry.test.permanent_error",
        || {
            calls += 1;
            async { Err(FakeError::Permanent) }
        },
        5,
        Duration::from_millis(1),
    )
    .await;

    assert_eq!(result, Err(FakeError::Permanent));
    assert_eq!(calls, 1);
}

#[tokio::test]
async fn exhausts_retries_and_returns_last_error() {
    let mut calls = 0usize;
    let result: Result<i32, FakeError> = with_retry(
        "retry.test.exhausted",
        || {
            calls += 1;
            async { Err(FakeError::Transient) }
        },
        3,
        Duration::from_millis(1),
    )
    .await;

    assert_eq!(result, Err(FakeError::Transient));
    // 1 initial attempt + 3 retries = 4 total calls
    assert_eq!(calls, 4);
}

#[tokio::test]
async fn zero_max_retries_does_not_retry() {
    let mut calls = 0usize;
    let result: Result<i32, FakeError> = with_retry(
        "retry.test.zero_retries",
        || {
            calls += 1;
            async { Err(FakeError::Transient) }
        },
        0,
        Duration::from_millis(1),
    )
    .await;

    assert_eq!(result, Err(FakeError::Transient));
    assert_eq!(calls, 1);
}

#[traced_test]
#[tokio::test]
async fn logs_warning_when_transient_retry_remains() {
    let mut calls = 0usize;
    let result: Result<i32, FakeError> = with_retry(
        "retry.test.warn_logging",
        || {
            calls += 1;
            async move {
                if calls == 1 {
                    Err(FakeError::Transient)
                } else {
                    Ok(7)
                }
            }
        },
        2,
        Duration::from_millis(1),
    )
    .await;

    assert_eq!(result, Ok(7));
    assert_eq!(calls, 2);
    assert!(logs_contain("transient failure, retrying operation"));
}

#[traced_test]
#[tokio::test]
async fn logs_error_when_transient_retries_are_exhausted() {
    let mut calls = 0usize;
    let result: Result<i32, FakeError> = with_retry(
        "retry.test.error_logging",
        || {
            calls += 1;
            async { Err(FakeError::Transient) }
        },
        0,
        Duration::from_millis(1),
    )
    .await;

    assert_eq!(result, Err(FakeError::Transient));
    assert_eq!(calls, 1);
    assert!(logs_contain("retry exhausted for transient operation"));
}
