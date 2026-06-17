// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use tempfile::TempDir;

use super::{StorageError, StorageHandle, sqlite::SqliteStorage};

pub(crate) async fn temporary_sqlite_storage() -> Result<(StorageHandle, TempDir), StorageError> {
    let temp_dir = tempfile::tempdir().map_err(|err| {
        StorageError::Backend(format!(
            "failed to create temporary SQLite directory: {err}"
        ))
    })?;
    let database_path = temp_dir.path().join("scans.db");
    let database_url = format!("sqlite:{}", database_path.display());
    let storage = SqliteStorage::new(&database_url).await?;

    Ok((Arc::new(storage), temp_dir))
}

#[cfg(test)]
#[path = "test_support_tests.rs"]
mod test_support_tests;
