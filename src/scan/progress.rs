// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Persisted scan progress payload.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    Pending,
    Running,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetProgress {
    pub target: String,
    pub spider_state: StageState,
    pub spider_last_status: Option<String>,
    pub active_scan_state: StageState,
    pub active_scan_percentage: i32,
    pub overall_percentage: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub overall_percentage: i32,
    pub targets: Vec<TargetProgress>,
}

impl ScanProgress {
    pub fn new(targets: &[String]) -> Self {
        let targets = targets
            .iter()
            .cloned()
            .map(|target| TargetProgress {
                target,
                spider_state: StageState::Pending,
                spider_last_status: None,
                active_scan_state: StageState::Pending,
                active_scan_percentage: 0,
                overall_percentage: 0,
            })
            .collect();

        Self {
            overall_percentage: 0,
            targets,
        }
    }

    pub fn mark_spider_running(&mut self, index: usize) {
        let target = &mut self.targets[index];
        target.spider_state = StageState::Running;
        target.spider_last_status = Some("running".to_string());
        self.refresh();
    }

    pub fn mark_spider_done(&mut self, index: usize) {
        let target = &mut self.targets[index];
        target.spider_state = StageState::Done;
        target.spider_last_status = Some("stopped".to_string());
        self.refresh();
    }

    pub fn mark_active_scan_running(&mut self, index: usize) {
        let target = &mut self.targets[index];
        target.active_scan_state = StageState::Running;
        self.refresh();
    }

    pub fn update_active_scan(&mut self, index: usize, percentage: i32) {
        let target = &mut self.targets[index];
        target.active_scan_percentage = percentage.clamp(0, 100);
        if target.active_scan_percentage >= 100 {
            target.active_scan_state = StageState::Done;
        }
        self.refresh();
    }

    pub fn mark_active_scan_done(&mut self, index: usize) {
        let target = &mut self.targets[index];
        target.active_scan_state = StageState::Done;
        target.active_scan_percentage = 100;
        self.refresh();
    }

    pub fn as_value(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("scan progress should serialize")
    }

    fn refresh(&mut self) {
        for target in &mut self.targets {
            let spider_done = matches!(target.spider_state, StageState::Done);
            let spider_running = matches!(target.spider_state, StageState::Running);
            let active_pct = target.active_scan_percentage.clamp(0, 100) as f64;
            target.overall_percentage = if spider_done {
                (25.0 + (0.75 * active_pct)).floor() as i32
            } else if spider_running {
                1
            } else {
                0
            };
        }

        self.overall_percentage = if self.targets.is_empty() {
            0
        } else {
            self.targets
                .iter()
                .map(|target| target.overall_percentage)
                .sum::<i32>()
                / self.targets.len() as i32
        };
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod progress_tests;
