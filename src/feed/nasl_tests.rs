// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn oid_encoding_uses_alert_id_arcs_from_spec_examples() {
    assert_eq!(
        encode_oid(&"353-1".parse().unwrap()),
        "1.3.6.1.4.1.25623.3.353.1"
    );
    assert_eq!(
        encode_oid(&"40012".parse().unwrap()),
        "1.3.6.1.4.1.25623.3.40012"
    );
    assert_eq!(
        encode_oid(&"10020-4".parse().unwrap()),
        "1.3.6.1.4.1.25623.3.10020.4"
    );
}

#[test]
fn text_normalization_protects_nasl_strings_from_quotes_and_backslashes() {
    let normalized = normalize_text("Use [docs](https://example.com).\n\u{2022} path C:\\tmp");
    assert_eq!(normalized, "Use docs (https://example.com). - path C:\\tmp");
    assert_eq!(
        escape_nasl_string("quote \" and slash \\"),
        "quote ' and slash \\\\"
    );
}

#[test]
fn text_normalization_preserves_literal_brackets_in_alert_descriptions() {
    let normalized = normalize_text("Header [X-Frame-Options] is missing.");
    assert_eq!(normalized, "Header [X-Frame-Options] is missing.");
}
