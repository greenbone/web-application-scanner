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

#[test]
fn openvas_date_tag_format_converts_generated_version_timestamps() {
    assert_eq!(
        format_openvas_date_tag("2026-06-10T12:52:47+0000"),
        "2026-06-10 12:52:47 +0000"
    );
}

#[test]
fn openvas_date_tag_format_converts_zap_fractional_utc_timestamps() {
    assert_eq!(
        format_openvas_date_tag("2020-10-30 12:12:42.788Z"),
        "2020-10-30 12:12:42 +0000"
    );
}

#[test]
fn rendered_metadata_uses_openvas_date_tag_format_but_keeps_script_version_format() {
    let metadata = NaslMetadata {
        oid: "1.3.6.1.4.1.25623.3.100013".to_string(),
        version_date: "2026-06-10T12:52:47+0000".to_string(),
        creation_date: "2026-06-10T12:52:47+0000".to_string(),
        last_modification: "2020-10-30 12:12:42.788Z".to_string(),
        name: "Information Disclosure - Private IP Address".to_string(),
        copyright_year: "2026".to_string(),
        cvss_base: None,
        cvss_base_vector: None,
        severity_origin: None,
        xrefs: std::collections::BTreeSet::new(),
        cves: std::collections::BTreeSet::new(),
        tags: vec![Tag {
            name: "summary".to_string(),
            value: "Date tags must be accepted by OpenVAS description mode.".to_string(),
        }],
    };

    let rendered = render(&metadata);

    assert!(rendered.contains("script_version(\"2026-06-10T12:52:47+0000\");"));
    assert!(
        rendered
            .contains("script_tag(name:\"creation_date\", value:\"2026-06-10 12:52:47 +0000\");")
    );
    assert!(
        rendered.contains(
            "script_tag(name:\"last_modification\", value:\"2020-10-30 12:12:42 +0000\");"
        )
    );
}
