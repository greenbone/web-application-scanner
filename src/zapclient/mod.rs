// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! HTTP client for communicating with the ZAP API.

pub mod ajaxspider;
pub mod alert;
pub mod ascan;
pub mod context;

use crate::config::settings::Settings;
use reqwest::StatusCode;
use thiserror::Error;

/// Errors returned by [`ZapClient`] operations.
#[derive(Debug, Error)]
pub enum ZapClientError {
    /// A required setting was missing.
    #[error("missing required ZAP setting: {0}")]
    MissingSetting(&'static str),

    /// Base URL could not be parsed as a valid URL.
    #[error("invalid ZAP base URL '{0}'")]
    InvalidBaseUrl(String),

    /// Request transport failed.
    #[error("failed to call ZAP API: {0}")]
    Request(#[from] reqwest::Error),

    /// ZAP API returned an unsuccessful status code.
    #[error("ZAP API returned HTTP {status}: {body}")]
    UnexpectedStatus { status: StatusCode, body: String },

    /// ZAP API response could not be parsed.
    #[error("failed to parse ZAP API response: {0}")]
    ParseResponse(#[from] serde_json::Error),

    /// ZAP API returned unexpected content in the response body.
    #[error("ZAP API returned unexpected content in field {field}: {content}")]
    UnexpectedContent { field: String, content: String },
}

/// Lightweight client for issuing ZAP API requests.
#[derive(Debug, Clone)]
pub struct ZapClient {
    base_url: reqwest::Url,
    api_key: String,
    http_client: reqwest::Client,
}

impl ZapClient {
    /// Build a ZAP client from service settings.
    pub fn from_settings(settings: &Settings) -> Result<Self, ZapClientError> {
        Self::new(settings.zap_base_url.clone(), settings.zap_api_key.clone())
    }

    /// Build a ZAP client from a base URL and API key.
    pub fn new(base_url: String, api_key: String) -> Result<Self, ZapClientError> {
        if base_url.trim().is_empty() {
            return Err(ZapClientError::MissingSetting("GREENBONE_WAS_ZAP_BASE_URL"));
        }

        if api_key.trim().is_empty() {
            return Err(ZapClientError::MissingSetting("GREENBONE_WAS_ZAP_API_KEY"));
        }

        let parsed_base_url =
            reqwest::Url::parse(&base_url).map_err(|_| ZapClientError::InvalidBaseUrl(base_url))?;

        Ok(Self {
            base_url: parsed_base_url,
            api_key,
            http_client: reqwest::Client::new(),
        })
    }

    fn endpoint_url(&self, endpoint: &str) -> reqwest::Url {
        let mut url = self.base_url.clone();

        let base_path = url.path().trim_end_matches('/');
        let endpoint = endpoint.trim_start_matches('/');
        let joined_path = if base_path.is_empty() || base_path == "/" {
            format!("/{endpoint}")
        } else {
            format!("{base_path}/{endpoint}")
        };

        url.set_path(&joined_path);
        url.set_query(None);
        url
    }
}

/// ZAP client wrapper that automatically retries operations on transient failures.
///
/// Wraps a [`ZapClient`] and applies exponential backoff retry logic to all API calls.
/// Transient errors (network failures, timeouts) are automatically retried; permanent
/// errors (invalid arguments, authentication failures) fail immediately.
#[derive(Debug, Clone)]
pub struct RetryingZapClient {
    inner: ZapClient,
    max_retries: u32,
    max_delay: std::time::Duration,
}

impl RetryingZapClient {
    /// Create a new retrying client from an existing ZAP client.
    pub fn new(client: ZapClient, max_retries: u32, max_delay: std::time::Duration) -> Self {
        Self {
            inner: client,
            max_retries,
            max_delay,
        }
    }

    /// Start an AJAX spider scan on the given context and target URL.
    pub async fn start_ajax_spider_scan(
        &self,
        context_name: &str,
        target_url: &str,
        in_scope: bool,
        recursive: bool,
    ) -> Result<(), ZapClientError> {
        let inner = self.inner.clone();
        let context = context_name.to_string();
        let target = target_url.to_string();

        crate::scan::retry::with_retry(
            "zap.start_ajax_spider_scan",
            move || {
                let inner = inner.clone();
                let context = context.clone();
                let target = target.clone();
                async move {
                    inner
                        .start_ajax_spider_scan(&context, &target, in_scope, recursive)
                        .await
                }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Get the current status of the AJAX spider.
    pub async fn get_ajax_spider_status(
        &self,
    ) -> Result<ajaxspider::AjaxSpiderStatus, ZapClientError> {
        let inner = self.inner.clone();

        crate::scan::retry::with_retry(
            "zap.get_ajax_spider_status",
            move || {
                let inner = inner.clone();
                async move { inner.get_ajax_spider_status().await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Stop the AJAX spider scan.
    pub async fn stop_ajax_spider_scan(&self) -> Result<(), ZapClientError> {
        let inner = self.inner.clone();

        crate::scan::retry::with_retry(
            "zap.stop_ajax_spider_scan",
            move || {
                let inner = inner.clone();
                async move { inner.stop_ajax_spider_scan().await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Start an active scan on the given context and target URL.
    pub async fn start_active_scan(
        &self,
        context_id: &str,
        target_url: &str,
        recurse: bool,
        in_scope: bool,
    ) -> Result<String, ZapClientError> {
        let inner = self.inner.clone();
        let context = context_id.to_string();
        let target = target_url.to_string();

        crate::scan::retry::with_retry(
            "zap.start_active_scan",
            move || {
                let inner = inner.clone();
                let context = context.clone();
                let target = target.clone();
                async move {
                    inner
                        .start_active_scan(&context, &target, recurse, in_scope)
                        .await
                }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Get the progress percentage (0-100) of an active scan.
    pub async fn get_active_scan_status(&self, scan_id: &str) -> Result<i32, ZapClientError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();

        crate::scan::retry::with_retry(
            "zap.get_active_scan_status",
            move || {
                let inner = inner.clone();
                let id = id.clone();
                async move { inner.get_active_scan_status(&id).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Stop an active scan.
    pub async fn stop_active_scan(&self, scan_id: &str) -> Result<(), ZapClientError> {
        let inner = self.inner.clone();
        let id = scan_id.to_string();

        crate::scan::retry::with_retry(
            "zap.stop_active_scan",
            move || {
                let inner = inner.clone();
                let id = id.clone();
                async move { inner.stop_active_scan(&id).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Create a new context with the given name.
    pub async fn new_context(&self, context_name: &str) -> Result<String, ZapClientError> {
        let inner = self.inner.clone();
        let name = context_name.to_string();

        crate::scan::retry::with_retry(
            "zap.new_context",
            move || {
                let inner = inner.clone();
                let name = name.clone();
                async move { inner.new_context(&name).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Include a URL pattern in a context.
    pub async fn include_in_context(
        &self,
        context_name: &str,
        url_pattern: &str,
    ) -> Result<(), ZapClientError> {
        let inner = self.inner.clone();
        let context = context_name.to_string();
        let pattern = url_pattern.to_string();

        crate::scan::retry::with_retry(
            "zap.include_in_context",
            move || {
                let inner = inner.clone();
                let context = context.clone();
                let pattern = pattern.clone();
                async move { inner.include_in_context(&context, &pattern).await }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Fetch alerts from a context.
    pub async fn get_alerts(
        &self,
        context_name: &str,
        base_url: Option<&str>,
        start: Option<u32>,
        count: Option<u32>,
    ) -> Result<Vec<alert::Alert>, ZapClientError> {
        let inner = self.inner.clone();
        let context = context_name.to_string();
        let base = base_url.map(|s| s.to_string());

        crate::scan::retry::with_retry(
            "zap.get_alerts",
            move || {
                let inner = inner.clone();
                let context = context.clone();
                let base = base.clone();
                async move {
                    inner
                        .get_alerts(&context, base.as_deref(), start, count)
                        .await
                }
            },
            self.max_retries,
            self.max_delay,
        )
        .await
    }

    /// Remove a context by name. This operation is **not retried** as cleanup
    /// operations are typically best-effort.
    pub async fn remove_context(&self, context_name: &str) -> Result<(), ZapClientError> {
        self.inner.remove_context(context_name).await
    }

    /// Get a reference to the inner [`ZapClient`] for direct access when retries are not desired.
    pub fn inner(&self) -> &ZapClient {
        &self.inner
    }
}

#[cfg(test)]
#[path = "zapclient_tests.rs"]
mod zapclient_tests;
