// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::BTreeSet;

use super::alert_doc::AlertId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NaslMetadata {
    pub oid: String,
    pub version_date: String,
    pub creation_date: String,
    pub last_modification: String,
    pub name: String,
    pub copyright_year: String,
    pub cvss_base: Option<String>,
    pub cvss_base_vector: Option<String>,
    pub severity_origin: Option<String>,
    pub xrefs: BTreeSet<Xref>,
    pub cves: BTreeSet<String>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Xref {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

pub fn encode_oid(alert_id: &AlertId) -> String {
    match alert_id.sub {
        Some(sub) => format!("1.3.6.1.4.1.25623.3.{}.{}", alert_id.base, sub),
        None => format!("1.3.6.1.4.1.25623.3.{}", alert_id.base),
    }
}

pub fn render(metadata: &NaslMetadata) -> String {
    let mut output = String::new();
    output.push_str("if (description)\n{\n");
    line_call(&mut output, "script_oid", &[&metadata.oid]);
    line_call(&mut output, "script_version", &[&metadata.version_date]);
    line_tag(
        &mut output,
        "creation_date",
        &format_openvas_date_tag(&metadata.creation_date),
    );
    line_tag(
        &mut output,
        "last_modification",
        &format_openvas_date_tag(&metadata.last_modification),
    );
    line_call(&mut output, "script_name", &[&metadata.name]);
    output.push('\n');
    output.push_str("  script_category(ACT_GATHER_INFO);\n");
    line_call(&mut output, "script_family", &["Web application scanner"]);
    line_call(
        &mut output,
        "script_copyright",
        &[&format!(
            "Copyright (C) {} Greenbone AG",
            metadata.copyright_year
        )],
    );
    output.push('\n');

    if let Some(base) = &metadata.cvss_base {
        line_tag(&mut output, "cvss_base", base);
    }
    if let Some(vector) = &metadata.cvss_base_vector {
        line_tag(&mut output, "cvss_base_vector", vector);
    }
    if (metadata.cvss_base.is_some() || metadata.cvss_base_vector.is_some())
        && let Some(origin) = &metadata.severity_origin
    {
        line_tag(&mut output, "severity_origin", origin);
    }
    line_tag(&mut output, "qod_type", "remote_analysis");
    output.push('\n');

    for xref in &metadata.xrefs {
        output.push_str(&format!(
            "  script_xref(name:\"{}\", value:\"{}\");\n",
            escape_nasl_string(&xref.name),
            escape_nasl_string(&xref.value)
        ));
    }
    for cve in &metadata.cves {
        line_call(&mut output, "script_cve_id", &[cve]);
    }
    output.push('\n');

    for tag in &metadata.tags {
        line_tag(&mut output, &tag.name, &tag.value);
    }

    output.push('\n');
    output.push_str("  exit(0);\n");
    output.push_str("}\n");
    output
}

pub fn solution_type(status: Option<&str>, solution: Option<&str>) -> Option<&'static str> {
    if status
        .map(|status| status.eq_ignore_ascii_case("deprecated"))
        .unwrap_or(false)
    {
        return Some("WillNotFix");
    }
    let solution = solution?;
    let lower = solution.to_ascii_lowercase();
    if lower.contains("upgrade")
        || lower.contains("upgrading")
        || lower.contains("fixed version")
        || lower.contains("patch")
        || lower.contains("vendor")
    {
        Some("VendorFix")
    } else if lower.contains("workaround")
        || lower.contains("disable")
        || lower.contains("block access")
        || lower.contains("temporary")
    {
        Some("Workaround")
    } else {
        Some("Mitigation")
    }
}

pub fn normalize_body_summary(body: &str, fallback: &str) -> String {
    for paragraph in body.split("\n\n") {
        let normalized = normalize_text(paragraph);
        if !normalized.is_empty() && !normalized.starts_with('#') {
            return normalized;
        }
    }
    normalize_text(fallback)
}

pub fn normalize_text(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.trim().chars().peekable();
    let mut previous_was_space = false;
    while let Some(character) = chars.next() {
        let mapped = match character {
            '\u{2022}' | '\u{25e6}' | '\u{2043}' => Some('-'),
            '\r' | '\n' | '\t' => Some(' '),
            '[' => {
                let mut label = String::new();
                let mut rendered_link = false;
                while let Some(next) = chars.next() {
                    if next == ']' && chars.peek() == Some(&'(') {
                        chars.next();
                        let mut url = String::new();
                        for url_char in chars.by_ref() {
                            if url_char == ')' {
                                break;
                            }
                            url.push(url_char);
                        }
                        output.push_str(&label);
                        if !url.is_empty() {
                            output.push_str(" (");
                            output.push_str(&url);
                            output.push(')');
                        }
                        previous_was_space = false;
                        rendered_link = true;
                        break;
                    }
                    label.push(next);
                }
                if rendered_link {
                    None
                } else if label.is_empty() {
                    Some('[')
                } else {
                    output.push('[');
                    output.push_str(&label);
                    None
                }
            }
            '#' if output.is_empty() => None,
            other => Some(other),
        };

        let Some(mapped) = mapped else {
            continue;
        };
        if mapped.is_whitespace() {
            if !previous_was_space && !output.is_empty() {
                output.push(' ');
                previous_was_space = true;
            }
        } else {
            output.push(mapped);
            previous_was_space = false;
        }
    }
    output.trim().to_string()
}

pub fn format_openvas_date_tag(value: &str) -> String {
    let value = value.trim();
    if value.len() < 19 {
        return normalize_text(value);
    }

    let bytes = value.as_bytes();
    if !is_date_time_prefix(bytes) {
        return normalize_text(value);
    }

    let date = &value[..10];
    let time = &value[11..19];
    let mut rest = &value[19..];

    if let Some(stripped) = rest.strip_prefix('.') {
        let fraction_len = stripped
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        rest = &stripped[fraction_len..];
    }

    rest = rest.trim_start();
    let offset = if let Some(stripped) = rest.strip_prefix('Z') {
        rest = stripped;
        "+0000".to_string()
    } else if rest.len() >= 6
        && matches!(rest.as_bytes()[0], b'+' | b'-')
        && rest.as_bytes()[3] == b':'
        && rest.as_bytes()[1..3].iter().all(u8::is_ascii_digit)
        && rest.as_bytes()[4..6].iter().all(u8::is_ascii_digit)
    {
        let offset = format!("{}{}", &rest[..3], &rest[4..6]);
        rest = &rest[6..];
        offset
    } else if rest.len() >= 5
        && matches!(rest.as_bytes()[0], b'+' | b'-')
        && rest.as_bytes()[1..5].iter().all(u8::is_ascii_digit)
    {
        let offset = rest[..5].to_string();
        rest = &rest[5..];
        offset
    } else {
        "+0000".to_string()
    };

    let suffix = rest.trim_start();
    if suffix.starts_with('(') {
        format!("{date} {time} {offset} {}", normalize_text(suffix))
    } else {
        format!("{date} {time} {offset}")
    }
}

fn is_date_time_prefix(bytes: &[u8]) -> bool {
    bytes.len() >= 19
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && matches!(bytes[10], b'T' | b' ')
        && bytes[11..13].iter().all(u8::is_ascii_digit)
        && bytes[13] == b':'
        && bytes[14..16].iter().all(u8::is_ascii_digit)
        && bytes[16] == b':'
        && bytes[17..19].iter().all(u8::is_ascii_digit)
}

pub fn escape_nasl_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "'")
}

fn line_call(output: &mut String, function: &str, values: &[&str]) {
    let args = values
        .iter()
        .map(|value| format!("\"{}\"", escape_nasl_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    output.push_str(&format!("  {function}({args});\n"));
}

fn line_tag(output: &mut String, name: &str, value: &str) {
    output.push_str(&format!(
        "  script_tag(name:\"{}\", value:\"{}\");\n",
        escape_nasl_string(name),
        escape_nasl_string(value)
    ));
}

#[cfg(test)]
#[path = "nasl_tests.rs"]
mod nasl_tests;
