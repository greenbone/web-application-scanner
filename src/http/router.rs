// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

use crate::api;

pub const API_BASE_PATH: &str = "/api/v1";

pub fn build_router() -> Router {
    let public_routes = Router::new().route("/health", get(api::health::get_health));

    // TODO: Add authentication middleware to private routes
    let private_routes = Router::new().route("/scans", get(api::scans::get_scans));

    Router::new()
        .nest(API_BASE_PATH, public_routes)
        .nest(API_BASE_PATH, private_routes)
        .layer(TraceLayer::new_for_http())
}
