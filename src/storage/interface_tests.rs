// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn parse_range_single_index() {
    assert_eq!(parse_range("5").unwrap(), (5, None));
}

#[test]
fn parse_range_pair() {
    assert_eq!(parse_range("2-10").unwrap(), (2, Some(10)));
}

#[test]
fn parse_range_inverted_is_error() {
    assert!(parse_range("10-2").is_err());
}

#[test]
fn parse_range_non_numeric_is_error() {
    assert!(parse_range("abc").is_err());
    assert!(parse_range("1-abc").is_err());
}

#[test]
fn parse_range_whitespace_trimmed() {
    assert_eq!(parse_range(" 3 - 7 ").unwrap(), (3, Some(7)));
}
