// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
};

use crate::api;

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

pub async fn get_health_alive() -> StatusCode {
    StatusCode::OK
}

pub async fn get_health_ready() -> StatusCode {
    StatusCode::OK
}

pub async fn get_health_started() -> StatusCode {
    StatusCode::OK
}
