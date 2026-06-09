// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

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
