// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::{
    Json,
    extract::{Path, Query},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    api,
    api::dto::scans::{
        PreferencesResponse, ScanActionRequest, ScanIdResponse, ScanRequest, ScanResponse,
        ScanResultResponse, ScanStatus,
    },
};

#[derive(Debug, Deserialize)]
pub struct ResultRangeQuery {
    pub range: Option<String>,
}

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

pub async fn create_scan(Json(_scan): Json<ScanRequest>) -> impl IntoResponse {
    let response = ScanIdResponse {
        id: "scan-placeholder".to_string(),
    };

    (StatusCode::CREATED, Json(response))
}

pub async fn get_scan_preferences() -> Json<PreferencesResponse> {
    Json(PreferencesResponse::default())
}

pub async fn get_scan(Path(id): Path<String>) -> Json<ScanResponse> {
    Json(ScanResponse {
        id,
        status: ScanStatus::Stored,
    })
}

pub async fn scan_action(Path(_id): Path<String>, Json(_action): Json<ScanActionRequest>) -> StatusCode {
    StatusCode::NO_CONTENT
}

pub async fn delete_scan(Path(_id): Path<String>) -> StatusCode {
    StatusCode::NO_CONTENT
}

pub async fn get_scan_results(
    Path(_id): Path<String>,
    Query(_query): Query<ResultRangeQuery>,
) -> Json<Vec<ScanResultResponse>> {
    let mut results: Vec<ScanResultResponse> = vec![];

    for i in 0..5 {
        results.push(ScanResultResponse {
            id: format!("result-{}", i),
        });
    }

    Json(results)
}

pub async fn get_scan_result(Path((_id, rid)): Path<(String, String)>) -> Json<ScanResultResponse> {
    Json(ScanResultResponse { id: rid })
}

pub async fn get_scan_status(Path(id): Path<String>) -> Json<ScanResponse> {
    Json(ScanResponse {
        id,
        status: ScanStatus::Stored,
    })
}
