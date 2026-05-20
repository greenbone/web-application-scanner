// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[tokio::main]
async fn main() {
    greenbone_was::run().await.expect("application failed");
}
