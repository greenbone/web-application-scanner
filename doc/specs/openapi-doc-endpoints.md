# OpenAPI 3 Documentation Endpoints

**Date**: June 22, 2026  
**Objective**: Add dynamically generated OpenAPI 3 documentation endpoints serving YAML and HTML (Swagger UI) as an optional, toggleable feature.

## Overview

Implement code-first OpenAPI specification generation using `utoipa` 5.x behind a Cargo feature flag. When enabled, the service will expose three documentation endpoints:

- `GET /doc` → **Swagger UI** (interactive HTML documentation)
- `GET /doc/openapi.yml` → **OpenAPI 3.0 YAML spec** (machine-readable)
- `GET /doc/openapi.json` → **OpenAPI 3.0 JSON spec** (internal; Swagger UI consumes this)

All assets are embedded in the binary and served locally — no CDN or external dependencies.

### Feature Flag

- **Name**: `api-docs`
- **Default**: Enabled
- **Dependencies**: `utoipa` 5.x (with `axum_extras` feature), `utoipa-swagger-ui` 9.x (with `axum` feature)
- **Usage**:
  - Default build (docs enabled): `cargo build`
  - Production build (docs disabled): `cargo build --release --no-default-features`

## Technical Stack

- **`utoipa` 5.x** — Code-first OpenAPI annotation framework for Rust
  - Feature: `axum_extras` — enables automatic `Path<T>` and `Query<T>` parameter extraction
  - Optional dependency; only compiled when `api-docs` feature is enabled
- **`utoipa-swagger-ui` 9.x** — Swagger UI Axum integration
  - Feature: `axum` — Axum route integration
  - Embeds Swagger UI assets directly into the binary
  - Supports OpenAPI 3.0
  - Optional dependency; only compiled when `api-docs` feature is enabled
- **`serde_yaml`** — Already in dependencies; used to serialize the OpenAPI spec to YAML

## Implementation Plan

### Phase 1: Add Feature Flag and Optional Dependencies

**File**: `Cargo.toml`

Add to `[features]`:

```toml
[features]
default = ["api-docs"]
api-docs = ["utoipa", "utoipa-swagger-ui"]
```

Add to `[dependencies]`:

```toml
utoipa = { version = "5", features = ["axum_extras"], optional = true }
utoipa-swagger-ui = { version = "9", features = ["axum"], optional = true }
```

### Phase 2: Add `ToSchema` to DTO Types

**Files**: 
- `src/api/dto/scans.rs`
- `src/scan/status.rs`

Add `#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]` to all public DTO types:

**In `src/api/dto/scans.rs`:**
- `ScanRequest`
- `ScanActionRequest`
- `ScanAction`
- `ScanDetailResponse`
- `ScanStatusResponse`
- `ScanResultResponse`
- `PreferencesResponse`
- `Target`
- `Credential`
- `UsernamePasswordCredential`
- `ScannerPreference`
- `Vt`
- `Parameter`
- `ResultType`
- `HostInfo`

**In `src/scan/status.rs`:**
- `ScanStatus`

**Example:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ScanRequest {
    // ...
}
```

### Phase 3: Add Path Annotations to Handlers

**Files**:
- `src/api/health.rs`
- `src/api/scans.rs`

Add `#[cfg_attr(feature = "api-docs", utoipa::path(...))]` macro to all handler functions. Each annotation specifies:
- HTTP method (`get`, `post`, `delete`, `head`)
- Path pattern (`/health`, `/scans`, `/scans/{id}`, etc.)
- Path parameters (if any)
- Query parameters (with `Query` struct reference)
- Request body (with `request_body` clause)
- Response codes and schemas

**Example structure:**

```rust
#[cfg_attr(
    feature = "api-docs",
    utoipa::path(
        post,
        path = "/scans",
        request_body = ScanRequest,
        responses(
            (status = 201, description = "Scan created", body = String),
            (status = 400, description = "Bad Request"),
            (status = 403, description = "Scan ID already in use"),
        ),
        tag = "scan"
    )
)]
pub async fn create_scan(/* ... */) {
    // ...
}
```

**Handlers to annotate:**

Health endpoints (4):
- `head_health`
- `get_health_alive`
- `get_health_ready`
- `get_health_started`

Scan endpoints (10):
- `head_scans`
- `create_scan`
- `get_scan_preferences`
- `get_scan`
- `scan_action`
- `delete_scan`
- `get_scan_results` (with `Query<ResultRangeQuery>`)
- `get_scan_result` (with `Path<{id, rid}>`)
- `get_scan_status`

### Phase 4: Create OpenAPI Definition Module

**File**: `src/api/openapi.rs` (new, only compiled when `api-docs` feature is enabled)

Define the OpenAPI document aggregator and YAML handler:

```rust
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
```

### Phase 5: Declare the New Module

**File**: `src/api/mod.rs`

Add:

```rust
#[cfg(feature = "api-docs")]
pub mod openapi;
```

### Phase 6: Update Router

**File**: `src/http/router.rs`

Import and conditionally wire up Swagger UI and the YAML endpoint:

