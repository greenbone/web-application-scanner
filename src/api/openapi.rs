// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! OpenAPI 3 documentation specification and handlers.
//!
//! This module is only compiled when the `api-docs` feature is enabled.

use axum::{
    http::{StatusCode, header},
    response::IntoResponse,
};
use utoipa::OpenApi;

use crate::{api, scan::status::ScanStatus};

/// OpenAPI 3.0 specification for the Greenbone Web Application Scanner API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Web Application Scanner",
        description = "A wrapper for the Zed Attack Proxy (ZAP) web application scanner",
        contact(name = "Greenbone AG", url = "https://www.greenbone.net/"),
        license(name = "AGPL-3.0-or-later", url = "https://spdx.org/licenses/AGPL-3.0-or-later.html"),
        version = "0.1",
    ),
    paths(
        // Health endpoints
        api::health::head_health,
        api::health::get_health_alive,
        api::health::get_health_ready,
        api::health::get_health_started,
        // Scan endpoints
        api::scans::head_scans,
        api::scans::create_scan,
        api::scans::get_scan_preferences,
        api::scans::get_scan,
        api::scans::scan_action,
        api::scans::delete_scan,
        api::scans::get_scan_results,
        api::scans::get_scan_result,
        api::scans::get_scan_status,
    ),
    components(schemas(
        // DTO types
        api::dto::scans::ScanRequest,
        api::dto::scans::ScanActionRequest,
        api::dto::scans::ScanAction,
        api::dto::scans::ScanDetailResponse,
        api::dto::scans::ScanStatusResponse,
        api::dto::scans::ScanResultResponse,
        api::dto::scans::PreferencesResponse,
        api::dto::scans::Target,
        api::dto::scans::Credential,
        api::dto::scans::UsernamePasswordCredential,
        api::dto::scans::ScannerPreference,
        api::dto::scans::Vt,
        api::dto::scans::Parameter,
        api::dto::scans::ResultType,
        api::dto::scans::HostInfo,
        // Status
        ScanStatus,
    )),
)]
pub struct ApiDoc;

/// Serve the OpenAPI specification as YAML.
///
/// Returns the complete OpenAPI 3.0 specification in YAML format.
pub async fn get_openapi_yaml() -> impl IntoResponse {
    let openapi = ApiDoc::openapi();
    let yaml = serde_yaml::to_string(&openapi)
        .unwrap_or_else(|_| "error: failed to serialize OpenAPI spec".to_string());

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/yaml; charset=utf-8")],
        yaml,
    )
}
