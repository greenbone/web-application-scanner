// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{ZapClient, ZapClientError};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum AlertRiskLevel {
    Informational,
    Low,
    Medium,
    High,
    #[serde(other)]
    Unknown,
}

/// A single alert returned by the ZAP `alert/view/alerts` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Alert {
    /// The plugin ID of the alert.
    #[serde(rename = "pluginId")]
    pub plugin_id: String,

    /// The name of the alert, e.g. "Cross Site Scripting".
    pub name: String,

    /// The risk level of the alert, e.g. "High", "Medium", "Low", or "Informational".
    pub risk: AlertRiskLevel,

    /// A description of the alert, including details about the vulnerability and how it was detected.
    pub description: String,

    /// The URL where the alert was triggered.
    pub url: String,
}

/// Response payload returned by the ZAP `alert/view/alerts` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AlertsResponse {
    alerts: Vec<Alert>,
}

impl ZapClient {
    /// Get the list of alerts for the specified context name, optional base URL and
    /// optional pagination parameters.
    pub async fn get_alerts(
        &self,
        context_name: &str,
        base_url: Option<&str>,
        start: Option<u32>,
        count: Option<u32>,
    ) -> Result<Vec<Alert>, ZapClientError> {
        let endpoint = self.endpoint_url("JSON/alert/view/alerts");
        let mut form = vec![
            ("apikey".to_string(), self.api_key.clone()),
            ("contextName".to_string(), context_name.to_string()),
        ];

        if let Some(base_url) = base_url {
            form.push(("baseurl".to_string(), base_url.to_string()));
        }

        if let Some(start) = start {
            form.push(("start".to_string(), start.to_string()));
        }

        if let Some(count) = count {
            form.push(("count".to_string(), count.to_string()));
        }

        let response = self
            .http_client
            .post(endpoint)
            .form(&form)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<AlertsResponse>(&body)?;

        Ok(parsed_response.alerts)
    }
}

#[cfg(test)]
#[path = "alert_tests.rs"]
mod alert_tests;
