// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    api::dto::scans::{HostInfo, HostScanningEntry},
    scan::ScanProgress,
};

use super::progress_to_host_info;

// ─── helpers ─────────────────────────────────────────────────────────────────

fn make_progress(hosts: &[&str]) -> ScanProgress {
    let hosts: Vec<String> = hosts.iter().map(|s| s.to_string()).collect();
    ScanProgress::new(&hosts)
}

// ─── all targets pending ──────────────────────────────────────────────────────

#[test]
fn all_pending_targets_are_queued() {
    let progress = make_progress(&["http://a.example", "http://b.example"]);
    let info = progress_to_host_info(&progress);

    assert_eq!(info.all, 2);
    assert_eq!(info.queued, 2);
    assert_eq!(info.finished, 0);
    assert!(info.scanning.is_empty());
}

#[test]
fn alive_equals_all() {
    let progress = make_progress(&["http://a.example", "http://b.example"]);
    let info = progress_to_host_info(&progress);

    assert_eq!(info.alive, info.all);
}

#[test]
fn excluded_and_dead_are_zero() {
    let progress = make_progress(&["http://a.example"]);
    let info = progress_to_host_info(&progress);

    assert_eq!(info.excluded, 0);
    assert_eq!(info.dead, 0);
}

// ─── spider running ───────────────────────────────────────────────────────────

#[test]
fn spider_running_target_appears_in_scanning_with_progress_1() {
    let mut progress = make_progress(&["http://a.example"]);
    progress.mark_spider_running(0);

    let info = progress_to_host_info(&progress);

    assert_eq!(info.queued, 0);
    assert_eq!(info.finished, 0);
    assert_eq!(
        info.scanning,
        vec![HostScanningEntry {
            host: "http://a.example".to_string(),
            progress: 1,
        }]
    );
}

// ─── active scan running ──────────────────────────────────────────────────────

#[test]
fn active_scan_running_target_appears_in_scanning_with_formula_progress() {
    let mut progress = make_progress(&["http://a.example"]);
    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_running(0);
    progress.update_active_scan(0, 50);

    let info = progress_to_host_info(&progress);

    // floor(25 + 0.75 * 50) = 62
    assert_eq!(info.scanning.len(), 1);
    assert_eq!(info.scanning[0].host, "http://a.example");
    assert_eq!(info.scanning[0].progress, 62);
    assert_eq!(info.queued, 0);
    assert_eq!(info.finished, 0);
}

// ─── active scan done ─────────────────────────────────────────────────────────

#[test]
fn active_scan_done_target_counted_as_finished() {
    let mut progress = make_progress(&["http://a.example"]);
    progress.mark_spider_running(0);
    progress.mark_spider_done(0);
    progress.mark_active_scan_done(0);

    let info = progress_to_host_info(&progress);

    assert_eq!(info.finished, 1);
    assert_eq!(info.queued, 0);
    assert!(info.scanning.is_empty());
}

// ─── mixed targets ────────────────────────────────────────────────────────────

#[test]
fn mixed_targets_populate_queued_scanning_and_finished_correctly() {
    let mut progress = make_progress(&[
        "http://pending.example",
        "http://spider.example",
        "http://active.example",
        "http://done.example",
    ]);

    // index 0: stays pending (queued)
    // index 1: spider running (scanning, progress = 1)
    progress.mark_spider_running(1);
    // index 2: spider done, active scan at 40% (scanning, progress = floor(25 + 30) = 55)
    progress.mark_spider_running(2);
    progress.mark_spider_done(2);
    progress.mark_active_scan_running(2);
    progress.update_active_scan(2, 40);
    // index 3: fully finished
    progress.mark_spider_running(3);
    progress.mark_spider_done(3);
    progress.mark_active_scan_done(3);

    let info = progress_to_host_info(&progress);

    assert_eq!(info.all, 4);
    assert_eq!(info.queued, 1);
    assert_eq!(info.finished, 1);
    assert_eq!(info.scanning.len(), 2);
    assert_eq!(info.alive, 4);

    let spider_entry = info
        .scanning
        .iter()
        .find(|e| e.host == "http://spider.example")
        .expect("spider.example should be in scanning");
    assert_eq!(spider_entry.progress, 1);

    let active_entry = info
        .scanning
        .iter()
        .find(|e| e.host == "http://active.example")
        .expect("active.example should be in scanning");
    assert_eq!(active_entry.progress, 55);
}

// ─── empty target list ────────────────────────────────────────────────────────

#[test]
fn empty_progress_produces_zeroed_host_info() {
    let progress = make_progress(&[]);
    let info = progress_to_host_info(&progress);

    assert_eq!(
        info,
        HostInfo {
            all: 0,
            excluded: 0,
            dead: 0,
            alive: 0,
            queued: 0,
            finished: 0,
            scanning: vec![],
        }
    );
}
