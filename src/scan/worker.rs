// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Background scan worker runtime.

use std::sync::Arc;
use std::time::Duration;

use regex::escape;
use tokio::{
    task::JoinHandle,
    time::{Instant, sleep},
};
use tracing::{debug, error, warn};

use crate::{
    api::dto::scans::ResultType,
    scan::{
        RetryingScanStateCoordinator, Scan, ScanProgress, ScanResult, ScanStateCoordinator,
        ScanStatus, observability::emit_queue_wait_telemetry, queue::ScanQueue, retry::IsTransient,
    },
    storage::{StorageError, StorageHandle},
    zapclient::ajaxspider::AjaxSpiderStatus,
    zapclient::alert::{Alert, AlertRiskLevel},
    zapclient::{RetryingZapClient, ZapClient, ZapClientError},
};

const DEFAULT_SCAN_POLL_INTERVAL: Duration = Duration::from_millis(50);
const DEFAULT_ALERT_PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone)]
pub struct ScanRuntimeConfig {
    pub worker_count: usize,
    pub alert_poll_interval: Duration,
    pub scan_poll_interval: Duration,
    pub alert_page_size: u32,
    pub stop_grace_period: Duration,
    /// Maximum number of retry attempts for transient failures before a scan transitions to `failed`.
    pub retry_max_retries: u32,
    /// Maximum backoff delay between retry attempts.
    pub retry_max_delay: Duration,
}

impl Default for ScanRuntimeConfig {
    fn default() -> Self {
        Self {
            worker_count: 1,
            alert_poll_interval: Duration::from_secs(10),
            scan_poll_interval: DEFAULT_SCAN_POLL_INTERVAL,
            alert_page_size: DEFAULT_ALERT_PAGE_SIZE,
            stop_grace_period: Duration::from_secs(300),
            retry_max_retries: 10,
            retry_max_delay: Duration::from_secs(60),
        }
    }
}

#[derive(Clone)]
pub struct ScanRuntimeHandle {
    queue: Arc<ScanQueue>,
    storage: StorageHandle,
    scan_state: ScanStateCoordinator,
    stop_grace_period: Duration,
}

impl ScanRuntimeHandle {
    pub async fn enqueue(&self, scan_id: String) {
        self.queue.enqueue(scan_id).await;
    }

    pub async fn remove_queued(&self, scan_id: &str) -> bool {
        self.queue.remove(scan_id).await
    }

    pub async fn request_stop(&self, scan_id: String) {
        let storage = self.storage.clone();
        let scan_state = self.scan_state.clone();
        let stop_grace_period = self.stop_grace_period;
        let handle: JoinHandle<()> = tokio::spawn(async move {
            sleep(stop_grace_period).await;

            let scan: Scan = match storage.get_scan(&scan_id).await {
                Ok(scan_record) => scan_record.into(),
                Err(error) => {
                    warn!(scan_id, error = %error, "failed to load scan while enforcing stop grace period");
                    return;
                }
            };

            if scan.status == ScanStatus::Running && scan.stop_requested {
                warn!(
                    scan_id,
                    "scan stop grace period exceeded; transitioning scan to failed"
                );
                if let Err(error) = scan_state
                    .overwrite_status(&scan_id, ScanStatus::Running, ScanStatus::Failed)
                    .await
                {
                    warn!(scan_id, error = %error, "failed to transition timed-out stop request to failed");
                }
            }
        });
        std::mem::drop(handle);
    }
}

pub fn start_scan_runtime(
    storage: StorageHandle,
    zap_client: ZapClient,
    config: ScanRuntimeConfig,
) -> ScanRuntimeHandle {
    let queue = Arc::new(ScanQueue::new());
    let worker_count = config.worker_count.max(1);
    let retrying_zap = RetryingZapClient::new(
        zap_client.clone(),
        config.retry_max_retries,
        config.retry_max_delay,
    );
    let retrying_state = RetryingScanStateCoordinator::new(
        ScanStateCoordinator::new(storage.clone()),
        config.retry_max_retries,
        config.retry_max_delay,
    );

    for worker_index in 0..worker_count {
        let worker = ScanWorker {
            worker_index,
            storage: storage.clone(),
            scan_state: retrying_state.clone(),
            zap_client: retrying_zap.clone(),
            queue: queue.clone(),
            config: config.clone(),
        };
        let handle: JoinHandle<()> = tokio::spawn(async move {
            worker.run().await;
        });
        std::mem::drop(handle);
    }

    ScanRuntimeHandle {
        queue,
        storage: storage.clone(),
        scan_state: ScanStateCoordinator::new(storage),
        stop_grace_period: config.stop_grace_period,
    }
}

