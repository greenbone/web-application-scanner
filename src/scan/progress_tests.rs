// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{ScanProgress, StageState};

// ─── new ─────────────────────────────────────────────────────────────────────

#[test]
fn new_creates_pending_targets_with_zero_progress() {
    let hosts = vec![
        "http://a.example".to_string(),
        "http://b.example".to_string(),
    ];
    let progress = ScanProgress::new(&hosts);

    assert_eq!(progress.targets.len(), 2);
    assert_eq!(progress.overall_percentage, 0);
    for target in &progress.targets {
        assert_eq!(target.spider_state, StageState::Pending);
        assert_eq!(target.spider_last_status, None);
        assert_eq!(target.active_scan_state, StageState::Pending);
        assert_eq!(target.active_scan_percentage, 0);
        assert_eq!(target.passive_scan_state, StageState::Pending);
        assert_eq!(target.passive_scan_percentage, 0);
        assert_eq!(target.passive_scan_initial_records, None);
        assert_eq!(target.passive_scan_current_records, None);
        assert_eq!(target.overall_percentage, 0);
    }
}

#[test]
fn new_with_no_targets_yields_zero_overall_percentage() {
    let progress = ScanProgress::new(&[]);
    assert_eq!(progress.overall_percentage, 0);
    assert!(progress.targets.is_empty());
}

// ─── per-host progress percentage ────────────────────────────────────────────

#[test]
fn overall_percentage_is_zero_when_spider_is_pending() {
    let hosts = vec!["http://a.example".to_string()];
    let progress = ScanProgress::new(&hosts);

    assert_eq!(progress.targets[0].overall_percentage, 0);
}

#[test]
fn overall_percentage_is_one_when_spider_is_running() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);

    assert_eq!(progress.targets[0].overall_percentage, 1);
}

#[test]
fn overall_percentage_is_25_when_spider_done_and_active_scan_at_zero() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);

    assert_eq!(progress.targets[0].overall_percentage, 25);
}

#[test]
fn overall_percentage_applies_formula_for_active_scan_at_50_percent() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_running(0);
    progress.update_active_scan(0, 50);

    // floor(25 + 0.70 * 50 + 0.05 * 0) = 60
    assert_eq!(progress.targets[0].overall_percentage, 60);
}

#[test]
fn overall_percentage_is_95_when_active_scan_done_and_passive_scan_is_pending() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);

    // floor(25 + 0.70 * 100 + 0.05 * 0) = 95
    assert_eq!(progress.targets[0].overall_percentage, 95);
}

#[test]
fn overall_percentage_is_100_when_active_and_passive_scan_done() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);
    progress.mark_passive_scan_done(0);

    // floor(25 + 0.70 * 100 + 0.05 * 100) = 100
    assert_eq!(progress.targets[0].overall_percentage, 100);
}

// ─── mark_spider_running ─────────────────────────────────────────────────────

#[test]
fn mark_spider_running_sets_state_and_last_status() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);

    assert_eq!(progress.targets[0].spider_state, StageState::Running);
    assert_eq!(
        progress.targets[0].spider_last_status.as_deref(),
        Some("running")
    );
}

// ─── mark_spider_done ────────────────────────────────────────────────────────

#[test]
fn mark_spider_done_sets_state_and_last_status_to_stopped() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);

    assert_eq!(progress.targets[0].spider_state, StageState::Done);
    assert_eq!(
        progress.targets[0].spider_last_status.as_deref(),
        Some("stopped")
    );
}

// ─── mark_active_scan_running ─────────────────────────────────────────────────

#[test]
fn mark_active_scan_running_sets_state_to_running() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_running(0);

    assert_eq!(progress.targets[0].active_scan_state, StageState::Running);
}

// ─── update_active_scan ──────────────────────────────────────────────────────

#[test]
fn update_active_scan_clamps_negative_percentage_to_zero() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.update_active_scan(0, -10);

    assert_eq!(progress.targets[0].active_scan_percentage, 0);
}

