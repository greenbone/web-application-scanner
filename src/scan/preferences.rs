// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scan-owned scanner preference definitions and defaults.

use serde::{Deserialize, Serialize};

/// Preference ID for selecting scan behavior mode.
pub const SCAN_MODE_PREFERENCE_ID: &str = "scan_mode";

/// Preference ID for AJAX spider timeout in seconds.
pub const AJAX_SPIDER_TIMEOUT_PREFERENCE_ID: &str = "ajax_spider_timeout";

/// Scan execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    /// Disable active scan stage.
    Safe,
    /// Enable active scan stage.
    Active,
}

impl ScanMode {
    /// Default scan mode used when no preference override is provided.
    pub const fn default_mode() -> Self {
        Self::Safe
    }

    /// String representation used in preference transport values.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Active => "active",
        }
    }
}

/// Logical value type for preference metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferenceValueType {
    /// Finite set of string values.
    Enum,
    /// Non-negative decimal integer represented as string.
    Integer,
}

impl PreferenceValueType {
    /// Lowercase schema-friendly type name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Enum => "enum",
            Self::Integer => "integer",
        }
    }
}

/// Static definition for one supported scanner preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerPreferenceDefinition {
    /// Stable preference ID.
    pub id: &'static str,
    /// Human-readable preference display name.
    pub name: &'static str,
    /// Description shown by preference discovery endpoint.
    pub description: &'static str,
    /// Logical value type.
    pub value_type: PreferenceValueType,
    /// Default value encoded as string for transport compatibility.
    pub default_value: &'static str,
    /// Allowed values for enum preferences; empty for numeric values.
    pub allowed_values: &'static [&'static str],
}

/// `scan_mode` preference definition.
pub const SCAN_MODE_PREFERENCE: ScannerPreferenceDefinition = ScannerPreferenceDefinition {
    id: SCAN_MODE_PREFERENCE_ID,
    name: "Scan Mode",
    description: "Scan mode: 'safe' disables active scans, 'active' enables active scans.",
    value_type: PreferenceValueType::Enum,
    default_value: "safe",
    allowed_values: &["safe", "active"],
};

/// `ajax_spider_timeout` preference definition.
pub const AJAX_SPIDER_TIMEOUT_PREFERENCE: ScannerPreferenceDefinition =
    ScannerPreferenceDefinition {
        id: AJAX_SPIDER_TIMEOUT_PREFERENCE_ID,
        name: "AJAX Spider Timeout",
        description: "Scan-level AJAX spider timeout in seconds; enforced per target. Value 0 means unlimited.",
        value_type: PreferenceValueType::Integer,
        default_value: "0",
        allowed_values: &[],
    };

/// All supported scanner preference definitions.
pub fn preference_definitions() -> &'static [ScannerPreferenceDefinition] {
    &[SCAN_MODE_PREFERENCE, AJAX_SPIDER_TIMEOUT_PREFERENCE]
}

/// Default scanner preference values as `(id, value)` tuples.
pub fn default_preference_values() -> Vec<(&'static str, &'static str)> {
    preference_definitions()
        .iter()
        .map(|pref| (pref.id, pref.default_value))
        .collect()
}

#[cfg(test)]
#[path = "preferences_tests.rs"]
mod preferences_tests;
