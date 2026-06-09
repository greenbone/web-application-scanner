// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions},
};
use std::str::FromStr;

use crate::{
    api::dto::scans::{ResultType, ScannerPreference, Target, Vt},
    scan::ScanStatus,
};

use super::interface::{ResultRecord, ScanRecord, ScanStorage, StorageError};

/// SQLite-backed storage using an async connection pool.
///
/// Opens a SQLite database with WAL mode and foreign keys enabled.
/// Schema is automatically created on initialization.
pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    /// Open (or create) a SQLite database at `url` and ensure the schema exists.
    ///
    /// `url` follows the SQLite connection string format accepted by sqlx, for
    /// example `sqlite:path/to/scans.db` or `sqlite::memory:`.
    pub async fn new(url: &str) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str(url)
            .map_err(|e| StorageError::Backend(e.to_string()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let storage = Self { pool };
        storage.init_schema().await?;
        Ok(storage)
    }

    async fn init_schema(&self) -> Result<(), StorageError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scans (
                id               TEXT    PRIMARY KEY,
                target           TEXT    NOT NULL,
                scan_preferences TEXT    NOT NULL DEFAULT '[]',
                vts              TEXT    NOT NULL DEFAULT '[]',
                status           TEXT    NOT NULL DEFAULT 'new',
                queued_time      INTEGER,
                start_time       INTEGER,
                end_time         INTEGER,
                context_name     TEXT,
                context_id       TEXT,
                alert_cursor     INTEGER,
                progress         TEXT,
                interruption_reason TEXT
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS scan_results (
                id          INTEGER NOT NULL,
                scan_id     TEXT    NOT NULL,
                result_type TEXT    NOT NULL,
                ip_address  TEXT,
                hostname    TEXT,
                oid         TEXT,
                port        INTEGER,
                protocol    TEXT,
                message     TEXT,
                detail      TEXT,
                PRIMARY KEY (scan_id, id),
                FOREIGN KEY (scan_id) REFERENCES scans(id) ON DELETE CASCADE
            )",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(())
    }
}

// ─── Serialisation helpers ────────────────────────────────────────────────────

fn status_to_db(status: &ScanStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "new".to_string())
}

fn status_from_db(s: &str) -> Result<ScanStatus, StorageError> {
    // Backward compatibility with pre-Phase-0 persisted values.
    let normalized = match s {
        "stored" => "new",
        "requested" => "queued",
        "failed" => "interrupted",
        "succeeded" => "done",
        _ => s,
    };

    serde_json::from_value(serde_json::Value::String(normalized.to_string())).map_err(|e| {
        StorageError::Backend(format!("unrecognised scan status '{normalized}': {e}"))
    })
}

fn result_type_to_db(rt: &ResultType) -> String {
    serde_json::to_value(rt)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "log".to_string())
}

fn result_type_from_db(s: &str) -> Result<ResultType, StorageError> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| StorageError::Backend(format!("unrecognised result type '{s}': {e}")))
}

fn target_to_db(target: &Target) -> Result<String, StorageError> {
    serde_json::to_string(target).map_err(|e| StorageError::Backend(e.to_string()))
}

fn target_from_db(s: &str) -> Result<Target, StorageError> {
    serde_json::from_str(s).map_err(|e| StorageError::Backend(e.to_string()))
}

fn prefs_to_db(prefs: &[ScannerPreference]) -> Result<String, StorageError> {
    serde_json::to_string(prefs).map_err(|e| StorageError::Backend(e.to_string()))
}

fn prefs_from_db(s: &str) -> Result<Vec<ScannerPreference>, StorageError> {
    serde_json::from_str(s).map_err(|e| StorageError::Backend(e.to_string()))
}

fn vts_to_db(vts: &[Vt]) -> Result<String, StorageError> {
    serde_json::to_string(vts).map_err(|e| StorageError::Backend(e.to_string()))
}

fn vts_from_db(s: &str) -> Result<Vec<Vt>, StorageError> {
    serde_json::from_str(s).map_err(|e| StorageError::Backend(e.to_string()))
}

fn detail_to_db(detail: &Option<serde_json::Value>) -> Option<String> {
    detail.as_ref().and_then(|v| serde_json::to_string(v).ok())
}

fn detail_from_db(s: Option<&str>) -> Option<serde_json::Value> {
    s.and_then(|v| serde_json::from_str(v).ok())
}

fn progress_to_db(progress: &Option<serde_json::Value>) -> Option<String> {
    progress
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok())
}

fn progress_from_db(s: Option<&str>) -> Option<serde_json::Value> {
    s.and_then(|value| serde_json::from_str(value).ok())
}

// ─── Trait implementation ─────────────────────────────────────────────────────

