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
        &normalize_text(&metadata.creation_date),
    );
    line_tag(
        &mut output,
        "last_modification",
        &normalize_text(&metadata.last_modification),
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
mod tests {
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
        let normalized = normalize_text("Use [docs](https://example.com).\n• path C:\\tmp");
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
}
