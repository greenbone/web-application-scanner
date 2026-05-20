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
use uuid::Uuid;

use crate::{
    api,
    api::dto::scans::{
        PreferencesResponse, ScanAction, ScanActionRequest, ScanDetailResponse, ScanIdResponse,
        ScanRequest, ScanResultResponse, ScanStatus, ScanStatusResponse,
    },
    app::AppState,
    storage::interface::{ResultRecord, ScanRecord, StorageError, parse_range},
};

/// Query parameters for the GET `/scans/:id/results` endpoint.
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

// ─── Result conversion ────────────────────────────────────────────────────────

/// Convert a storage result record into an API response.
fn result_response(r: ResultRecord) -> ScanResultResponse {
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
/// Returns 201 Created with the generated scan ID if successful.
pub async fn create_scan(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let scan = ScanRecord {
        id: id.clone(),
        target: req.target,
        scan_preferences: req.scan_preferences,
        vts: req.vts,
        status: ScanStatus::Stored,
        start_time: None,
        end_time: None,
    };
    match state.storage.create_scan(scan).await {
        Ok(()) => (StatusCode::CREATED, Json(ScanIdResponse { id })).into_response(),
        Err(e) => storage_err(e),
    }
}

/// GET /scans/preferences — Retrieve available scan preferences.
pub async fn get_scan_preferences() -> Json<PreferencesResponse> {
    Json(PreferencesResponse::default())
}

/// GET /scans/:id — Retrieve scan details.
///
/// Returns the target, scan preferences, and VTs for the requested scan.
/// Returns 404 if the scan does not exist.
pub async fn get_scan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.storage.get_scan(&id).await {
        Ok(scan) => Json(ScanDetailResponse {
            scan_id: scan.id,
            target: scan.target,
            scan_preferences: scan.scan_preferences,
            vts: scan.vts,
        })
        .into_response(),
        Err(e) => storage_err(e),
    }
}

/// POST /scans/:id — Perform an action on a scan (start or stop).
///
/// Enforces state transitions: Start only from Stored/Succeeded/Failed,
/// Stop only from Requested/Running. Returns 406 if transition is invalid.
pub async fn scan_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ScanActionRequest>,
) -> impl IntoResponse {
    let scan = match state.storage.get_scan(&id).await {
        Ok(s) => s,
        Err(e) => return storage_err(e),
    };

    let new_status = match req.action {
        ScanAction::Start => match scan.status {
            ScanStatus::Stored | ScanStatus::Succeeded | ScanStatus::Failed => {
                ScanStatus::Requested
            }
            _ => return StatusCode::NOT_ACCEPTABLE.into_response(),
        },
        ScanAction::Stop => match scan.status {
            ScanStatus::Requested | ScanStatus::Running => ScanStatus::Stopped,
            _ => return StatusCode::NOT_ACCEPTABLE.into_response(),
        },
    };

    match state.storage.update_scan_status(&id, new_status).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => storage_err(e),
    }
}

/// DELETE /scans/:id — Delete a scan and all its results.
///
/// Returns 406 if the scan is Running or Requested (cannot delete active scans).
pub async fn delete_scan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let scan = match state.storage.get_scan(&id).await {
        Ok(s) => s,
        Err(e) => return storage_err(e),
    };
    if matches!(scan.status, ScanStatus::Running | ScanStatus::Requested) {
        return StatusCode::NOT_ACCEPTABLE.into_response();
    }

    match state.storage.delete_scan(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => storage_err(e),
    }
}

/// GET /scans/:id/results — Retrieve scan results with optional range filtering.
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

    match state.storage.get_results(&id, start, end).await {
        Ok(results) => Json(results.into_iter().map(result_response).collect::<Vec<_>>())
            .into_response(),
        Err(e) => storage_err(e),
    }
}

/// GET /scans/:id/results/:rid — Retrieve a single scan result by index.
///
/// The `:rid` parameter is a 0-based result index. Returns 404 if not found.
pub async fn get_scan_result(
    State(state): State<AppState>,
    Path((id, rid)): Path<(String, String)>,
) -> impl IntoResponse {
    let result_id: i64 = match rid.parse() {
        Ok(n) => n,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.storage.get_result(&id, result_id).await {
        Ok(r) => Json(result_response(r)).into_response(),
        Err(e) => storage_err(e),
    }
}

/// GET /scans/:id/status — Retrieve the current status and timestamps of a scan.
///
/// Returns the status, start time, and end time of the scan.
pub async fn get_scan_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.storage.get_scan(&id).await {
        Ok(scan) => Json(ScanStatusResponse {
            status: scan.status,
            start_time: scan.start_time,
            end_time: scan.end_time,
        })
        .into_response(),
        Err(e) => storage_err(e),
    }
}