#[test]
fn update_active_scan_clamps_percentage_above_100() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.update_active_scan(0, 150);

    assert_eq!(progress.targets[0].active_scan_percentage, 100);
}

#[test]
fn update_active_scan_at_100_transitions_state_to_done() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.update_active_scan(0, 100);

    assert_eq!(progress.targets[0].active_scan_state, StageState::Done);
}

// ─── mark_active_scan_done ───────────────────────────────────────────────────

#[test]
fn mark_active_scan_done_sets_percentage_to_100() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);

    assert_eq!(progress.targets[0].active_scan_percentage, 100);
    assert_eq!(progress.targets[0].active_scan_state, StageState::Done);
}

// ─── mark_passive_scan_running ───────────────────────────────────────────────

#[test]
fn mark_passive_scan_running_sets_state_to_running() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);
    progress.mark_passive_scan_running(0);

    assert_eq!(progress.targets[0].passive_scan_state, StageState::Running);
}

// ─── update_passive_scan ─────────────────────────────────────────────────────

#[test]
fn update_passive_scan_clamps_negative_percentage_to_zero() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);
    progress.update_passive_scan(0, -10);

    assert_eq!(progress.targets[0].passive_scan_percentage, 0);
}

#[test]
fn update_passive_scan_clamps_percentage_above_100() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);
    progress.update_passive_scan(0, 150);

    assert_eq!(progress.targets[0].passive_scan_percentage, 100);
}

#[test]
fn update_passive_scan_at_100_transitions_state_to_done() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);
    progress.update_passive_scan(0, 100);

    assert_eq!(progress.targets[0].passive_scan_state, StageState::Done);
}

// ─── mark_passive_scan_done ──────────────────────────────────────────────────

#[test]
fn mark_passive_scan_done_sets_percentage_to_100() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);
    progress.mark_passive_scan_done(0);

    assert_eq!(progress.targets[0].passive_scan_percentage, 100);
    assert_eq!(progress.targets[0].passive_scan_state, StageState::Done);
}

#[test]
fn set_passive_scan_records_updates_initial_and_current_values() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);

    progress.set_passive_scan_records(0, 123, 45);

    assert_eq!(progress.targets[0].passive_scan_initial_records, Some(123));
    assert_eq!(progress.targets[0].passive_scan_current_records, Some(45));
}

// ─── overall_percentage (multi-target) ───────────────────────────────────────

#[test]
fn overall_percentage_is_average_of_target_percentages() {
    let hosts = vec![
        "http://a.example".to_string(),
        "http://b.example".to_string(),
    ];
    let mut progress = ScanProgress::new(&hosts);

    // Target 0: spider running → overall = 1
    progress.mark_spider_running(0);
    // Target 1: still pending → overall = 0

    // overall = (1 + 0) / 2 = 0 (integer division)
    assert_eq!(progress.overall_percentage, 0);

    // Target 0: spider done + active 100% + passive 100% → overall = 100
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);
    progress.mark_passive_scan_done(0);
    // overall = (100 + 0) / 2 = 50
    assert_eq!(progress.overall_percentage, 50);
}

// ─── as_value / round-trip ────────────────────────────────────────────────────

#[test]
fn as_value_serializes_to_json_and_deserializes_back() {
    let hosts = vec!["http://a.example".to_string()];
    let mut progress = ScanProgress::new(&hosts);
    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.update_active_scan(0, 50);

    let value = progress.as_value();
    let restored: ScanProgress =
        serde_json::from_value(value).expect("progress should deserialize");

    assert_eq!(restored.targets[0].spider_state, StageState::Done);
    assert_eq!(restored.targets[0].active_scan_percentage, 50);
    assert_eq!(restored.targets[0].passive_scan_percentage, 0);
    assert_eq!(restored.targets[0].passive_scan_initial_records, None);
    assert_eq!(restored.targets[0].passive_scan_current_records, None);
    assert_eq!(restored.targets[0].overall_percentage, 60);
    assert_eq!(restored.overall_percentage, 60);
}
