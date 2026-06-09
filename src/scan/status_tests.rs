// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::ScanStatus;

#[test]
fn start_transition_is_only_allowed_from_new() {
    assert_eq!(
        ScanStatus::New.start_command_transition(),
        Some(ScanStatus::Queued)
    );
    assert_eq!(ScanStatus::Queued.start_command_transition(), None);
    assert_eq!(ScanStatus::Done.start_command_transition(), None);
}

#[test]
fn stop_transition_is_only_allowed_from_queued_or_running() {
    assert_eq!(
        ScanStatus::Queued.stop_command_transition(),
        Some(ScanStatus::Stopped)
    );
    assert_eq!(
        ScanStatus::Running.stop_command_transition(),
        Some(ScanStatus::StopRequested)
    );
    assert_eq!(ScanStatus::New.stop_command_transition(), None);
}

#[test]
fn delete_is_only_allowed_for_new_or_terminal_states() {
    assert!(ScanStatus::New.can_delete());
    assert!(ScanStatus::Stopped.can_delete());
    assert!(ScanStatus::Interrupted.can_delete());
    assert!(ScanStatus::Done.can_delete());
    assert!(!ScanStatus::Queued.can_delete());
    assert!(!ScanStatus::Running.can_delete());
    assert!(!ScanStatus::StopRequested.can_delete());
}
