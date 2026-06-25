// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{ZapClient, ZapClientError};
use serde::Deserialize;

/// Response payload returned by the ZAP `pscan/view/recordsToScan` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PscanRecordsToScanResponse {
    #[serde(rename = "recordsToScan")]
    records_to_scan: String,
}

impl ZapClient {
    /// Get the number of records left to process in the passive scanner.
    pub async fn get_passive_scan_records_to_scan(&self) -> Result<u64, ZapClientError> {
        let endpoint = self.endpoint_url("JSON/pscan/view/recordsToScan");
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

        let parsed_response = serde_json::from_str::<PscanRecordsToScanResponse>(&body)?;
        let records_to_scan = parsed_response
            .records_to_scan
            .parse::<i64>()
            .map_err(|_| ZapClientError::UnexpectedContent {
                field: "recordsToScan".to_string(),
                content: parsed_response.records_to_scan.clone(),
            })?;

        if records_to_scan < 0 {
            return Err(ZapClientError::UnexpectedContent {
                field: "recordsToScan".to_string(),
                content: parsed_response.records_to_scan,
            });
        }

        Ok(records_to_scan as u64)
    }
}

#[cfg(test)]
#[path = "pscan_tests.rs"]
mod pscan_tests;
