// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan domain primitives and lifecycle transitions.

pub mod errors;
mod model;
mod observability;
pub mod preferences;
pub mod progress;
pub mod queue;
pub mod retry;
pub mod service;
mod state_coordinator;
pub mod status;
pub mod validation;
pub mod worker;

pub use errors::ScanServiceError;
pub use model::{Scan, ScanResult, ScanStatusView};
pub use progress::ScanProgress;
pub use service::{CreateScanRequest, DefaultScanService, ScanService, ScanServiceHandle};
pub use state_coordinator::{RetryingScanStateCoordinator, ScanStateCoordinator};
pub use status::ScanStatus;
pub use worker::{ScanRuntimeConfig, ScanRuntimeHandle, start_scan_runtime};
