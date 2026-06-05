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

#[cfg(test)]
#[path = "zapclient_tests.rs"]
mod zapclient_tests;
