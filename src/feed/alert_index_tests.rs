// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use serde_json::Value;

use crate::feed::alert_doc::parse_alert_doc;

use super::*;

fn parse(path: &str, content: &str) -> AlertDoc {
    parse_alert_doc(Path::new(path), content).unwrap().unwrap()
}

fn render_json(docs: &[AlertDoc]) -> Value {
    let (json, report) = generate_alert_index(docs);
    assert!(!report.has_errors(), "{:?}", report.diagnostics());
    serde_json::from_str(&json.unwrap()).unwrap()
}

#[test]
fn alert_index_entry_uses_schema_shape_and_omits_empty_solution() {
    let doc = parse(
        "10020-1.md",
        r#"---
title: "Missing Header"
alertid: 10020-1
alertindex: 1002001
alerttype: "Passive"
status: release
type: alert
risk: Medium
solution: ""
references:
  - https://example.com/ref
cwe: 1021
wasc: 15
custom_empty: ""
---
Body text.
"#,
    );

    let json = render_json(&[doc]);
    let alert = &json["alerts"][0];

    assert_eq!(json["schema_version"], 1);
    assert_eq!(alert["id"], "10020-1");
    assert_eq!(alert["oid"], "1.3.6.1.4.1.25623.3.10020.1");
    assert_eq!(alert["document_type"], "alert");
    assert_eq!(alert["name"], "Missing Header");
    assert_eq!(alert["description"], "Body text.");
    assert!(alert.get("solution").is_none());
    assert_eq!(alert["cwe"], serde_json::json!(["CWE-1021"]));
    assert_eq!(alert["cve"], serde_json::json!([]));
    assert_eq!(alert["wasc"], serde_json::json!(["WASC-15"]));
    assert_eq!(
        alert["references"],
        serde_json::json!([{ "type": "url", "value": "https://example.com/ref" }])
    );
    assert_eq!(alert["raw_frontmatter"]["solution"], Value::Null);
    assert_eq!(alert["raw_frontmatter"]["custom_empty"], Value::Null);
}

#[test]
fn alert_index_includes_parent_alertset_even_when_child_vt_exists() {
    let parent = parse(
        "10020.md",
        r#"---
title: "Anti-clickjacking Header"
alertid: 10020
alertindex: 1002000
type: alertset
alerts:
  10020-1:
    alertid: 10020-1
    name: "Missing Anti-clickjacking Header"
---
"#,
    );
    let child = parse(
        "10020-1.md",
        r#"---
alertid: 10020-1
type: alert
---
Child body.
"#,
    );

    let json = render_json(&[child, parent]);
    let alerts = json["alerts"].as_array().unwrap();

    assert_eq!(alerts.len(), 2);
    assert_eq!(alerts[0]["id"], "10020");
    assert_eq!(alerts[0]["document_type"], "alertset");
    assert_eq!(
        alerts[0]["child_alerts"],
        serde_json::json!([{ "id": "10020-1", "name": "Missing Anti-clickjacking Header" }])
    );
    assert_eq!(alerts[1]["id"], "10020-1");
    assert_eq!(alerts[1]["name"], "Missing Anti-clickjacking Header");
}

#[test]
fn alert_index_preserves_and_categorizes_all_source_alert_tags() {
    let doc = parse(
        "90023.md",
        r#"---
title: "XXE"
alertid: 90023
type: alert
alerttags:
  - CWE-611
  - CVE-2026-1234
  - PCI_DSS
  - OWASP_2021_A05
  - POLICY_PENTEST
  - WSTG-v42-INPV-07
  - SYSTEMIC
---
Body.
"#,
    );

    let json = render_json(&[doc]);
    let tags = &json["alerts"][0]["alert_tags"];

    assert_eq!(
        tags["raw"],
        serde_json::json!([
            "CVE-2026-1234",
            "CWE-611",
            "OWASP_2021_A05",
            "PCI_DSS",
            "POLICY_PENTEST",
            "SYSTEMIC",
            "WSTG-v42-INPV-07"
        ])
    );
    assert_eq!(tags["cwe"], serde_json::json!(["CWE-611"]));
    assert_eq!(tags["cve"], serde_json::json!(["CVE-2026-1234"]));
    assert_eq!(tags["compliance"], serde_json::json!(["PCI_DSS"]));
    assert_eq!(tags["owasp"], serde_json::json!(["OWASP_2021_A05"]));
    assert_eq!(tags["policies"], serde_json::json!(["POLICY_PENTEST"]));
    assert_eq!(tags["wstg"], serde_json::json!(["WSTG-v42-INPV-07"]));
    assert_eq!(tags["misc"], serde_json::json!(["SYSTEMIC"]));
    assert_eq!(json["alerts"][0]["cwe"], serde_json::json!(["CWE-611"]));
    assert_eq!(
        json["alerts"][0]["cve"],
        serde_json::json!(["CVE-2026-1234"])
    );
}

#[test]
fn alert_index_output_is_deterministic_for_same_documents() {
    let first = parse(
        "2.md",
        r#"---
title: "Second"
alertid: 2
type: alert
alerttags:
  - POLICY_QA_STD
---
Body.
"#,
    );
    let second = parse(
        "1.md",
        r#"---
title: "First"
alertid: 1
type: alert
---
Body.
"#,
    );

    let (left, left_report) = generate_alert_index(&[first.clone(), second.clone()]);
    let (right, right_report) = generate_alert_index(&[first, second]);

    assert!(!left_report.has_errors(), "{:?}", left_report.diagnostics());
    assert!(
        !right_report.has_errors(),
        "{:?}",
        right_report.diagnostics()
    );
    assert_eq!(left, right);
}
