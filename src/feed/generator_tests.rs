// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use crate::feed::alert_doc::parse_alert_doc;

use super::*;

fn parse(path: &str, content: &str) -> AlertDoc {
    parse_alert_doc(Path::new(path), content).unwrap().unwrap()
}

#[test]
fn generator_uses_child_metadata_and_parent_alertset_traceability() {
    let parent = parse(
        "10020.md",
        r#"---
title: "Anti-clickjacking Header"
alertid: 10020
alertindex: 1002000
alerttype: "Passive"
status: release
type: alertset
alerts:
  10020-1:
    alertid: 10020-1
    name: "Missing Anti-clickjacking Header"
code: https://example.com/source
---
"#,
    );
    let child = parse(
        "10020-1.md",
        r#"---
title: "Missing Anti-clickjacking Header"
alertid: 10020-1
alertindex: 1002001
type: alert
risk: Medium
solution: "Set CSP."
references:
  - https://example.com/ref
cwe: 1021
wasc: 15
alerttags:
  - CWE-1021
  - OWASP_2021_A05
  - OWASP_2023_API4
  - WSTG-v42-CLNT-09
  - CUSTOM_PAYLOADS
  - TEST_TIMING
  - OUT_OF_BAND
  - POLICY_QA_STD
other: ""
---
The response does not protect against clickjacking.
"#,
    );

    let (vts, report) = generate_vts(
        vec![parent, child],
        &GenerateConfig {
            version_date: "2026-06-01T00:00:00+0000".to_string(),
        },
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics());
    assert!(!report.has_warnings(), "{:?}", report.diagnostics());
    assert_eq!(vts.len(), 1);
    let nasl = &vts[0].rendered;
    assert!(nasl.contains("script_category(ACT_GATHER_INFO);"));
    assert!(nasl.contains("script_xref(name:\"ZAP-Alert-Type\", value:\"Passive\");"));
    assert!(
        nasl.contains("script_xref(name:\"ZAP-Alert-Set\", value:\"Anti-clickjacking Header\");")
    );
    assert!(!nasl.contains("script_tag(name:\"cvss_base\""));
    assert!(!nasl.contains("script_tag(name:\"cvss_base_vector\""));
    assert!(!nasl.contains("script_tag(name:\"severity_origin\""));
    assert!(nasl.contains("script_xref(name:\"OWASP\", value:\"OWASP_2021_A05\");"));
    assert!(nasl.contains("script_xref(name:\"OWASP-API\", value:\"OWASP_2023_API4\");"));
    assert!(!nasl.contains("CUSTOM_PAYLOADS"));
    assert!(!nasl.contains("TEST_TIMING"));
    assert!(!nasl.contains("OUT_OF_BAND"));
    assert!(!nasl.contains("POLICY_QA_STD"));
}

#[test]
fn generator_emits_parent_alertset_vt_when_no_child_alert_exists() {
    let parent = parse(
        "777.md",
        r#"---
title: "Parent Only"
alertid: 777
alertindex: 77700
alerttype: "Tool"
status: release
type: alertset
code: https://example.com/source
---
Parent summary.
"#,
    );

    let (vts, report) = generate_vts(
        vec![parent],
        &GenerateConfig {
            version_date: "2026-06-01T00:00:00+0000".to_string(),
        },
    );

    assert_eq!(vts.len(), 1);
    assert!(report.has_warnings());
    assert!(vts[0].rendered.contains("script_name(\"Parent Only\");"));
    assert!(
        vts[0]
            .rendered
            .contains("script_xref(name:\"ZAP-Alert-Type\", value:\"Tool\");")
    );
}

#[test]
fn deprecated_alerts_are_generated_with_will_not_fix_solution_type() {
    let doc = parse(
        "10046.md",
        r#"---
title: "Deprecated Alert"
alertid: 10046
alertindex: 1004600
alerttype: "Passive"
status: deprecated
type: alert
risk: Low
cwe: 1
wasc: 1
alerttags: []
---
This alert is deprecated.
"#,
    );

    let (vts, report) = generate_vts(
        vec![doc],
        &GenerateConfig {
            version_date: "2026-06-01T00:00:00+0000".to_string(),
        },
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics());
    assert!(
        vts[0]
            .rendered
            .contains("script_tag(name:\"solution_type\", value:\"WillNotFix\");")
    );
    assert!(
        vts[0]
            .rendered
            .contains("script_xref(name:\"ZAP-Status\", value:\"deprecated\");")
    );
}

#[test]
fn explicit_cvss_fields_are_rendered_without_risk_inference() {
    let doc = parse(
        "40012.md",
        r#"---
title: "XSS"
alertid: 40012
alertindex: 4001200
alerttype: "Active"
status: release
type: alert
risk: Medium
cvss_base: "5.0"
cvss_base_vector: "AV:N/AC:L/Au:N/C:P/I:N/A:N"
severity_origin: "ZAP"
solution: "Validate input."
cwe: 79
wasc: 8
alerttags: []
---
Summary.
"#,
    );

    let (vts, report) = generate_vts(
        vec![doc],
        &GenerateConfig {
            version_date: "2026-06-01T00:00:00+0000".to_string(),
        },
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics());
    assert_eq!(vts.len(), 1);
    let nasl = &vts[0].rendered;
    assert!(nasl.contains("script_tag(name:\"cvss_base\", value:\"5.0\");"));
    assert!(
        nasl.contains(
            "script_tag(name:\"cvss_base_vector\", value:\"AV:N/AC:L/Au:N/C:P/I:N/A:N\");"
        )
    );
    assert!(nasl.contains("script_tag(name:\"severity_origin\", value:\"ZAP\");"));
}

#[test]
fn generator_emits_leaf_alerts_missing_optional_risk_or_solution_without_severity_or_solution_tags()
{
    let doc = parse(
        "40012.md",
        r#"---
title: "XSS"
alertid: 40012
alertindex: 4001200
alerttype: "Active"
status: release
type: alert
cwe: 79
wasc: 8
alerttags: []
---
Summary.
"#,
    );

    let (vts, report) = generate_vts(
        vec![doc],
        &GenerateConfig {
            version_date: "2026-06-01T00:00:00+0000".to_string(),
        },
    );

    assert_eq!(vts.len(), 1);
    assert!(!report.has_errors(), "{:?}", report.diagnostics());
    assert_eq!(report.warning_count(), 2);
    assert!(!vts[0].rendered.contains("cvss_base"));
    assert!(!vts[0].rendered.contains("cvss_base_vector"));
    assert!(!vts[0].rendered.contains("severity_origin"));
    assert!(!vts[0].rendered.contains("script_tag(name:\"solution\""));
    assert!(
        !vts[0]
            .rendered
            .contains("script_tag(name:\"solution_type\"")
    );
}
