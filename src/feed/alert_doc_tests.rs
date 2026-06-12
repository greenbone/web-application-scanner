// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use super::*;

#[test]
fn alert_id_parses_base_and_sub_ids_for_oid_and_file_names() {
    let id = "10020-1".parse::<AlertId>().unwrap();
    assert_eq!(id.base, 10020);
    assert_eq!(id.sub, Some(1));
    assert_eq!(id.file_stem(), "zap_10020_1");
}

#[test]
fn parser_reads_leaf_frontmatter_fields_used_by_metadata_mapping() {
    let doc = parse_alert_doc(
        Path::new("10020-1.md"),
        r#"---
title: "Missing Header"
alertid: 10020-1
alertindex: 1002001
alerttype: "Passive"
status: release
type: alert
risk: Medium
cvss_base: "5.0"
cvss_base_vector: "AV:N/AC:L/Au:N/C:P/I:N/A:N"
severity_origin: ZAP
solution: "Set a header."
references:
  - https://example.com/ref
cwe: 1021
wasc: 15
alerttags:
  - CWE-1021
---
Body text.
"#,
    )
    .unwrap()
    .unwrap();

    assert_eq!(doc.alert_id.to_string(), "10020-1");
    assert_eq!(doc.kind, AlertKind::Alert);
    assert_eq!(doc.cvss_base.as_deref(), Some("5.0"));
    assert_eq!(
        doc.cvss_base_vector.as_deref(),
        Some("AV:N/AC:L/Au:N/C:P/I:N/A:N")
    );
    assert_eq!(doc.severity_origin.as_deref(), Some("ZAP"));
    assert_eq!(doc.references, ["https://example.com/ref"]);
    assert_eq!(doc.body, "Body text.");
    assert_eq!(doc.raw_frontmatter["title"], "Missing Header");
}

#[test]
fn parser_preserves_empty_raw_frontmatter_values_as_null_for_index_generation() {
    let doc = parse_alert_doc(
        Path::new("10020-1.md"),
        r#"---
title: "Missing Header"
alertid: 10020-1
type: alert
solution: ""
---
Body text.
"#,
    )
    .unwrap()
    .unwrap();

    assert_eq!(doc.solution, None);
    assert_eq!(doc.raw_frontmatter["solution"], serde_json::Value::Null);
}

#[test]
fn parser_rejects_missing_frontmatter_so_invalid_sources_fail_early() {
    let err = parse_alert_doc(Path::new("broken.md"), "title: Missing").unwrap_err();
    assert_eq!(err, "missing YAML frontmatter delimiter");
}

#[test]
fn parser_rejects_non_scalar_references_so_invalid_urls_are_not_silently_dropped() {
    let err = parse_alert_doc(
        Path::new("broken-reference.md"),
        r#"---
title: "Broken Reference"
alertid: 10020
type: alert
references:
  - https://example.com/ref
  - nested:
      value: https://example.com/hidden
---
Body text.
"#,
    )
    .unwrap_err();

    assert_eq!(err, "references[1] must be a scalar value");
}
