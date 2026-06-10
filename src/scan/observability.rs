// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Observability helpers for scan lifecycle events.

use tracing::info;

use crate::scan::ScanStatus;

pub(crate) fn emit_scan_created(scan_id: &str) {
    info!(scan_id, "scan created");
}

pub(crate) fn emit_scan_deleted(scan_id: &str) {
    info!(scan_id, "scan deleted");
}

pub(crate) fn emit_status_transition(scan_id: &str, from: ScanStatus, to: ScanStatus) {
    info!(
        scan_id,
        from_status = status_label(from),
        to_status = status_label(to),
        "scan status transition"
    );
}

pub(crate) fn emit_queue_wait_telemetry(scan_id: &str, queued_time: i64, start_time: i64) {
    let queue_wait_seconds = (start_time - queued_time).max(0);
    info!(
        telemetry_event = "scan_queue_wait_seconds",
        scan_id,
        queue_wait_seconds,
        "scan telemetry event"
    );
}

fn status_label(status: ScanStatus) -> &'static str {
    match status {
        ScanStatus::Stored => "stored",
        ScanStatus::Requested => "requested",
        ScanStatus::Running => "running",
        ScanStatus::Stopped => "stopped",
        ScanStatus::Failed => "failed",
        ScanStatus::Succeeded => "succeeded",
    }
}
