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
pub mod storage;
pub mod zapclient;

use std::sync::Arc;

use crate::{
    app::{AppState, error::AppError},
    config::settings::{Settings, StorageBackend},
    storage::{
        in_memory::InMemoryStorage,
        sqlite::SqliteStorage,
    },
};

use tracing::info;

/// Initialize and run the web application scanner service. 
pub async fn run() -> Result<(), AppError> {
    let settings = Settings::load()?;
    logging::init_logging(&settings);

    let storage: Arc<dyn storage::ScanStorage> = match settings.storage_backend {
        StorageBackend::InMemory => {
            info!("Using in-memory storage backend");
            Arc::new(InMemoryStorage::new())
        }
        StorageBackend::Sqlite => {
            let url = settings.sqlite_url.as_deref().unwrap(); // validated in settings
            info!("Using SQLite storage backend: {}", url);
            Arc::new(
                SqliteStorage::new(url)
                    .await
                    .map_err(|e| AppError::Storage(e.to_string()))?,
            )
        }
    };

    let state = AppState::new(storage);
    let router = http::router::build_router(state);
    let listener = http::listener::bind_tcp(settings.port).await?;

    info!("Starting HTTP server on port {}", settings.port);
    axum::serve(listener, router)
        .await
        .map_err(AppError::Server)?;

    Ok(())
}
