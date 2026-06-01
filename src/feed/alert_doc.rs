// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::BTreeMap, fmt, path::Path, path::PathBuf, str::FromStr};

use serde::Deserialize;
use serde_yaml::Value;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlertId {
    pub base: u32,
    pub sub: Option<u32>,
}

impl AlertId {
    pub fn parent_id(&self) -> AlertId {
        AlertId {
            base: self.base,
            sub: None,
        }
    }

    pub fn file_stem(&self) -> String {
        match self.sub {
            Some(sub) => format!("zap_{}_{}", self.base, sub),
            None => format!("zap_{}", self.base),
        }
    }
}

impl fmt::Display for AlertId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.sub {
            Some(sub) => write!(f, "{}-{}", self.base, sub),
            None => write!(f, "{}", self.base),
        }
    }
}

impl FromStr for AlertId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err("alertid is empty".to_string());
        }
        let mut parts = value.split('-');
        let base = parse_id_part(parts.next().unwrap_or_default(), "base alert id")?;
        let sub = match parts.next() {
            Some(part) => Some(parse_id_part(part, "sub alert id")?),
            None => None,
        };
        if parts.next().is_some() {
            return Err(format!("invalid alertid format: {value}"));
        }
        Ok(AlertId { base, sub })
    }
}

fn parse_id_part(value: &str, name: &str) -> Result<u32, String> {
    if value.is_empty() || !value.chars().all(|character| character.is_ascii_digit()) {
        return Err(format!("{name} must be numeric"));
    }
    value
        .parse()
        .map_err(|_| format!("{name} is too large: {value}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertKind {
    Alert,
    AlertSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertDoc {
    pub path: PathBuf,
    pub kind: AlertKind,
    pub alert_id: AlertId,
    pub title: Option<String>,
    pub name: Option<String>,
    pub alert_index: Option<String>,
    pub alert_type: Option<String>,
    pub status: Option<String>,
    pub risk: Option<String>,
    pub cvss_base: Option<String>,
    pub cvss_base_vector: Option<String>,
    pub severity_origin: Option<String>,
    pub solution: Option<String>,
    pub references: Vec<String>,
    pub other: Option<String>,
    pub cwe: Option<String>,
    pub wasc: Option<String>,
    pub alert_tags: Vec<String>,
    pub tech_tags: Vec<String>,
    pub code: Option<String>,
    pub help: Option<String>,
    pub link_text: Option<String>,
    pub date: Option<String>,
    pub last_modification: Option<String>,
    pub child_names: BTreeMap<String, String>,
    pub body: String,
}

#[derive(Debug, Deserialize)]
struct Frontmatter {
    #[serde(rename = "type")]
    kind: Option<String>,
    title: Option<String>,
    name: Option<String>,
    alertid: Option<Value>,
    alertindex: Option<Value>,
    alerttype: Option<String>,
    status: Option<String>,
    risk: Option<String>,
    #[serde(alias = "cvssbase")]
    cvss_base: Option<Value>,
    #[serde(alias = "cvssbasevector")]
    cvss_base_vector: Option<String>,
    severity_origin: Option<String>,
    solution: Option<String>,
    references: Option<Value>,
    other: Option<String>,
    cwe: Option<Value>,
    wasc: Option<Value>,
    alerttags: Option<Vec<String>>,
    techtags: Option<Vec<String>>,
    code: Option<String>,
    help: Option<String>,
    linktext: Option<String>,
    date: Option<String>,
    lastmod: Option<String>,
    alerts: Option<BTreeMap<String, ChildAlert>>,
}

#[derive(Debug, Deserialize)]
struct ChildAlert {
    alertid: Option<Value>,
    name: Option<String>,
}

pub fn parse_alert_doc(path: &Path, content: &str) -> Result<Option<AlertDoc>, String> {
    if path.file_name().and_then(|name| name.to_str()) == Some("_index.md") {
        return Ok(None);
    }

    let (frontmatter, body) = split_frontmatter(content)?;
    let raw: Frontmatter =
        serde_yaml::from_str(frontmatter).map_err(|err| format!("malformed YAML: {err}"))?;
    let kind = match raw.kind.as_deref() {
        Some("alert") => AlertKind::Alert,
        Some("alertset") => AlertKind::AlertSet,
        Some(other) => return Err(format!("unsupported document type: {other}")),
        None => return Err("missing document type".to_string()),
    };

    let alert_id = value_to_string(raw.alertid.as_ref())
        .ok_or_else(|| "missing alertid".to_string())?
        .parse::<AlertId>()?;

    let mut child_names = BTreeMap::new();
    for (key, child) in raw.alerts.unwrap_or_default() {
        let child_id = value_to_string(child.alertid.as_ref()).unwrap_or(key);
        if let Some(name) = child.name {
            child_names.insert(child_id, name);
        }
    }

    Ok(Some(AlertDoc {
        path: path.to_path_buf(),
        kind,
        alert_id,
        title: opt_empty_to_none(raw.title),
        name: opt_empty_to_none(raw.name),
        alert_index: value_to_string(raw.alertindex.as_ref()),
        alert_type: opt_empty_to_none(raw.alerttype),
        status: opt_empty_to_none(raw.status),
        risk: opt_empty_to_none(raw.risk),
        cvss_base: value_to_string(raw.cvss_base.as_ref()).and_then(empty_to_none),
        cvss_base_vector: opt_empty_to_none(raw.cvss_base_vector),
        severity_origin: opt_empty_to_none(raw.severity_origin),
        solution: opt_empty_to_none(raw.solution),
        references: value_to_strings(raw.references.as_ref(), "references")?,
        other: opt_empty_to_none(raw.other),
        cwe: value_to_string(raw.cwe.as_ref()).and_then(empty_to_none),
        wasc: value_to_string(raw.wasc.as_ref()).and_then(empty_to_none),
        alert_tags: raw.alerttags.unwrap_or_default(),
        tech_tags: raw.techtags.unwrap_or_default(),
        code: opt_empty_to_none(raw.code),
        help: opt_empty_to_none(raw.help),
        link_text: opt_empty_to_none(raw.linktext),
        date: opt_empty_to_none(raw.date),
        last_modification: opt_empty_to_none(raw.lastmod),
        child_names,
        body: body.trim().to_string(),
    }))
}

fn split_frontmatter(content: &str) -> Result<(&str, &str), String> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err("missing YAML frontmatter delimiter".to_string());
    }

    let frontmatter_start = content
        .find('\n')
        .ok_or_else(|| "missing YAML frontmatter body".to_string())?
        + 1;
    let mut offset = frontmatter_start;
    for line in content[frontmatter_start..].lines() {
        if line.trim() == "---" {
            let frontmatter = &content[frontmatter_start..offset];
            let body_start = offset + line.len();
            let body_start = if content[body_start..].starts_with("\r\n") {
                body_start + 2
            } else if content[body_start..].starts_with('\n') {
                body_start + 1
            } else {
                body_start
            };
            return Ok((frontmatter, &content[body_start..]));
        }
        offset += line.len();
        if content[offset..].starts_with("\r\n") {
            offset += 2;
        } else if content[offset..].starts_with('\n') {
            offset += 1;
        }
    }

    Err("missing closing YAML frontmatter delimiter".to_string())
}

fn value_to_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_to_strings(value: Option<&Value>, field: &str) -> Result<Vec<String>, String> {
    match value {
        Some(Value::Sequence(values)) => {
            let mut output = Vec::new();
            for (index, value) in values.iter().enumerate() {
                let Some(value) = value_to_string(Some(value)) else {
                    return Err(format!("{field}[{index}] must be a scalar value"));
                };
                output.push(value);
            }
            Ok(output)
        }
        Some(value) => value_to_string(Some(value))
            .map(|value| vec![value])
            .ok_or_else(|| format!("{field} must be a scalar value or list of scalar values")),
        None => Ok(Vec::new()),
    }
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn opt_empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(empty_to_none)
}

#[cfg(test)]
mod tests {
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
}
