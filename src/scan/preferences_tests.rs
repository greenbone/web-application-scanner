// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{
    AJAX_SPIDER_TIMEOUT_PREFERENCE, AJAX_SPIDER_TIMEOUT_PREFERENCE_ID, PreferenceValueType,
    SCAN_MODE_PREFERENCE, SCAN_MODE_PREFERENCE_ID, ScanMode, default_preference_values,
    preference_definitions,
};

#[test]
fn scan_mode_default_is_safe() {
    assert_eq!(ScanMode::default_mode(), ScanMode::Safe);
    assert_eq!(ScanMode::default_mode().as_str(), "safe");
}

#[test]
fn preference_definitions_include_scan_mode_and_ajax_spider_timeout() {
    let defs = preference_definitions();
    assert_eq!(defs.len(), 2);
    assert!(defs.iter().any(|p| p.id == SCAN_MODE_PREFERENCE_ID));
    assert!(defs
        .iter()
        .any(|p| p.id == AJAX_SPIDER_TIMEOUT_PREFERENCE_ID));
}

#[test]
fn scan_mode_preference_definition_matches_contract() {
    assert_eq!(SCAN_MODE_PREFERENCE.id, SCAN_MODE_PREFERENCE_ID);
    assert_eq!(SCAN_MODE_PREFERENCE.value_type, PreferenceValueType::Enum);
    assert_eq!(SCAN_MODE_PREFERENCE.default_value, "safe");
    assert_eq!(SCAN_MODE_PREFERENCE.allowed_values, &["safe", "active"]);
}

#[test]
fn ajax_spider_timeout_definition_has_zero_default_for_unlimited() {
    assert_eq!(
        AJAX_SPIDER_TIMEOUT_PREFERENCE.id,
        AJAX_SPIDER_TIMEOUT_PREFERENCE_ID
    );
    assert_eq!(
        AJAX_SPIDER_TIMEOUT_PREFERENCE.value_type,
        PreferenceValueType::Integer
    );
    assert_eq!(AJAX_SPIDER_TIMEOUT_PREFERENCE.default_value, "0");
    assert!(AJAX_SPIDER_TIMEOUT_PREFERENCE
        .description
        .contains("0 means unlimited"));
}

#[test]
fn default_preference_values_match_definition_defaults() {
    let defaults = default_preference_values();
    assert_eq!(defaults.len(), 2);

    assert!(defaults
        .iter()
        .any(|(id, value)| *id == SCAN_MODE_PREFERENCE_ID && *value == "safe"));
    assert!(defaults
        .iter()
        .any(|(id, value)| *id == AJAX_SPIDER_TIMEOUT_PREFERENCE_ID && *value == "0"));
}
