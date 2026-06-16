// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[tokio::test]
async fn temporary_sqlite_storage_opens_file_backed_database() {
    let (storage, _temp_dir) = temporary_sqlite_storage().await.unwrap();

    assert!(matches!(
        storage.get_scan("missing").await.unwrap_err(),
        StorageError::NotFound(_)
    ));
}
