// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! ZAP Context API endpoint client.

use super::{ZapClient, ZapClientError};
use serde::Deserialize;

/// Response payload returned by the ZAP `contextList` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ContextListResponse {
    /// List of available ZAP context names.
    #[serde(rename = "contextList")]
    context_list: Vec<String>,
}

/// Response payload returned by the ZAP `newContext` endpoint.
#[derive(Deserialize)]
struct NewContextResponse {
    #[serde(rename = "contextId")]
    context_id: String,
}

/// Response payload returned by the ZAP `removeContext` endpoint.
#[derive(Deserialize)]
struct RemoveContextResponse {
    #[serde(rename = "Result")]
    result: String,
}

/// Response payload returned by the ZAP `includeInContext` endpoint.
#[derive(Deserialize)]
struct IncludeInContextResponse {
    #[serde(rename = "Result")]
    result: String,
}

impl ZapClient {
    /// Get the list of contexts.
    pub async fn get_context_list(&self) -> Result<Vec<String>, ZapClientError> {
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

        let parsed_response = serde_json::from_str::<ContextListResponse>(&body)?;

        Ok(parsed_response.context_list)
    }

    /// Create a new context with the given name.
    /// Returns the ID of the newly created context on success.
    pub async fn new_context(&self, name: &str) -> Result<String, ZapClientError> {
        let endpoint = self.endpoint_url("JSON/context/action/newContext");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[("apikey", self.api_key.as_str()), ("contextName", name)])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<NewContextResponse>(&body)?;

        Ok(parsed_response.context_id)
    }

    /// Remove a context by name.
    /// Returns `Ok(())` on success, or an error if the context could not be removed.
    pub async fn remove_context(&self, name: &str) -> Result<(), ZapClientError> {
        let endpoint = self.endpoint_url("JSON/context/action/removeContext");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[("apikey", self.api_key.as_str()), ("contextName", name)])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<RemoveContextResponse>(&body)?;

        if parsed_response.result == "OK" {
            Ok(())
        } else {
            Err(ZapClientError::UnexpectedContent {
                field: "Result".to_string(),
                content: parsed_response.result,
            })
        }
    }

    /// Include a URL in the context with the given name.
    /// Returns `Ok(())` on success, or an error if the URL could not be included.
    pub async fn include_in_context(
        &self,
        context_name: &str,
        regex: &str,
    ) -> Result<(), ZapClientError> {
        let endpoint = self.endpoint_url("JSON/context/action/includeInContext");
        let response = self
            .http_client
            .post(endpoint)
            .form(&[
                ("apikey", self.api_key.as_str()),
                ("contextName", context_name),
                ("regex", regex),
            ])
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(ZapClientError::UnexpectedStatus { status, body });
        }

        let parsed_response = serde_json::from_str::<IncludeInContextResponse>(&body)?;

        if parsed_response.result == "OK" {
            Ok(())
        } else {
            Err(ZapClientError::UnexpectedContent {
                field: "Result".to_string(),
                content: parsed_response.result,
            })
        }
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
