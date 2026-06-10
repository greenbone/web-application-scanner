// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan domain model types.

use crate::{
    api::dto::scans::{ResultType, ScannerPreference, Target, Vt},
    storage::{ResultRecord, ScanRecord},
};

use super::ScanStatus;

/// Scan domain entity used by service contracts.
#[derive(Debug, Clone)]
pub struct Scan {
    pub id: String,
    pub target: Target,
    pub scan_preferences: Vec<ScannerPreference>,
    pub vts: Vec<Vt>,
    pub status: ScanStatus,
    pub stop_requested: bool,
    pub queued_time: Option<i64>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub context_name: Option<String>,
    pub context_id: Option<String>,
    pub alert_cursor: Option<i64>,
    pub progress: Option<serde_json::Value>,
    pub interruption_reason: Option<String>,
}

/// Scan status read model for service consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanStatusView {
    pub status: ScanStatus,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

/// Scan result domain entity used by service contracts.
#[derive(Debug, Clone)]
pub struct ScanResult {
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

impl Scan {
    pub fn status_view(&self) -> ScanStatusView {
        ScanStatusView {
            status: self.status,
            start_time: self.start_time,
            end_time: self.end_time,
        }
    }
}

impl From<ScanRecord> for Scan {
    fn from(record: ScanRecord) -> Self {
        Self {
            id: record.id,
            target: record.target,
            scan_preferences: record.scan_preferences,
            vts: record.vts,
            status: record.status,
            stop_requested: record.stop_requested,
            queued_time: record.queued_time,
            start_time: record.start_time,
            end_time: record.end_time,
            context_name: record.context_name,
            context_id: record.context_id,
            alert_cursor: record.alert_cursor,
            progress: record.progress,
            interruption_reason: record.interruption_reason,
        }
    }
}

impl From<Scan> for ScanRecord {
    fn from(scan: Scan) -> Self {
        Self {
            id: scan.id,
            target: scan.target,
            scan_preferences: scan.scan_preferences,
            vts: scan.vts,
            status: scan.status,
            stop_requested: scan.stop_requested,
            queued_time: scan.queued_time,
            start_time: scan.start_time,
            end_time: scan.end_time,
            context_name: scan.context_name,
            context_id: scan.context_id,
            alert_cursor: scan.alert_cursor,
            progress: scan.progress,
            interruption_reason: scan.interruption_reason,
        }
    }
}

impl From<ResultRecord> for ScanResult {
    fn from(record: ResultRecord) -> Self {
        Self {
            id: record.id,
            scan_id: record.scan_id,
            result_type: record.result_type,
            ip_address: record.ip_address,
            hostname: record.hostname,
            oid: record.oid,
            port: record.port,
            protocol: record.protocol,
            message: record.message,
            detail: record.detail,
        }
    }
}

impl From<ScanResult> for ResultRecord {
    fn from(result: ScanResult) -> Self {
        Self {
            id: result.id,
            scan_id: result.scan_id,
            result_type: result.result_type,
            ip_address: result.ip_address,
            hostname: result.hostname,
            oid: result.oid,
            port: result.port,
            protocol: result.protocol,
            message: result.message,
            detail: result.detail,
        }
    }
}
