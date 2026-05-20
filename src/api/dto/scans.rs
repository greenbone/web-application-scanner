// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Target {
    pub hosts: Vec<String>,
    #[serde(default)]
    pub excluded_hosts: Vec<String>,
    #[serde(default)]
    pub credentials: Vec<Credential>,
}

#[derive(Debug, Deserialize)]
pub struct Credential {
    pub service: String,
    pub port: Option<i32>,
    pub up: Option<UsernamePasswordCredential>,
}

#[derive(Debug, Deserialize)]
pub struct UsernamePasswordCredential {
    pub username: String,
    pub password: Option<String>,
    pub privilege_username: Option<String>,
    pub privilege_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScannerPreference {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Parameter {
    pub id: i32,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Vt {
    pub oid: String,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Serialize)]
pub struct ScanIdResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Default)]
pub struct PreferencesResponse {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Stored,
}

#[derive(Debug, Serialize)]
pub struct ScanResponse {
    pub id: String,
    pub status: ScanStatus,
}

#[derive(Debug, Serialize)]
pub struct ScanResultResponse {
    pub id: String,
}