// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP server listener setup.

use crate::app::error::AppError;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

/// Bind a TCP listener to the specified port on all interfaces.
///
/// Returns an error if the port is already in use or permission is denied.
pub async fn bind_tcp(port: u16) -> Result<TcpListener, AppError> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Binding TCP listener to {}", addr);

    TcpListener::bind(addr).await.map_err(AppError::Bind)
}
