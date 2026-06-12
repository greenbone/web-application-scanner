// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{ZapClient, ZapClientError};
use serde::Deserialize;

/// Response payload returned by the ZAP `ascan/scan` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AscanScanResponse {
    /// The scan ID assigned by ZAP to the newly started active scan.
    /// This ID can be used to query the status of the scan and retrieve results.
    #[serde(rename = "scan")]
    scan_id: String,
}

/// Response payload returned by the ZAP `ascan/status` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AscanStatusResponse {
    /// The current status of the active scan, represented as a percentage (0-100).
    status: String,
}

/// Response payload returned by the ZAP `ascan/stop` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AscanStopResponse {
    /// The stop request status returned by ZAP.
    #[serde(rename = "Result")]
    status: String,
}

impl ZapClient {
    /// Start an active scan for the specified context and target URL.
    pub async fn start_active_scan(
        &self,
        context_id: &str,
        url: &str,
        recurse: bool,
        in_scope_only: bool,
    ) -> Result<String, ZapClientError> {
        let endpoint = self.endpoint_url("JSON/ascan/action/scan");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[
                ("apikey", self.api_key.as_str()),
                ("url", url),
                ("recurse", if recurse { "true" } else { "false" }),
                ("inScopeOnly", if in_scope_only { "true" } else { "false" }),
                ("contextId", context_id),
            ])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<AscanScanResponse>(&body)?;

        Ok(parsed_response.scan_id)
    }

    /// Get the current status (completion percentage) of the active scan with the given scan ID.
    pub async fn get_active_scan_status(&self, scan_id: &str) -> Result<i32, ZapClientError> {
        let endpoint = self.endpoint_url("JSON/ascan/view/status");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[("apikey", self.api_key.as_str()), ("scanId", scan_id)])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<AscanStatusResponse>(&body)?;
        let progress_int = parsed_response.status.parse::<i32>().map_err(|_| {
            ZapClientError::UnexpectedContent {
                field: "status".to_string(),
                content: parsed_response.status.clone(),
            }
        })?;

        if !(0..=100).contains(&progress_int) {
            return Err(ZapClientError::UnexpectedContent {
                field: "status".to_string(),
                content: parsed_response.status,
            });
        }

        Ok(progress_int)
    }

    /// Stop an active scan identified by scan ID.
    pub async fn stop_active_scan(&self, scan_id: &str) -> Result<(), ZapClientError> {
        let endpoint = self.endpoint_url("JSON/ascan/action/stop");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[("apikey", self.api_key.as_str()), ("scanId", scan_id)])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<AscanStopResponse>(&body)?;
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
#[path = "ascan_tests.rs"]
mod ascan_tests;
