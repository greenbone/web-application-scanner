// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{ZapClient, ZapClientError};
use serde::Deserialize;

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

    /// Get the current status of the AJAX Spider scan.
    pub async fn get_ajax_spider_status(&self) -> Result<String, ZapClientError> {
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

        Ok(parsed_response.status)
    }
}

#[cfg(test)]
#[path = "ajaxspider_tests.rs"]
mod ajaxspider_tests;
