// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Health check endpoints.
//!
//! Provides endpoints for orchestration systems and monitoring tools to verify
//! server availability, readiness, and liveness.

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
};

use crate::api;

/// HEAD /health — Return API version and authentication info as headers.
pub async fn head_health() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("api-version"),
        HeaderValue::from_static(api::API_VERSION),
    );
    headers.insert(
        HeaderName::from_static("authentication"),
        HeaderValue::from_static("none"),
    );

    (StatusCode::OK, headers)
}

/// GET /health/alive — Check if the server is alive.
pub async fn get_health_alive() -> StatusCode {
    StatusCode::OK
}

/// GET /health/ready — Check if the server is ready to serve traffic.
pub async fn get_health_ready() -> StatusCode {
    StatusCode::OK
}

/// GET /health/started — Check if the server has started up.
pub async fn get_health_started() -> StatusCode {
    StatusCode::OK
}
