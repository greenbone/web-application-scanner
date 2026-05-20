// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ScanRequest {
    pub target: Target,
    #[serde(default)]
    pub scan_preferences: Vec<ScannerPreference>,
    pub vts: Vec<Vt>,
}

#[derive(Debug, Deserialize)]
pub struct ScanActionRequest {
    pub action: ScanAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanAction {
    Start,
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub hosts: Vec<String>,
    #[serde(default)]
    pub excluded_hosts: Vec<String>,
    #[serde(default)]
    pub credentials: Vec<Credential>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub service: String,
    pub port: Option<i32>,
    pub up: Option<UsernamePasswordCredential>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsernamePasswordCredential {
    pub username: String,
    pub password: Option<String>,
    pub privilege_username: Option<String>,
    pub privilege_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerPreference {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub id: i32,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vt {
    pub oid: String,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

/// Lifecycle phase of a scan, matching the OpenAPI Status.status enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Stored,
    Requested,
    Running,
    Stopped,
    Failed,
    Succeeded,
}

/// Type of a scan result, matching the OpenAPI Result.type enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    Alarm,
    Log,
    Error,
    HostStart,
    HostStop,
    HostDetail,
}

#[derive(Debug, Serialize)]
pub struct ScanIdResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Default)]
pub struct PreferencesResponse {}

/// Response for GET /scans/:id – returns the full scan definition.
#[derive(Debug, Serialize)]
pub struct ScanDetailResponse {
    pub scan_id: String,
    pub target: Target,
    pub scan_preferences: Vec<ScannerPreference>,
    pub vts: Vec<Vt>,
}

/// Response for GET /scans/:id/status – returns lifecycle status and timestamps.
#[derive(Debug, Serialize)]
pub struct ScanStatusResponse {
    pub status: ScanStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
}

/// Full result record returned by GET /scans/:id/results and /results/:rid.
#[derive(Debug, Serialize)]
pub struct ScanResultResponse {
    pub id: i64,
    #[serde(rename = "type")]
    pub result_type: ResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}