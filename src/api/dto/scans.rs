// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan API request and response data transfer objects.

use serde::{Deserialize, Serialize};

use crate::scan::ScanStatus;

/// Request body for POST /scans — Create a new scan.
#[derive(Debug, Clone, Deserialize)]
pub struct ScanRequest {
    /// Target hosts to scan.
    pub target: Target,
    /// Optional scanner preferences.
    #[serde(default)]
    pub scan_preferences: Vec<ScannerPreference>,
    /// Vulnerability tests (VTs) to run.
    pub vts: Vec<Vt>,
}

/// Request body for POST /scans/{id} — Perform an action on a scan.
#[derive(Debug, Deserialize)]
pub struct ScanActionRequest {
    /// The action to perform (Start or Stop).
    pub action: ScanAction,
}

/// Scan action type.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanAction {
    /// Start the scan.
    Start,
    /// Stop the scan.
    Stop,
}

/// Scan target specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Target hosts to scan.
    pub hosts: Vec<String>,
    /// Hosts to exclude from the scan.
    #[serde(default)]
    pub excluded_hosts: Vec<String>,
    /// Credentials for authentication during scan.
    #[serde(default)]
    pub credentials: Vec<Credential>,
}

/// Authentication credential for a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    /// Service type (must be "http" for web app scans).
    pub service: String,
    /// Optional port number.
    pub port: Option<i32>,
    /// Username/password credential.
    pub up: Option<UsernamePasswordCredential>,
}

/// Username and password credential pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsernamePasswordCredential {
    /// Username for authentication.
    pub username: String,
    /// Password (optional).
    pub password: Option<String>,
    /// Privilege escalation username (for sudo/elevation).
    pub privilege_username: Option<String>,
    /// Privilege escalation password.
    pub privilege_password: Option<String>,
}

/// Scanner preference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerPreference {
    /// Preference identifier.
    pub id: String,
    /// Preference value.
    pub value: String,
}

/// VT parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter identifier.
    pub id: i32,
    /// Parameter value.
    pub value: String,
}

/// Vulnerability test (VT) specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vt {
    /// VT OID (Object Identifier).
    pub oid: String,
    /// VT parameters.
    #[serde(default)]
    pub parameters: Vec<Parameter>,
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

/// Response body for POST /scans – returns the created scan ID.
#[derive(Debug, Serialize)]
pub struct ScanIdResponse {
    /// The UUID of the newly created scan.
    pub id: String,
}

/// Response body for GET /scans/preferences – available scanner preferences.
#[derive(Debug, Serialize, Default)]
pub struct PreferencesResponse {}

/// Response body for GET /scans/{id} – full scan details.
#[derive(Debug, Serialize)]
pub struct ScanDetailResponse {
    /// The scan UUID.
    pub scan_id: String,
    /// Target specification.
    pub target: Target,
    /// Scanner preferences applied to this scan.
    pub scan_preferences: Vec<ScannerPreference>,
    /// Vulnerability tests run in this scan.
    pub vts: Vec<Vt>,
}

/// Response body for GET /scans/{id}/status – scan lifecycle and timing.
#[derive(Debug, Serialize)]
pub struct ScanStatusResponse {
    /// Current scan status.
    pub status: ScanStatus,
    /// Unix timestamp when the scan started (None if not yet started).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    /// Unix timestamp when the scan ended (None if still running or not started).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
}

/// Response body for GET /scans/{id}/results/{rid} – a single scan result.
///
/// Individual result record with optional fields omitted from JSON serialization.
#[derive(Debug, Serialize)]
pub struct ScanResultResponse {
    /// 0-based result index within the scan.
    pub id: i64,
    /// Type of result (alarm, log, error, etc.).
    #[serde(rename = "type")]
    pub result_type: ResultType,
    /// IP address associated with this result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// Hostname associated with this result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// OID of the vulnerability test that generated this result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oid: Option<String>,
    /// Port associated with this result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    /// Protocol (e.g., TCP, UDP).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// Result message or description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional detailed result information (e.g., JSON-structured extra data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}
