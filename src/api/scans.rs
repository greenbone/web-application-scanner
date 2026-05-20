// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::Json;

pub async fn get_scans() -> Json<&'static str> {
    Json("This is the scans endpoint")
}
