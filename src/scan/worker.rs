// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Background scan worker runtime.

use std::{sync::Arc, time::Duration};

use regex::escape;
use tokio::{
    task::JoinHandle,
    time::{Instant, sleep},
};
use tracing::{debug, error, warn};

use crate::{
    api::dto::scans::ResultType,
    scan::{
        ScanProgress, ScanStatus,
        observability::{emit_queue_wait_telemetry, emit_status_transition},
        queue::ScanQueue,
    },
    storage::{ResultRecord, ScanRecord, StorageError, StorageHandle},
    zapclient::{
        ZapClient,
        ZapClientError,
        ajaxspider::AjaxSpiderStatus,
        alert::{Alert, AlertRiskLevel},
    },
};

const DEFAULT_SCAN_POLL_INTERVAL: Duration = Duration::from_millis(50);
const DEFAULT_ALERT_PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone)]
pub struct ScanRuntimeConfig {
    pub worker_count: usize,
    pub alert_poll_interval: Duration,
    pub scan_poll_interval: Duration,
    pub alert_page_size: u32,
}

impl Default for ScanRuntimeConfig {
    fn default() -> Self {
        Self {
            worker_count: 1,
            alert_poll_interval: Duration::from_secs(10),
            scan_poll_interval: DEFAULT_SCAN_POLL_INTERVAL,
            alert_page_size: DEFAULT_ALERT_PAGE_SIZE,
        }
    }
}

#[derive(Clone)]
pub struct ScanRuntimeHandle {
    queue: Arc<ScanQueue>,
}

impl ScanRuntimeHandle {
    pub async fn enqueue(&self, scan_id: String) {
        self.queue.enqueue(scan_id).await;
    }

    pub async fn remove_queued(&self, scan_id: &str) -> bool {
        self.queue.remove(scan_id).await
    }
}

pub fn start_scan_runtime(
    storage: StorageHandle,
    zap_client: ZapClient,
    config: ScanRuntimeConfig,
) -> ScanRuntimeHandle {
    let queue = Arc::new(ScanQueue::new());
    let worker_count = config.worker_count.max(1);

    for worker_index in 0..worker_count {
        let worker = ScanWorker {
            worker_index,
            storage: storage.clone(),
            zap_client: zap_client.clone(),
            queue: queue.clone(),
            config: config.clone(),
        };
        let handle: JoinHandle<()> = tokio::spawn(async move {
            worker.run().await;
        });
        std::mem::drop(handle);
    }

    ScanRuntimeHandle { queue }
}

struct ScanWorker {
    worker_index: usize,
    storage: StorageHandle,
    zap_client: ZapClient,
    queue: Arc<ScanQueue>,
    config: ScanRuntimeConfig,
}

impl ScanWorker {
    async fn run(self) {
        loop {
            let scan_id = self.queue.dequeue().await;
            if let Err(error) = self.process_scan(&scan_id).await {
                error!(
                    worker = self.worker_index,
                    scan_id,
                    error = %error,
                    "scan worker interrupted scan execution"
                );
            }
        }
    }

    async fn process_scan(&self, scan_id: &str) -> Result<(), WorkerError> {
        let claim_result = self
            .storage
            .transition_scan_status(scan_id, ScanStatus::Queued, ScanStatus::Running)
            .await;

        match claim_result {
            Ok(()) => {}
            Err(StorageError::InvalidState) | Err(StorageError::NotFound(_)) => {
                debug!(
                    worker = self.worker_index,
                    scan_id,
                    "skipping queued scan that was already handled or removed"
                );
                return Ok(());
            }
            Err(error) => return Err(WorkerError::Storage(error)),
        }

        let scan = self.storage.get_scan(scan_id).await?;
        emit_status_transition(scan_id, ScanStatus::Queued, ScanStatus::Running);
        if let (Some(queued_time), Some(start_time)) = (scan.queued_time, scan.start_time) {
            emit_queue_wait_telemetry(scan_id, queued_time, start_time);
        }
        match self.execute_scan(&scan).await {
            Ok(()) => Ok(()),
            Err(error) => {
                self.interrupt_scan(scan_id).await;
                Err(error)
            }
        }
    }

    async fn execute_scan(&self, scan: &ScanRecord) -> Result<(), WorkerError> {
        let mut progress = ScanProgress::new(&scan.target.hosts);
        self.storage
            .update_scan_progress(&scan.id, Some(progress.as_value()))
            .await?;

        let (context_name, context_id) = self.ensure_context(scan).await?;

        for (index, target) in scan.target.hosts.iter().enumerate() {
            progress.mark_spider_running(index);
            self.storage
                .update_scan_progress(&scan.id, Some(progress.as_value()))
                .await?;

            self.zap_client
                .start_ajax_spider_scan(&context_name, target, true, false)
                .await?;

            loop {
                match self.zap_client.get_ajax_spider_status().await? {
                    AjaxSpiderStatus::Running => sleep(self.config.scan_poll_interval).await,
                    AjaxSpiderStatus::Stopped => break,
                }
            }

            progress.mark_spider_done(index);
            progress.mark_active_scan_running(index);
            self.storage
                .update_scan_progress(&scan.id, Some(progress.as_value()))
                .await?;

            let active_scan_id = self
                .zap_client
                .start_active_scan(&context_id, target, true, true)
                .await?;
            let mut last_alert_poll = Instant::now() - self.config.alert_poll_interval;

            loop {
                if last_alert_poll.elapsed() >= self.config.alert_poll_interval {
                    self.poll_and_persist_alerts(&scan.id, &context_name).await?;
                    last_alert_poll = Instant::now();
                }

                let active_percentage = self
                    .zap_client
                    .get_active_scan_status(&active_scan_id)
                    .await?;
                progress.update_active_scan(index, active_percentage);
                self.storage
                    .update_scan_progress(&scan.id, Some(progress.as_value()))
                    .await?;

                if active_percentage >= 100 {
                    break;
                }

                sleep(self.config.scan_poll_interval).await;
            }

            progress.mark_active_scan_done(index);
            self.storage
                .update_scan_progress(&scan.id, Some(progress.as_value()))
                .await?;
            self.poll_and_persist_alerts(&scan.id, &context_name).await?;
        }

        self.poll_and_persist_alerts(&scan.id, &context_name).await?;
        if let Err(error) = self.zap_client.remove_context(&context_name).await {
            warn!(scan_id = scan.id, error = %error, "failed to remove ZAP context after successful scan completion");
        }

        self.storage.update_scan_status(&scan.id, ScanStatus::Done).await?;
        emit_status_transition(&scan.id, ScanStatus::Running, ScanStatus::Done);
        Ok(())
    }

