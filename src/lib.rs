// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Greenbone Web Application Scanner (WAS)
//!
//! Greenbone WAS is a wrapper for the web application vulnerability scanner
//! *[Zed Attack Proxy (ZAP)](https://www.zaproxy.org/)* that offers an API
//! based on the [openvasd scanner API](https://greenbone.github.io/scanner-api/)
//! to run scans and retrieve results.

pub mod api;
pub mod app;
pub mod config;
pub mod http;
pub mod logging;
pub mod scan;
pub mod storage;
pub mod zapclient;

use std::sync::Arc;

use crate::{
    app::{AppState, error::AppError},
    config::settings::Settings,
    scan::{DefaultScanService, ScanRuntimeConfig, ScanServiceHandle, start_scan_runtime},
    storage::sqlite::SqliteStorage,
    zapclient::ZapClient,
};

use tracing::info;

/// Initialize and run the web application scanner service.
pub async fn run() -> Result<(), AppError> {
    let settings = Settings::load()?;
    logging::init_logging(&settings);

    let url = settings.sqlite_url.as_deref().unwrap(); // validated in settings
    info!("Using SQLite storage backend: {}", url);
    let storage: Arc<dyn storage::ScanStorage> = Arc::new(
        SqliteStorage::new(url)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?,
    );
    let zap_client =
        ZapClient::from_settings(&settings).map_err(|e| AppError::Runtime(e.to_string()))?;
    let runtime = start_scan_runtime(
        storage.clone(),
        zap_client,
        ScanRuntimeConfig {
            worker_count: settings.scan_worker_count,
            alert_poll_interval: std::time::Duration::from_secs(
                settings.scan_alert_poll_interval_seconds,
            ),
            ..ScanRuntimeConfig::default()
        },
    );
    let scan_service: ScanServiceHandle =
        Arc::new(DefaultScanService::new(storage.clone(), runtime));

    let state = AppState::new(storage, scan_service);
    let router = http::router::build_router(state);
    let listener = http::listener::bind_tcp(settings.port).await?;

    info!("Starting HTTP server on port {}", settings.port);
    axum::serve(listener, router)
        .await
        .map_err(AppError::Server)?;

    Ok(())
}
