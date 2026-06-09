// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use crate::{
    app::AppState,
    config::settings::SQLITE_IN_MEMORY_URL,
    scan::{DefaultScanService, ScanServiceHandle},
    storage::sqlite::SqliteStorage,
};

use super::build_router;

#[tokio::test]
async fn build_router_accepts_all_route_patterns() {
    let storage = Arc::new(SqliteStorage::new(SQLITE_IN_MEMORY_URL).await.unwrap());
    let scan_service: ScanServiceHandle =
        Arc::new(DefaultScanService::new_storage_only(storage.clone()));
    let state = AppState::new(storage, scan_service);

    let _router = build_router(state);
}
