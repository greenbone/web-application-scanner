//! Tests for OpenAPI documentation endpoints.

#![cfg(feature = "api-docs")]

use std::sync::Arc;

use axum::{body::to_bytes, response::IntoResponse};

use crate::{
    app::AppState, http::router::build_router, scan::DefaultScanService,
    storage::test_support::temporary_sqlite_storage,
};

#[tokio::test]
async fn router_builds_successfully_with_openapi_endpoints() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let scan_service = Arc::new(DefaultScanService::new_storage_only(storage.clone()));
    let state = AppState::new(storage, scan_service);

    // Just verify the router builds without panicking
    let _router = build_router(state);
}

#[tokio::test]
async fn openapi_yaml_handler_returns_valid_yaml() {
    use crate::api::openapi::get_openapi_yaml;

    let response = get_openapi_yaml().await.into_response();
    assert_eq!(response.status(), 200);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let yaml = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        yaml.contains("openapi:"),
        "YAML should contain openapi version"
    );
    assert!(
        yaml.contains("Web Application Scanner"),
        "YAML should contain API title"
    );
    assert!(
        yaml.contains("Greenbone AG"),
        "YAML should contain contact information"
    );
}

#[tokio::test]
async fn openapi_spec_includes_all_health_endpoints() {
    use crate::api::openapi::ApiDoc;
    use utoipa::OpenApi;

    let openapi = ApiDoc::openapi();
    let spec_json = serde_json::to_value(&openapi).unwrap();

    // Verify all health endpoints are documented
    let paths = &spec_json["paths"];
    assert!(
        paths["/health"].is_object(),
        "Spec should document /health endpoint"
    );
    assert!(
        paths["/health/alive"].is_object(),
        "Spec should document /health/alive endpoint"
    );
    assert!(
        paths["/health/ready"].is_object(),
        "Spec should document /health/ready endpoint"
    );
    assert!(
        paths["/health/started"].is_object(),
        "Spec should document /health/started endpoint"
    );
}

#[tokio::test]
async fn openapi_spec_includes_all_scan_endpoints() {
    use crate::api::openapi::ApiDoc;
    use utoipa::OpenApi;

    let openapi = ApiDoc::openapi();
    let spec_json = serde_json::to_value(&openapi).unwrap();

    // Verify all scan endpoints are documented
    let paths = &spec_json["paths"];
    assert!(
        paths["/scans"].is_object(),
        "Spec should document /scans endpoint"
    );
    assert!(
        paths["/scans/preferences"].is_object(),
        "Spec should document /scans/preferences endpoint"
    );
    assert!(
        paths["/scans/{id}"].is_object(),
        "Spec should document /scans/{{id}} endpoint"
    );
    assert!(
        paths["/scans/{id}/results"].is_object(),
        "Spec should document /scans/{{id}}/results endpoint"
    );
    assert!(
        paths["/scans/{id}/results/{rid}"].is_object(),
        "Spec should document /scans/{{id}}/results/{{rid}} endpoint"
    );
    assert!(
        paths["/scans/{id}/status"].is_object(),
        "Spec should document /scans/{{id}}/status endpoint"
    );
}

#[tokio::test]
async fn openapi_spec_includes_all_dto_schemas() {
    use crate::api::openapi::ApiDoc;
    use utoipa::OpenApi;

    let openapi = ApiDoc::openapi();
    let spec_json = serde_json::to_value(&openapi).unwrap();

    // Verify all DTO schemas are documented
    let components = &spec_json["components"]["schemas"];
    assert!(components["ScanRequest"].is_object());
    assert!(components["ScanActionRequest"].is_object());
    assert!(components["ScanAction"].is_object());
    assert!(components["ScanDetailResponse"].is_object());
    assert!(components["ScanStatusResponse"].is_object());
    assert!(components["ScanResultResponse"].is_object());
    assert!(components["PreferencesResponse"].is_object());
    assert!(components["Target"].is_object());
    assert!(components["Credential"].is_object());
    assert!(components["UsernamePasswordCredential"].is_object());
    assert!(components["ScannerPreference"].is_object());
    assert!(components["Vt"].is_object());
    assert!(components["Parameter"].is_object());
    assert!(components["ResultType"].is_object());
    assert!(components["HostInfo"].is_object());
    assert!(components["ScanStatus"].is_object());
}
