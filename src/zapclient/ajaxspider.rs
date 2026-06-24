// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{ZapClient, ZapClientError};
use serde::Deserialize;

/// Normalized status values returned by the AJAX Spider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AjaxSpiderStatus {
    Running,
    Stopped,
}

/// Response payload returned by the ZAP `ajaxSpider/Scan` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AjaxSpiderScanResponse {
    /// The result of the AJAX Spider scan request.
    #[serde(rename = "Result")]
    status: String,
}

/// Response payload returned by the ZAP `ajaxSpider/Status` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AjaxSpiderStatusResponse {
    /// The current status of the AJAX Spider scan.
    status: String,
}

/// Response payload returned by the ZAP `ajaxSpider/stop` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AjaxSpiderStopResponse {
    /// The stop request status returned by ZAP.
    #[serde(rename = "Result")]
    status: String,
}

/// Response payload returned by the ZAP `ajaxSpider/setOptionMaxDuration` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AjaxSpiderSetOptionMaxDurationResponse {
    /// The option update request status returned by ZAP.
    #[serde(rename = "Result")]
    status: String,
}

impl ZapClient {
    /// Start an AJAX Spider scan for the specified context, target URL and options.
    pub async fn start_ajax_spider_scan(
        &self,
        context_name: &str,
        url: &str,
        in_scope: bool,
        subtree_only: bool,
    ) -> Result<(), ZapClientError> {
        let endpoint = self.endpoint_url("JSON/ajaxSpider/action/scan");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[
                ("apikey", self.api_key.as_str()),
                ("url", url),
                ("inScope", if in_scope { "true" } else { "false" }),
                ("contextName", context_name),
                ("subtreeOnly", if subtree_only { "true" } else { "false" }),
            ])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<AjaxSpiderScanResponse>(&body)?;

        if parsed_response.status != "OK" {
            return Err(ZapClientError::UnexpectedContent {
                field: "Result".to_string(),
                content: parsed_response.status,
            });
        }

        Ok(())
    }

    /// Set the global AJAX spider max duration option in seconds.
    pub async fn set_ajax_spider_max_duration(
        &self,
        max_duration_seconds: u64,
    ) -> Result<(), ZapClientError> {
        let endpoint = self.endpoint_url("JSON/ajaxSpider/action/setOptionMaxDuration");
        let form = vec![
            ("apikey".to_string(), self.api_key.clone()),
            ("Integer".to_string(), max_duration_seconds.to_string()),
        ];
        let response = self.http_client.post(endpoint).form(&form).send().await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response =
            serde_json::from_str::<AjaxSpiderSetOptionMaxDurationResponse>(&body)?;
        if parsed_response.status != "OK" {
            return Err(ZapClientError::UnexpectedContent {
                field: "Result".to_string(),
                content: parsed_response.status,
            });
        }

        Ok(())
    }

    /// Get the current status of the AJAX Spider scan.
    pub async fn get_ajax_spider_status(&self) -> Result<AjaxSpiderStatus, ZapClientError> {
        let endpoint = self.endpoint_url("JSON/ajaxSpider/view/status");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[("apikey", self.api_key.as_str())])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<AjaxSpiderStatusResponse>(&body)?;
        match parsed_response.status.as_str() {
            "running" => Ok(AjaxSpiderStatus::Running),
            "stopped" => Ok(AjaxSpiderStatus::Stopped),
            _ => Err(ZapClientError::UnexpectedContent {
                field: "status".to_string(),
                content: parsed_response.status,
            }),
        }
    }

    /// Stop the currently running AJAX Spider scan.
    pub async fn stop_ajax_spider_scan(&self) -> Result<(), ZapClientError> {
        let endpoint = self.endpoint_url("JSON/ajaxSpider/action/stop");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[("apikey", self.api_key.as_str())])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<AjaxSpiderStopResponse>(&body)?;
        if parsed_response.status != "OK" {
            return Err(ZapClientError::UnexpectedContent {
                field: "Result".to_string(),
                content: parsed_response.status,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "ajaxspider_tests.rs"]
mod ajaxspider_tests;
