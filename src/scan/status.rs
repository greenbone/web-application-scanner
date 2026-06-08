// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Canonical scan lifecycle status model.

use serde::{Deserialize, Serialize};

/// Lifecycle phase of a scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    New,
    Queued,
    Running,
    StopRequested,
    Stopped,
    Interrupted,
    Done,
}

impl ScanStatus {
    /// Resolve next status for a start command.
    pub fn start_command_transition(self) -> Option<Self> {
        match self {
            Self::New => Some(Self::Queued),
            _ => None,
        }
    }

    /// Resolve next status for a stop command.
    pub fn stop_command_transition(self) -> Option<Self> {
        match self {
            Self::Queued => Some(Self::Stopped),
            Self::Running => Some(Self::StopRequested),
            _ => None,
        }
    }

    /// Return whether a scan can be deleted in this status.
    pub fn can_delete(self) -> bool {
        matches!(
            self,
            Self::New | Self::Stopped | Self::Interrupted | Self::Done
        )
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
