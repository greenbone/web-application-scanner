// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{
    api,
    api::dto::scans::{
        HostInfo, HostScanningEntry, ScanAction, ScanActionRequest, ScanDetailResponse,
        ScanRequest, ScanResultResponse, ScanStatusResponse,
    },
    app::AppState,
    scan::{CreateScanRequest, ScanProgress, ScanServiceError, progress::StageState},
    storage::interface::{StorageError, parse_range},
};

/// Query parameters for the GET `/scans/{id}/results` endpoint.
#[derive(Debug, Deserialize)]
pub struct ResultRangeQuery {
    /// Optional range specification (e.g., "5" or "0-10").
    pub range: Option<String>,
}

// ─── Error mapping ────────────────────────────────────────────────────────────

/// Convert storage errors to appropriate HTTP responses.
fn storage_err(e: StorageError) -> Response {
    match e {
        StorageError::NotFound(_) | StorageError::ResultNotFound(_, _) => {
            StatusCode::NOT_FOUND.into_response()
        }
        StorageError::AlreadyExists(_) => StatusCode::FORBIDDEN.into_response(),
        StorageError::InvalidState => StatusCode::NOT_ACCEPTABLE.into_response(),
        StorageError::BadRange(_) => StatusCode::BAD_REQUEST.into_response(),
        StorageError::Backend(msg) => {
            tracing::error!("storage backend error: {}", msg);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Convert scan service errors to appropriate HTTP responses.
fn scan_service_err(e: ScanServiceError) -> Response {
    match e {
        ScanServiceError::InvalidTransition { .. } => StatusCode::NOT_ACCEPTABLE.into_response(),
        ScanServiceError::ScanNotFound(_) => StatusCode::NOT_FOUND.into_response(),
        ScanServiceError::InvalidUrl { .. } => StatusCode::BAD_REQUEST.into_response(),
        ScanServiceError::Storage(storage_error) => storage_err(storage_error),
        ScanServiceError::ZapClient(zap_error) => {
            tracing::error!("scan service zap client error: {}", zap_error);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ─── Result conversion ────────────────────────────────────────────────────────

/// Convert a storage result record into an API response.
fn result_response(r: crate::scan::ScanResult) -> ScanResultResponse {
    ScanResultResponse {
        id: r.id,
        result_type: r.result_type,
        ip_address: r.ip_address,
        hostname: r.hostname,
        oid: r.oid,
        port: r.port,
        protocol: r.protocol,
        message: r.message,
        detail: r.detail,
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// HEAD /scans — Return API and authentication metadata as headers.
pub async fn head_scans() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("api-version"),
        HeaderValue::from_static(api::API_VERSION),
    );
    headers.insert(
        HeaderName::from_static("authentication"),
        HeaderValue::from_static("none"),
    );
    (StatusCode::NO_CONTENT, headers)
}

/// POST /scans — Create a new scan and return its UUID.
///
/// Returns 201 Created with the generated or provided scan ID if successful.
pub async fn create_scan(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> impl IntoResponse {
    let request = CreateScanRequest {
        scan_id: req.scan_id,
        target: req.target,
        scan_preferences: req.scan_preferences,
        vts: req.vts,
    };

    match state.scan_service.create_scan(request).await {
        Ok(id) => (StatusCode::CREATED, Json(id)).into_response(),
        Err(e) => scan_service_err(e),
    }
}

/// GET /scans/preferences — Retrieve available scan preferences.
pub async fn get_scan_preferences(State(state): State<AppState>) -> impl IntoResponse {
    match state.scan_service.get_default_preferences().await {
        Ok(preferences) => Json(preferences).into_response(),
        Err(e) => scan_service_err(e),
    }
}

/// GET /scans/{id} — Retrieve scan details.
///
/// Returns the target, scan preferences, and VTs for the requested scan.
/// Returns 404 if the scan does not exist.
pub async fn get_scan(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.scan_service.get_scan(&id).await {
        Ok(scan) => Json(ScanDetailResponse {
            scan_id: scan.id,
            target: scan.target,
            scan_preferences: scan.scan_preferences,
            vts: scan.vts,
        })
        .into_response(),
        Err(e) => scan_service_err(e),
    }
}

/// POST /scans/{id} — Perform an action on a scan (start or stop).
///
/// Enforces non-idempotent start and stop transitions. Returns 406 if invalid.
pub async fn scan_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ScanActionRequest>,
) -> impl IntoResponse {
    let result = match req.action {
        ScanAction::Start => state.scan_service.start_scan(&id).await,
        ScanAction::Stop => state.scan_service.stop_scan(&id).await,
    };

    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => scan_service_err(e),
    }
}

/// DELETE /scans/{id} — Delete a scan and all its results.
///
/// Returns 406 if the scan is not in `new` or a terminal status.
pub async fn delete_scan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.scan_service.delete_scan(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => scan_service_err(e),
    }
}

/// GET /scans/{id}/results — Retrieve scan results with optional range filtering.
///
/// Query parameter `range` accepts `N` (all from N onward) or `N-M` (inclusive range).
/// Defaults to all results if not specified.
pub async fn get_scan_results(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ResultRangeQuery>,
) -> impl IntoResponse {
    let (start, end) = match query.range.as_deref() {
        Some(r) => match parse_range(r) {
            Ok(pair) => pair,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        None => (0, None),
    };

    match state.scan_service.get_results(&id, start, end).await {
        Ok(results) => {
            Json(results.into_iter().map(result_response).collect::<Vec<_>>()).into_response()
        }
        Err(e) => scan_service_err(e),
    }
}

/// GET /scans/{id}/results/{rid} — Retrieve a single scan result by index.
///
/// The `{rid}` parameter is a 0-based result index. Returns 404 if not found.
pub async fn get_scan_result(
    State(state): State<AppState>,
    Path((id, rid)): Path<(String, String)>,
) -> impl IntoResponse {
    let result_id: i64 = match rid.parse() {
        Ok(n) => n,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.scan_service.get_scan_result(&id, result_id).await {
        Ok(r) => Json(result_response(r)).into_response(),
        Err(e) => scan_service_err(e),
    }
}

/// GET /scans/{id}/status — Retrieve the current status and timestamps of a scan.
///
/// Returns the status, start time, and end time of the scan.
pub async fn get_scan_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.scan_service.get_scan_status(&id).await {
        Ok(status_view) => {
            let host_info = status_view.progress.as_ref().map(progress_to_host_info);
            Json(ScanStatusResponse {
                status: status_view.status,
                start_time: status_view.start_time,
                end_time: status_view.end_time,
                host_info,
            })
            .into_response()
        }
        Err(e) => scan_service_err(e),
    }
}

/// Convert persisted [`ScanProgress`] into the [`HostInfo`] API representation.
fn progress_to_host_info(progress: &ScanProgress) -> HostInfo {
    let all = progress.targets.len() as i32;
    let queued = progress
        .targets
        .iter()
        .filter(|t| t.spider_state == StageState::Pending)
        .count() as i32;
    let finished = progress
        .targets
        .iter()
        .filter(|t| t.active_scan_state == StageState::Done)
        .count() as i32;
    let scanning: Vec<HostScanningEntry> = progress
        .targets
        .iter()
        .filter(|t| {
            t.spider_state != StageState::Pending && t.active_scan_state != StageState::Done
        })
        .map(|t| HostScanningEntry {
            host: t.target.clone(),
            progress: t.overall_percentage,
        })
        .collect();
    HostInfo {
        all,
        excluded: 0,
        dead: 0,
        alive: finished,
        queued,
        finished,
        scanning,
    }
}

#[cfg(test)]
#[path = "scans_tests.rs"]
mod scans_tests;
