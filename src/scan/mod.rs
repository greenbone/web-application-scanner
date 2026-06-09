// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan domain primitives and lifecycle transitions.

pub mod errors;
pub mod progress;
pub mod queue;
pub mod service;
pub mod status;
pub mod worker;

pub use errors::ScanServiceError;
pub use progress::ScanProgress;
pub use service::{CreateScanRequest, DefaultScanService, ScanService, ScanServiceHandle};
pub use status::ScanStatus;
pub use worker::{ScanRuntimeConfig, ScanRuntimeHandle, start_scan_runtime};
