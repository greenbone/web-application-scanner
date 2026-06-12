// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("greenbone-was-vtgen-{name}-{unique}"))
}

#[test]
fn parse_cli_requires_input_and_output_to_prevent_accidental_generation() {
    let err = parse_cli(["--input".to_string(), "alerts".to_string()]).unwrap_err();
    assert_eq!(err, "--output is required");
}

#[test]
fn parse_cli_rejects_removed_contributor_option() {
    let err = parse_cli([
        "--input".to_string(),
        "alerts".to_string(),
        "--output".to_string(),
        "nasl".to_string(),
        "--contributor".to_string(),
        "123456".to_string(),
    ])
    .unwrap_err();
    assert_eq!(err, "unknown argument: --contributor");
}

#[test]
fn civil_from_days_formats_unix_epoch_for_default_version_dates() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
}

#[test]
fn run_writes_alert_index_next_to_generated_nasl_files() {
    let input = unique_temp_dir("input");
    let output = unique_temp_dir("output");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(
        input.join("10020.md"),
        r#"---
title: "Missing Header"
alertid: 10020
alertindex: 1002000
alerttype: "Passive"
status: release
type: alert
risk: Medium
solution: ""
cwe: 1021
wasc: 15
---
Body.
"#,
    )
    .unwrap();

    run([
        "--input".to_string(),
        input.display().to_string(),
        "--output".to_string(),
        output.display().to_string(),
        "--version-date".to_string(),
        "2026-06-01T00:00:00+0000".to_string(),
    ])
    .unwrap();

    let index_path = output.join("alert-index.json");
    assert!(index_path.is_file());
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(index_path).unwrap()).unwrap();
    assert_eq!(index["alerts"][0]["id"], "10020");
    assert!(index["alerts"][0].get("solution").is_none());
    assert_eq!(
        index["alerts"][0]["raw_frontmatter"]["solution"],
        serde_json::Value::Null
    );
    assert!(output.join("zap_10020.nasl").is_file());
}

#[test]
fn dry_run_does_not_create_alert_index_file() {
    let input = unique_temp_dir("dry-input");
    let output = unique_temp_dir("dry-output");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(
        input.join("10020.md"),
        r#"---
title: "Missing Header"
alertid: 10020
type: alert
---
Body.
"#,
    )
    .unwrap();

    run([
        "--input".to_string(),
        input.display().to_string(),
        "--output".to_string(),
        output.display().to_string(),
        "--dry-run".to_string(),
    ])
    .unwrap();

    assert!(!output.join("alert-index.json").exists());
}
