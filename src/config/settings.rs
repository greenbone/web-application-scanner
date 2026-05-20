// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use config::{Config, ConfigBuilder, ConfigError, Environment, builder::DefaultState};
use serde::Deserialize;

pub const DEFAULT_LOG_FORMAT: &str = "fmt";
pub const DEFAULT_LOG_LEVEL: &str = "info";
pub const DEFAULT_PORT: u16 = 8030;

#[derive(Debug, Clone)]
pub struct Settings {
    pub log_format: String,
    pub log_level: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
struct RawSettings {
    log_format: String,
    log_level: String,
    port: u16,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();

        let cfg: Config = Self::config_builder()?
            .add_source(Environment::with_prefix("GREENBONE_WAS"))
            .build()?;

        let raw_settings: RawSettings = cfg.try_deserialize::<RawSettings>()?;
        Self::from_raw(raw_settings)
    }

    fn config_builder() -> Result<ConfigBuilder<DefaultState>, ConfigError> {
        Config::builder()
            .set_default("log_format", "fmt")?
            .set_default("log_level", "info")?
            .set_default("port", 8030)
    }

    fn from_raw(raw: RawSettings) -> Result<Self, ConfigError> {
        if raw.port == 0 {
            return Err(ConfigError::Message(
                "port must be between 1 and 65535".to_string(),
            ));
        }

        Ok(Self {
            log_format: raw.log_format,
            log_level: raw.log_level,
            port: raw.port,
        })
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
