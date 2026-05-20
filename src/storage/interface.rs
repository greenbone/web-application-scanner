// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use thiserror::Error;

use crate::api::dto::scans::{ResultType, ScanStatus, ScannerPreference, Target, Vt};

/// Full persisted scan record.
#[derive(Debug, Clone)]
pub struct ScanRecord {
    pub id: String,
    pub target: Target,
    pub scan_preferences: Vec<ScannerPreference>,
    pub vts: Vec<Vt>,
    pub status: ScanStatus,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

/// A single persisted result for a scan.
#[derive(Debug, Clone)]
pub struct ResultRecord {
    /// 0-based auto-incremented index within the scan.
    pub id: i64,
    pub scan_id: String,
    pub result_type: ResultType,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub oid: Option<String>,
    pub port: Option<i32>,
    pub protocol: Option<String>,
    pub message: Option<String>,
    pub detail: Option<serde_json::Value>,
}

/// Errors returned by storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("scan not found: {0}")]
    NotFound(String),

    #[error("result not found: scan={0} result={1}")]
    ResultNotFound(String, i64),

    #[error("scan already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid scan state for this operation")]
    InvalidState,

    #[error("invalid range: {0}")]
    BadRange(String),

    #[error("storage backend error: {0}")]
    Backend(String),
}

/// Parse a range string into a (start, optional end) index pair (both inclusive).
///
/// Accepts `N` (all results from index N onward) or `N-M` (results N through M).
pub fn parse_range(range: &str) -> Result<(usize, Option<usize>), StorageError> {
    let trimmed = range.trim();
    if let Some((s, e)) = trimmed.split_once('-') {
        let start: usize = s
            .trim()
            .parse()
            .map_err(|_| StorageError::BadRange(range.to_string()))?;
        let end: usize = e
            .trim()
            .parse()
            .map_err(|_| StorageError::BadRange(range.to_string()))?;
        if end < start {
            return Err(StorageError::BadRange(range.to_string()));
        }
        Ok((start, Some(end)))
    } else {
        let start: usize = trimmed
            .parse()
            .map_err(|_| StorageError::BadRange(range.to_string()))?;
        Ok((start, None))
    }
}

/// Abstract interface covering persistence of scans and their results.
///
/// Implementations must be `Send + Sync` so the same instance can be shared
/// across async tasks and Axum handler threads.
#[async_trait]
pub trait ScanStorage: Send + Sync {
    /// Persist a new scan. Returns [`StorageError::AlreadyExists`] if the ID
    /// is already in use.
    async fn create_scan(&self, scan: ScanRecord) -> Result<(), StorageError>;

    /// Retrieve a scan by its ID.
    async fn get_scan(&self, id: &str) -> Result<ScanRecord, StorageError>;

    /// Overwrite the lifecycle status of a scan.
    async fn update_scan_status(&self, id: &str, status: ScanStatus) -> Result<(), StorageError>;

    /// Delete a scan and all of its results.
    async fn delete_scan(&self, id: &str) -> Result<(), StorageError>;

    /// Append a result to a scan. The `id` field in `result` is ignored; the
    /// backend assigns the next 0-based auto-incremented index.
    async fn add_result(&self, scan_id: &str, result: ResultRecord) -> Result<(), StorageError>;

    /// Retrieve a single result by its 0-based index within the scan.
    async fn get_result(
        &self,
        scan_id: &str,
        result_id: i64,
    ) -> Result<ResultRecord, StorageError>;

    /// Retrieve results for a scan within an optional index range.
    ///
    /// `start` is the 0-based first index (inclusive). `end` is the last index
    /// (inclusive); when `None`, all results from `start` onward are returned.
    async fn get_results(
        &self,
        scan_id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ResultRecord>, StorageError>;
}

#[cfg(test)]
#[path = "interface_tests.rs"]
mod tests;
