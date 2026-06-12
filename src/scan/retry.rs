// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retry helper with exponential backoff for transient failures.

use std::{future::Future, time::Duration};

use tokio::time::sleep;
use tracing::{error, warn};

use crate::{storage::StorageError, zapclient::ZapClientError};

/// Marker trait for errors that can be retried.
///
/// Transient errors are those caused by temporary external conditions such as
/// network unavailability or storage lock contention. Non-transient errors
/// indicate permanent failures that should not be retried.
pub trait IsTransient {
    /// Returns `true` if the error is transient and the operation may be retried.
    fn is_transient(&self) -> bool;
}

impl IsTransient for StorageError {
    fn is_transient(&self) -> bool {
        matches!(self, StorageError::Backend(_))
    }
}

impl IsTransient for ZapClientError {
    fn is_transient(&self) -> bool {
        matches!(self, ZapClientError::Request(_))
    }
}

/// Computes the exponential backoff delay for a given attempt number, capped at `max_delay`.
///
/// The delay starts at 1 second and doubles with each attempt: `min(2^attempt, max_delay)`.
pub fn backoff_delay(attempt: u32, max_delay: Duration) -> Duration {
    // 2^attempt seconds, capped at max_delay; guard against overflow with checked_shl
    let secs = 1u64
        .checked_shl(attempt)
        .unwrap_or(u64::MAX)
        .min(max_delay.as_secs().max(1));
    Duration::from_secs(secs).min(max_delay)
}

/// Retries an async operation with exponential backoff when transient errors occur.
///
/// The operation is called repeatedly until it succeeds, returns a non-transient error,
/// or `max_retries` retry attempts are exhausted (meaning the operation is called at most
/// `max_retries + 1` times in total).
///
/// # Parameters
/// - `operation`: closure that produces a future returning `Result<T, E>`.
/// - `max_retries`: maximum number of additional attempts after the first failure.
/// - `max_delay`: upper bound for the backoff sleep duration.
pub async fn with_retry<F, Fut, T, E>(
    operation_name: &'static str,
    mut operation: F,
    max_retries: u32,
    max_delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: IsTransient + std::fmt::Debug,
{
    let mut attempt = 0u32;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if !error.is_transient() {
                    return Err(error);
                }

                if attempt >= max_retries {
                    error!(
                        operation = operation_name,
                        attempt,
                        max_retries,
                        error = ?error,
                        "retry exhausted for transient operation"
                    );
                    return Err(error);
                }

                let delay = backoff_delay(attempt, max_delay);
                warn!(
                    operation = operation_name,
                    attempt,
                    max_retries,
                    backoff_seconds = delay.as_secs_f64(),
                    error = ?error,
                    "transient failure, retrying operation"
                );
                sleep(delay).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod retry_tests;
