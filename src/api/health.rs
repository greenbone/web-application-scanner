// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::Json;

pub async fn get_health() -> Json<&'static str> {
    Json("This is the health endpoint")
}
