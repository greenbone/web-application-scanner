// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! FIFO queue used by the scan runtime workers.

use std::collections::VecDeque;

use tokio::sync::{Mutex, Notify};

#[derive(Debug, Default)]
pub struct ScanQueue {
    entries: Mutex<VecDeque<String>>,
    notify: Notify,
}

impl ScanQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn enqueue(&self, scan_id: String) {
        let mut entries = self.entries.lock().await;
        entries.push_back(scan_id);
        drop(entries);
        self.notify.notify_one();
    }

    pub async fn dequeue(&self) -> String {
        loop {
            let notified = self.notify.notified();
            if let Some(scan_id) = self.try_dequeue().await {
                return scan_id;
            }
            notified.await;
        }
    }

    pub async fn remove(&self, scan_id: &str) -> bool {
        let mut entries = self.entries.lock().await;
        if let Some(index) = entries.iter().position(|queued_id| queued_id == scan_id) {
            entries.remove(index);
            return true;
        }

        false
    }

    async fn try_dequeue(&self) -> Option<String> {
        let mut entries = self.entries.lock().await;
        entries.pop_front()
    }
}

#[cfg(test)]
#[path = "queue_tests.rs"]
mod queue_tests;
