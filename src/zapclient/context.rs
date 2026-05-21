// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ZAP Context API endpoint client.

use serde::Deserialize;
use super::{ZapClient, ZapClientError};

/// Response payload returned by the ZAP `contextList` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ContextListResponse {
    /// List of available ZAP context names.
    #[serde(rename = "contextList")]
    pub context_list: Vec<String>,
}

impl ZapClient {
    /// Call the `contextList` endpoint via POST and parse the returned JSON payload.
    pub async fn context_list(&self) -> Result<ContextListResponse, ZapClientError> {
        let endpoint = self.endpoint_url("JSON/context/view/contextList");
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

        Ok(serde_json::from_str::<ContextListResponse>(&body)?)
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