```rust
#[cfg(feature = "api-docs")]
use utoipa_swagger_ui::SwaggerUi;

pub fn build_router(state: AppState) -> Router {
    // ... existing routes ...

    let mut router = Router::new()
        .nest(&API_BASE_PATH, public_routes)
        .nest(&API_BASE_PATH, private_routes)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    #[cfg(feature = "api-docs")]
    {
        use crate::api::openapi::ApiDoc;
        let swagger_ui = SwaggerUi::new("/doc")
            .url("/doc/openapi.json", ApiDoc::openapi());
        router = router
            .route("/doc/openapi.yml", get(api::openapi::get_openapi_yaml))
            .merge(swagger_ui);
    }

    router
}
```

### Phase 7: Add Tests

**File**: `src/api/openapi_tests.rs` (new sidecar, only compiled when `api-docs` feature is enabled)

Add to top of file:

```rust
#![cfg(feature = "api-docs")]
```

Test that the documentation endpoints are accessible and return valid content:

```rust
//! Tests for OpenAPI documentation endpoints.

#![cfg(feature = "api-docs")]

use std::sync::Arc;

use crate::{
    app::AppState,
    http::router::build_router,
    scan::DefaultScanService,
    storage::test_support::temporary_sqlite_storage,
};

#[tokio::test]
async fn swagger_ui_endpoint_returns_html() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let scan_service = Arc::new(DefaultScanService::new_storage_only(storage.clone()));
    let state = AppState::new(storage, scan_service);

    let router = build_router(state);
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/doc")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("swagger-ui"), "Response should contain Swagger UI HTML");
}

#[tokio::test]
async fn openapi_yaml_endpoint_returns_valid_yaml() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let scan_service = Arc::new(DefaultScanService::new_storage_only(storage.clone()));
    let state = AppState::new(storage, scan_service);

    let router = build_router(state);
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/doc/openapi.yml")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let content_type = response
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.starts_with("application/yaml"),
        "Content-Type should be application/yaml"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let yaml = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        yaml.contains("openapi: '3"),
        "YAML should contain OpenAPI 3.0 version"
    );
    assert!(
        yaml.contains("Web Application Scanner"),
        "YAML should contain API title"
    );
}

#[tokio::test]
async fn openapi_json_endpoint_returns_valid_json() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();
    let scan_service = Arc::new(DefaultScanService::new_storage_only(storage.clone()));
    let state = AppState::new(storage, scan_service);

    let router = build_router(state);
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/doc/openapi.json")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_str = String::from_utf8(body.to_vec()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .expect("Response should be valid JSON");

    assert_eq!(json["openapi"], "3.0.3", "Should be OpenAPI 3.0.3");
    assert_eq!(
        json["info"]["title"],
        "Web Application Scanner",
        "Should have correct API title"
    );
}
```

## Build Variants

### Default Build (with API Docs)
```bash
cargo build
cargo run
# Endpoints available:
#  GET /doc → Swagger UI
#  GET /doc/openapi.yml → YAML spec
#  GET /doc/openapi.json → JSON spec
```

### Production Build (without API Docs)
```bash
cargo build --release --no-default-features
# Documentation endpoints NOT available
# Smaller binary size, no utoipa/utoipa-swagger-ui code included
```

### Test All Variants
```bash
# Test with feature enabled (default)
cargo test

# Test without feature
cargo test --no-default-features
```

## Verification Checklist

- [ ] **Feature flag**: `[features]` section in `Cargo.toml` defines `api-docs` with default enabled
- [ ] **Optional deps**: `utoipa` and `utoipa-swagger-ui` marked as `optional = true` and linked in feature
- [ ] **Build**: `cargo build` compiles without errors (with `api-docs` feature)
- [ ] **No-feature build**: `cargo build --no-default-features` compiles without errors
- [ ] **Tests**: `cargo test` passes (tests conditional on `#[cfg(feature = "api-docs")]`)
- [ ] **Feature-gated module**: `src/api/openapi.rs` only compiled with feature
- [ ] **Feature-gated handlers**: `#[cfg_attr(...)]` used on all `ToSchema`, `#[utoipa::path(...)]` annotations
- [ ] **Feature-gated router**: SwaggerUi wiring only executed with feature enabled
- [ ] **Swagger UI**: `GET /doc` returns HTML with Swagger UI interface
- [ ] **YAML Spec**: `GET /doc/openapi.yml` returns valid YAML starting with `openapi: '3`
- [ ] **JSON Spec**: `GET /doc/openapi.json` returns valid JSON OpenAPI document
- [ ] **Completeness**: Generated spec includes all 14 handlers and all DTO schemas
- [ ] **Consistency**: Spot-check: generated paths and schemas align with [openapi-reference.yml](openapi-reference.yml)
- [ ] **Local Assets**: No network requests made during Swagger UI initialization

## Notes

- The `doc/openapi-reference.yml` file is kept as-is, serving as a reference/guideline for spec consistency
- All Swagger UI assets are embedded in the binary; zero CDN/external dependencies (when feature is enabled)
- The generated OpenAPI spec is derived from actual code annotations, ensuring it stays synchronized with implementation
- Health endpoints are included in the public API documentation (they are part of the spec)
- The feature flag allows the API docs feature to be completely excluded from production builds for security or size concerns
- When the `api-docs` feature is disabled, all documentation-related code is not compiled, eliminating the dependency entirely
