// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod api;
pub mod app;
pub mod config;
pub mod http;
pub mod logging;

use crate::{app::error::AppError, config::settings::Settings};

use tracing::info;

pub async fn run() -> Result<(), AppError> {
    let settings = Settings::load()?;
    logging::init_logging(&settings);

    let router = http::router::build_router();
    let listener = http::listener::bind_tcp(settings.port).await?;

    info!("Starting HTTP server on port {}", settings.port);
    axum::serve(listener, router)
        .await
        .map_err(AppError::Server)?;

    Ok(())
}
