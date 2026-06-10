// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::ScanStatus;

#[test]
fn start_transition_is_only_allowed_from_stored() {
    assert_eq!(
        ScanStatus::Stored.start_command_transition(),
        Some(ScanStatus::Requested)
    );
    assert_eq!(ScanStatus::Requested.start_command_transition(), None);
    assert_eq!(ScanStatus::Succeeded.start_command_transition(), None);
}

#[test]
fn stop_transition_is_only_allowed_from_requested_or_running() {
    assert_eq!(
        ScanStatus::Requested.stop_command_transition(),
        Some(ScanStatus::Stopped)
    );
    assert_eq!(ScanStatus::Running.stop_command_transition(), None);
    assert_eq!(ScanStatus::Stored.stop_command_transition(), None);
}

#[test]
fn delete_is_only_allowed_for_stored_or_terminal_states() {
    assert!(ScanStatus::Stored.can_delete());
    assert!(ScanStatus::Stopped.can_delete());
    assert!(ScanStatus::Failed.can_delete());
    assert!(ScanStatus::Succeeded.can_delete());
    assert!(!ScanStatus::Requested.can_delete());
    assert!(!ScanStatus::Running.can_delete());
}

#[test]
fn serde_uses_stored_for_stored_status() {
    let encoded = serde_json::to_string(&ScanStatus::Stored).expect("status should serialize");
    assert_eq!(encoded, "\"stored\"");
}

#[test]
fn serde_uses_requested_for_requested_status() {
    let encoded = serde_json::to_string(&ScanStatus::Requested).expect("status should serialize");
    assert_eq!(encoded, "\"requested\"");
}
