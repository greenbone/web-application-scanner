// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::{
    Router,
    routing::{get, head},
};
use tower_http::trace::TraceLayer;

use crate::{api, app::AppState};

pub static API_BASE_PATH: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| format!("/api/{}", api::API_VERSION));

pub fn build_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/health", head(api::health::head_health))
        .route("/health/alive", get(api::health::get_health_alive))
        .route("/health/ready", get(api::health::get_health_ready))
        .route("/health/started", get(api::health::get_health_started));

    // TODO: Add authentication middleware to private routes
    let private_routes = Router::new()
        .route("/scans", head(api::scans::head_scans).post(api::scans::create_scan))
        .route("/scans/preferences", get(api::scans::get_scan_preferences))
        .route(
            "/scans/:id",
            get(api::scans::get_scan)
                .post(api::scans::scan_action)
                .delete(api::scans::delete_scan),
        )
        .route("/scans/:id/results", get(api::scans::get_scan_results))
        .route("/scans/:id/results/:rid", get(api::scans::get_scan_result))
        .route("/scans/:id/status", get(api::scans::get_scan_status));

    Router::new()
        .nest(&API_BASE_PATH, public_routes)
        .nest(&API_BASE_PATH, private_routes)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