enum RunningStage<'a> {
    Spider,
    ActiveScan { active_scan_id: &'a str },
}

struct ScanWorker {
    worker_index: usize,
    storage: StorageHandle,
    scan_state: RetryingScanStateCoordinator,
    zap_client: RetryingZapClient,
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
                    "scan worker failed scan execution"
                );
            }
        }
    }

    async fn process_scan(&self, scan_id: &str) -> Result<(), WorkerError> {
        let claim_result = self
            .scan_state
            .transition_status(scan_id, ScanStatus::Requested, ScanStatus::Running)
            .await;

        match claim_result {
            Ok(()) => {}
            Err(StorageError::InvalidState) | Err(StorageError::NotFound(_)) => {
                debug!(
                    worker = self.worker_index,
                    scan_id, "skipping requested scan that was already handled or removed"
                );
                return Ok(());
            }
            Err(error) => return Err(WorkerError::Storage(error)),
        }

        let scan: Scan = self.storage.get_scan(scan_id).await?.into();
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

    async fn execute_scan(&self, scan: &Scan) -> Result<(), WorkerError> {
        let mut progress = ScanProgress::new(&scan.target.hosts);

        {
            let pv = progress.as_value();
            self.scan_state.update_progress(&scan.id, Some(pv)).await?;
        }

        let (context_name, context_id) = self.ensure_context(scan).await?;

        if self.stop_requested(&scan.id).await? {
            self.complete_stop_request(&scan.id, Some(&context_name))
                .await?;
            return Ok(());
        }

        for (index, target) in scan.target.hosts.iter().enumerate() {
            if self.stop_requested(&scan.id).await? {
                self.complete_stop_request(&scan.id, Some(&context_name))
                    .await?;
                return Ok(());
            }

            progress.mark_spider_running(index);
            {
                let pv = progress.as_value();
                self.scan_state.update_progress(&scan.id, Some(pv)).await?;
            }

            self.zap_client
                .start_ajax_spider_scan(&context_name, target, true, false)
                .await?;

            loop {
                if self.stop_requested(&scan.id).await? {
                    self.handle_stop_request(&scan.id, Some(&context_name), RunningStage::Spider)
                        .await?;
                    return Ok(());
                }

                let status = self.zap_client.get_ajax_spider_status().await?;
                match status {
                    AjaxSpiderStatus::Running => sleep(self.config.scan_poll_interval).await,
                    AjaxSpiderStatus::Stopped => break,
                }
            }

            progress.mark_spider_done(index);
            progress.mark_active_scan_running(index);
            {
                let pv = progress.as_value();
                self.scan_state.update_progress(&scan.id, Some(pv)).await?;
            }

            let active_scan_id = self
                .zap_client
                .start_active_scan(&context_id, target, true, true)
                .await?;
            let mut last_alert_poll = Instant::now() - self.config.alert_poll_interval;

            loop {
                if self.stop_requested(&scan.id).await? {
                    self.handle_stop_request(
                        &scan.id,
                        Some(&context_name),
                        RunningStage::ActiveScan {
                            active_scan_id: &active_scan_id,
                        },
                    )
                    .await?;
                    return Ok(());
                }

                if last_alert_poll.elapsed() >= self.config.alert_poll_interval {
                    self.poll_and_persist_alerts(&scan.id, &context_name, Some(target.as_str()))
                        .await?;
                    last_alert_poll = Instant::now();
                }

                let active_percentage = self
                    .zap_client
                    .get_active_scan_status(&active_scan_id)
                    .await?;
                progress.update_active_scan(index, active_percentage);
                {
                    let pv = progress.as_value();
                    self.scan_state.update_progress(&scan.id, Some(pv)).await?;
                }

                if active_percentage >= 100 {
                    break;
                }

                sleep(self.config.scan_poll_interval).await;
            }

            progress.mark_active_scan_done(index);
            {
                let pv = progress.as_value();
                self.scan_state.update_progress(&scan.id, Some(pv)).await?;
            }
            self.poll_and_persist_alerts(&scan.id, &context_name, Some(target.as_str()))
                .await?;
        }

        self.poll_and_persist_alerts(
            &scan.id,
            &context_name,
            scan.target.hosts.last().map(String::as_str),
        )
        .await?;

        if self.stop_requested(&scan.id).await? {
            self.complete_stop_request(&scan.id, Some(&context_name))
                .await?;
            return Ok(());
        }

        if let Err(error) = self.zap_client.remove_context(&context_name).await {
            warn!(scan_id = scan.id, error = %error, "failed to remove ZAP context after successful scan completion");
        }

        self.scan_state
            .overwrite_status(&scan.id, ScanStatus::Running, ScanStatus::Succeeded)
            .await?;

        Ok(())
    }

    async fn stop_requested(&self, scan_id: &str) -> Result<bool, WorkerError> {
        let scan: Scan = self.storage.get_scan(scan_id).await?.into();
        Ok(scan.stop_requested)
    }

    async fn complete_stop_request(
        &self,
        scan_id: &str,
        context_name: Option<&str>,
    ) -> Result<(), WorkerError> {
        if let Some(name) = context_name
            && let Err(error) = self.zap_client.remove_context(name).await
        {
            warn!(scan_id, error = %error, "failed to remove ZAP context while stopping scan");
        }

        self.scan_state
            .transition_status(scan_id, ScanStatus::Running, ScanStatus::Stopped)
            .await?;

        Ok(())
    }

    async fn handle_stop_request(
        &self,
        scan_id: &str,
        context_name: Option<&str>,
        running_stage: RunningStage<'_>,
    ) -> Result<(), WorkerError> {
        match running_stage {
            RunningStage::Spider => {
                self.zap_client.stop_ajax_spider_scan().await?;
            }
            RunningStage::ActiveScan { active_scan_id } => {
                self.zap_client.stop_active_scan(active_scan_id).await?;
            }
        }

        self.complete_stop_request(scan_id, context_name).await
    }

    async fn ensure_context(&self, scan: &Scan) -> Result<(String, String), WorkerError> {
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

        {
            let cn = Some(context_name.clone());
            let ci = Some(context_id.clone());
            self.scan_state.update_context(&scan.id, cn, ci).await?;
        }

        Ok((context_name, context_id))
    }

    async fn poll_and_persist_alerts(
        &self,
        scan_id: &str,
        context_name: &str,
        target_url: Option<&str>,
    ) -> Result<(), WorkerError> {
        let page_size = self.config.alert_page_size;
        loop {
            let scan: Scan = self.storage.get_scan(scan_id).await?.into();
            let cursor = scan.alert_cursor.unwrap_or(0);
            let alerts = self
                .zap_client
                .get_alerts(context_name, None, Some(cursor as u32), Some(page_size))
                .await?;

            if alerts.is_empty() {
                break;
            }

            let results: Vec<ScanResult> = alerts
                .iter()
                .map(|alert| alert_to_result(scan_id, alert, target_url))
                .collect();
            let next_cursor = cursor + alerts.len() as i64;

            self.scan_state
                .persist_alert_batch(scan_id, next_cursor, results)
                .await?;

            if alerts.len() < page_size as usize {
                break;
            }
        }

        Ok(())
    }

    async fn interrupt_scan(&self, scan_id: &str) {
        let scan = match self.storage.get_scan(scan_id).await {
            Ok(scan_record) => Scan::from(scan_record),
            Err(error) => {
                error!(scan_id, error = %error, "failed to load scan for interruption handling");
                return;
            }
        };

        if matches!(
            scan.status,
            ScanStatus::Stored | ScanStatus::Stopped | ScanStatus::Failed | ScanStatus::Succeeded
        ) {
            return;
        }

        if let Some(context_name) = scan.context_name.as_deref()
            && let Err(error) = self.zap_client.remove_context(context_name).await
        {
            warn!(scan_id, error = %error, "failed to remove ZAP context while interrupting scan");
        }

        if let Err(error) = self
            .scan_state
            .overwrite_status(scan_id, scan.status, ScanStatus::Failed)
            .await
        {
            warn!(scan_id, error = %error, "failed to transition scan to failed state");
        }
    }
}

fn alert_to_result(scan_id: &str, alert: &Alert, target_url: Option<&str>) -> ScanResult {
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

    ScanResult {
        id: 0,
        scan_id: scan_id.to_string(),
        result_type: match alert.risk {
            AlertRiskLevel::Informational => ResultType::Log,
            _ => ResultType::Alarm,
        },
        ip_address: target_url.map(str::to_string),
        hostname,
        oid: Some(format!("ZAP-{}", alert.alert_ref)),
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

impl IsTransient for WorkerError {
    fn is_transient(&self) -> bool {
        match self {
            WorkerError::Storage(e) => e.is_transient(),
            WorkerError::ZapClient(e) => e.is_transient(),
        }
    }
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod worker_tests;