    async fn ensure_context(&self, scan: &ScanRecord) -> Result<(String, String), WorkerError> {
        if let (Some(context_name), Some(context_id)) =
            (scan.context_name.clone(), scan.context_id.clone())
        {
            return Ok((context_name, context_id));
        }

        let context_name = format!("greenbone-was-{}", scan.id);
        let context_id = self.zap_client.new_context(&context_name).await?;

        for target in &scan.target.hosts {
            let regex = format!("^{}.*$", escape(target));
            self.zap_client
                .include_in_context(&context_name, &regex)
                .await?;
        }

        self.storage
            .update_scan_context(&scan.id, Some(context_name.clone()), Some(context_id.clone()))
            .await?;

        Ok((context_name, context_id))
    }

    async fn poll_and_persist_alerts(
        &self,
        scan_id: &str,
        context_name: &str,
    ) -> Result<(), WorkerError> {
        loop {
            let scan = self.storage.get_scan(scan_id).await?;
            let cursor = scan.alert_cursor.unwrap_or(0);
            let alerts = self
                .zap_client
                .get_alerts(
                    context_name,
                    None,
                    Some(cursor as u32),
                    Some(self.config.alert_page_size),
                )
                .await?;

            if alerts.is_empty() {
                break;
            }

            let results = alerts
                .iter()
                .map(|alert| alert_to_result(scan_id, alert))
                .collect();

            self.storage.add_results(scan_id, results).await?;
            self.storage
                .update_alert_cursor(scan_id, Some(cursor + alerts.len() as i64))
                .await?;

            if alerts.len() < self.config.alert_page_size as usize {
                break;
            }
        }

        Ok(())
    }

    async fn interrupt_scan(&self, scan_id: &str) {
        let scan = match self.storage.get_scan(scan_id).await {
            Ok(scan) => scan,
            Err(error) => {
                error!(scan_id, error = %error, "failed to load scan for interruption handling");
                return;
            }
        };

        if matches!(
            scan.status,
            ScanStatus::New | ScanStatus::Stopped | ScanStatus::Interrupted | ScanStatus::Done
        ) {
            return;
        }

        if let Some(context_name) = scan.context_name.as_deref() {
            if let Err(error) = self.zap_client.remove_context(context_name).await {
                warn!(scan_id, error = %error, "failed to remove ZAP context while interrupting scan");
            }
        }

        if let Err(error) = self
            .storage
            .update_scan_status(scan_id, ScanStatus::Interrupted)
            .await
        {
            warn!(scan_id, error = %error, "failed to transition scan to interrupted state");
        } else {
            emit_status_transition(scan_id, scan.status, ScanStatus::Interrupted);
        }
    }
}

fn alert_to_result(scan_id: &str, alert: &Alert) -> ResultRecord {
    let parsed_url = reqwest::Url::parse(&alert.url).ok();
    let hostname = parsed_url
        .as_ref()
        .and_then(|url| url.host_str().map(str::to_string));
    let port = parsed_url
        .as_ref()
        .and_then(|url| url.port_or_known_default())
        .map(i32::from);
    let protocol = parsed_url.as_ref().and_then(|url| match url.scheme() {
        "http" | "https" => Some("tcp".to_string()),
        _ => None,
    });

    let mut message = format!(
        "{} ({}) at {}",
        alert.name,
        risk_label(&alert.risk),
        alert.url
    );
    if !alert.description.trim().is_empty() {
        message.push('\n');
        message.push_str(&alert.description);
    }

    ResultRecord {
        id: 0,
        scan_id: scan_id.to_string(),
        result_type: match alert.risk {
            AlertRiskLevel::Informational => ResultType::Log,
            _ => ResultType::Alarm,
        },
        ip_address: Some(alert.url.clone()),
        hostname,
        oid: Some(alert.plugin_id.clone()),
        port,
        protocol,
        message: Some(message),
        detail: None,
    }
}

fn risk_label(risk: &AlertRiskLevel) -> &'static str {
    match risk {
        AlertRiskLevel::Informational => "Informational",
        AlertRiskLevel::Low => "Low",
        AlertRiskLevel::Medium => "Medium",
        AlertRiskLevel::High => "High",
        AlertRiskLevel::Unknown => "Unknown",
    }
}

#[derive(Debug, thiserror::Error)]
enum WorkerError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    ZapClient(#[from] ZapClientError),
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod worker_tests;