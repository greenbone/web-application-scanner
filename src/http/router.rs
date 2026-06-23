// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP server router setup.

use axum::{
    Router,
    routing::{get, head},
};
use tower_http::trace::TraceLayer;

#[cfg(feature = "api-docs")]
use {utoipa::OpenApi, utoipa_swagger_ui::SwaggerUi};

use crate::{api, app::AppState};

/// Base path for all API endpoints including the API version.
pub static API_BASE_PATH: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| format!("/api/{}", api::API_VERSION));

/// Build and configure the Axum router with all public and private endpoints.
///
/// Public endpoints handle health checks. Private endpoints handle scan operations.
/// All endpoints are nested under the API base path.
pub fn build_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/health", head(api::health::head_health))
        .route("/health/alive", get(api::health::get_health_alive))
        .route("/health/ready", get(api::health::get_health_ready))
        .route("/health/started", get(api::health::get_health_started));

    // TODO: Add authentication middleware to private routes
    let private_routes = Router::new()
        .route(
            "/scans",
            head(api::scans::head_scans).post(api::scans::create_scan),
        )
        .route("/scans/preferences", get(api::scans::get_scan_preferences))
        .route(
            "/scans/{id}",
            get(api::scans::get_scan)
                .post(api::scans::scan_action)
                .delete(api::scans::delete_scan),
        )
        .route("/scans/{id}/results", get(api::scans::get_scan_results))
        .route(
            "/scans/{id}/results/{rid}",
            get(api::scans::get_scan_result),
        )
        .route("/scans/{id}/status", get(api::scans::get_scan_status));

    #[allow(unused_mut)]
    let mut router = Router::new()
        .nest(&API_BASE_PATH, public_routes)
        .nest(&API_BASE_PATH, private_routes)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    #[cfg(feature = "api-docs")]
    {
        use crate::api::openapi::ApiDoc;
        let swagger_ui = SwaggerUi::new("/doc").url("/doc/openapi.json", ApiDoc::openapi());
        router = router
            .route("/doc/openapi.yml", get(api::openapi::get_openapi_yaml))
            .merge(swagger_ui);
    }

    router
}

#[cfg(test)]
#[path = "router_tests.rs"]
mod router_tests;