#[async_trait]
impl ScanStorage for SqliteStorage {
    async fn create_scan(&self, scan: ScanRecord) -> Result<(), StorageError> {
        let target_json = target_to_db(&scan.target)?;
        let prefs_json = prefs_to_db(&scan.scan_preferences)?;
        let vts_json = vts_to_db(&scan.vts)?;
        let status_str = status_to_db(&scan.status);
        let progress_json = progress_to_db(&scan.progress);

        sqlx::query(
            "INSERT INTO scans (
                id, target, scan_preferences, vts, status, queued_time, start_time, end_time,
                context_name, context_id, alert_cursor, progress, interruption_reason
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&scan.id)
        .bind(&target_json)
        .bind(&prefs_json)
        .bind(&vts_json)
        .bind(&status_str)
        .bind(scan.queued_time)
        .bind(scan.start_time)
        .bind(scan.end_time)
        .bind(&scan.context_name)
        .bind(&scan.context_id)
        .bind(scan.alert_cursor)
        .bind(progress_json.as_deref())
        .bind(&scan.interruption_reason)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.kind() == sqlx::error::ErrorKind::UniqueViolation =>
            {
                StorageError::AlreadyExists(scan.id.clone())
            }
            other => StorageError::Backend(other.to_string()),
        })?;

        Ok(())
    }

    async fn get_scan(&self, id: &str) -> Result<ScanRecord, StorageError> {
        let row = sqlx::query(
            "SELECT id, target, scan_preferences, vts, status, queued_time, start_time, end_time,
                    context_name, context_id, alert_cursor, progress, interruption_reason
             FROM scans WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?
        .ok_or_else(|| StorageError::NotFound(id.to_string()))?;

        scan_from_row(&row)
    }

    async fn list_non_terminal_scans(&self) -> Result<Vec<ScanRecord>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, target, scan_preferences, vts, status, queued_time, start_time, end_time,
                    context_name, context_id, alert_cursor, progress, interruption_reason
             FROM scans
             WHERE status NOT IN ('done', 'stopped', 'interrupted')
             ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        rows.iter().map(scan_from_row).collect()
    }

    async fn update_scan_status(&self, id: &str, status: ScanStatus) -> Result<(), StorageError> {
        let status_str = status_to_db(&status);
        let result = sqlx::query("UPDATE scans SET status = ? WHERE id = ?")
            .bind(&status_str)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        Ok(())
    }

    async fn transition_scan_status(
        &self,
        id: &str,
        expected: ScanStatus,
        new_status: ScanStatus,
    ) -> Result<(), StorageError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let result = sqlx::query("UPDATE scans SET status = ? WHERE id = ? AND status = ?")
            .bind(status_to_db(&new_status))
            .bind(id)
            .bind(status_to_db(&expected))
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM scans WHERE id = ?")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| StorageError::Backend(e.to_string()))?;

            tx.rollback().await.ok();

            if exists.is_some() {
                return Err(StorageError::InvalidState);
            }
            return Err(StorageError::NotFound(id.to_string()));
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))
    }

    async fn update_scan_progress(
        &self,
        id: &str,
        progress: Option<serde_json::Value>,
    ) -> Result<(), StorageError> {
        let progress_json = progress_to_db(&progress);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let result = sqlx::query("UPDATE scans SET progress = ? WHERE id = ?")
            .bind(progress_json.as_deref())
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            tx.rollback().await.ok();
            return Err(StorageError::NotFound(id.to_string()));
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))
    }

    async fn update_scan_context(
        &self,
        id: &str,
        context_name: Option<String>,
        context_id: Option<String>,
    ) -> Result<(), StorageError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let result = sqlx::query("UPDATE scans SET context_name = ?, context_id = ? WHERE id = ?")
            .bind(&context_name)
            .bind(&context_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            tx.rollback().await.ok();
            return Err(StorageError::NotFound(id.to_string()));
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))
    }

    async fn update_alert_cursor(&self, id: &str, alert_cursor: Option<i64>) -> Result<(), StorageError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let result = sqlx::query("UPDATE scans SET alert_cursor = ? WHERE id = ?")
            .bind(alert_cursor)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            tx.rollback().await.ok();
            return Err(StorageError::NotFound(id.to_string()));
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))
    }

    async fn delete_scan(&self, id: &str) -> Result<(), StorageError> {
        let result = sqlx::query("DELETE FROM scans WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(id.to_string()));
        }
        Ok(())
    }

    async fn add_result(&self, scan_id: &str, result: ResultRecord) -> Result<(), StorageError> {
        self.add_results(scan_id, vec![result]).await
    }

    async fn add_results(&self, scan_id: &str, results: Vec<ResultRecord>) -> Result<(), StorageError> {
        // Verify scan exists first.
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scans WHERE id = ?")
            .bind(scan_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if count == 0 {
            tx.rollback().await.ok();
            return Err(StorageError::NotFound(scan_id.to_string()));
        }

        for result in results {
            let rt_str = result_type_to_db(&result.result_type);
            let detail_str = detail_to_db(&result.detail);

            // The result id is auto-assigned as (MAX(id)+1) within the scan, starting at 0.
            sqlx::query(
                "INSERT INTO scan_results
                    (id, scan_id, result_type, ip_address, hostname, oid, port, protocol, message, detail)
                 VALUES (
                    (SELECT COALESCE(MAX(id) + 1, 0) FROM scan_results WHERE scan_id = ?),
                    ?, ?, ?, ?, ?, ?, ?, ?, ?
                 )",
            )
            .bind(scan_id)
            .bind(scan_id)
            .bind(&rt_str)
            .bind(&result.ip_address)
            .bind(&result.hostname)
            .bind(&result.oid)
            .bind(result.port)
            .bind(&result.protocol)
            .bind(&result.message)
            .bind(detail_str.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn get_result(
        &self,
        scan_id: &str,
        result_id: i64,
    ) -> Result<ResultRecord, StorageError> {
        let row = sqlx::query(
            "SELECT id, scan_id, result_type, ip_address, hostname, oid, port, protocol, message, detail
             FROM scan_results WHERE scan_id = ? AND id = ?",
        )
        .bind(scan_id)
        .bind(result_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?
        .ok_or_else(|| StorageError::ResultNotFound(scan_id.to_string(), result_id))?;

        result_from_row(&row)
    }

    async fn get_results(
        &self,
        scan_id: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<ResultRecord>, StorageError> {
        // Verify scan exists.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scans WHERE id = ?")
            .bind(scan_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Backend(e.to_string()))?;

        if count == 0 {
            return Err(StorageError::NotFound(scan_id.to_string()));
        }

        let limit: i64 = match end {
            Some(e) => (e as i64) - (start as i64) + 1,
            None => -1, // SQLite: -1 means no limit
        };
        let offset = start as i64;

        let rows = sqlx::query(
            "SELECT id, scan_id, result_type, ip_address, hostname, oid, port, protocol, message, detail
             FROM scan_results WHERE scan_id = ?
             ORDER BY id ASC
             LIMIT ? OFFSET ?",
        )
        .bind(scan_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Backend(e.to_string()))?;

        rows.iter().map(result_from_row).collect()
    }
}

// ─── Row mapping helpers ──────────────────────────────────────────────────────

fn scan_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ScanRecord, StorageError> {
    let id: String = row
        .try_get("id")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let target_json: String = row
        .try_get("target")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let prefs_json: String = row
        .try_get("scan_preferences")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let vts_json: String = row
        .try_get("vts")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let queued_time: Option<i64> = row
        .try_get("queued_time")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let start_time: Option<i64> = row
        .try_get("start_time")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let end_time: Option<i64> = row
        .try_get("end_time")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let context_name: Option<String> = row
        .try_get("context_name")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let context_id: Option<String> = row
        .try_get("context_id")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let alert_cursor: Option<i64> = row
        .try_get("alert_cursor")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let progress: Option<String> = row
        .try_get("progress")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let interruption_reason: Option<String> = row
        .try_get("interruption_reason")
        .map_err(|e| StorageError::Backend(e.to_string()))?;

    Ok(ScanRecord {
        id,
        target: target_from_db(&target_json)?,
        scan_preferences: prefs_from_db(&prefs_json)?,
        vts: vts_from_db(&vts_json)?,
        status: status_from_db(&status_str)?,
        queued_time,
        start_time,
        end_time,
        context_name,
        context_id,
        alert_cursor,
        progress: progress_from_db(progress.as_deref()),
        interruption_reason,
    })
}

fn result_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ResultRecord, StorageError> {
    let id: i64 = row
        .try_get("id")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let scan_id: String = row
        .try_get("scan_id")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let result_type_str: String = row
        .try_get("result_type")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let ip_address: Option<String> = row
        .try_get("ip_address")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let hostname: Option<String> = row
        .try_get("hostname")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let oid: Option<String> = row
        .try_get("oid")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let port: Option<i32> = row
        .try_get("port")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let protocol: Option<String> = row
        .try_get("protocol")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let message: Option<String> = row
        .try_get("message")
        .map_err(|e| StorageError::Backend(e.to_string()))?;
    let detail_str: Option<String> = row
        .try_get("detail")
        .map_err(|e| StorageError::Backend(e.to_string()))?;

    Ok(ResultRecord {
        id,
        scan_id,
        result_type: result_type_from_db(&result_type_str)?,
        ip_address,
        hostname,
        oid,
        port,
        protocol,
        message,
        detail: detail_from_db(detail_str.as_deref()),
    })
}

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
