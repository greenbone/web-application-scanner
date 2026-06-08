// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan domain primitives and lifecycle transitions.

pub mod errors;
pub mod service;
pub mod status;

pub use errors::ScanServiceError;
pub use service::{CreateScanRequest, DefaultScanService, ScanService, ScanServiceHandle};
pub use status::ScanStatus;
