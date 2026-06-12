// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::{
    alert_doc::{AlertDoc, AlertId, AlertKind},
    nasl::{encode_oid, normalize_text},
    validation::ValidationReport,
};

#[derive(Debug, Serialize)]
struct AlertIndex {
    schema_version: u8,
    alerts: Vec<AlertIndexEntry>,
}

#[derive(Debug, Serialize)]
struct AlertIndexEntry {
    id: String,
    oid: String,
    document_type: &'static str,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alert_index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alert_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    risk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cvss_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cvss_base_vector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    severity_origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    other: Option<String>,
    cwe: Vec<String>,
    cve: Vec<String>,
    wasc: Vec<String>,
    references: Vec<Reference>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tech_tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alert_tags: Option<AlertTags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    link_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_modified: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    child_alerts: Vec<ChildAlert>,
    source: Source,
    raw_frontmatter: serde_json::Value,
}

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Reference {
    #[serde(rename = "type")]
    kind: &'static str,
    value: String,
}

#[derive(Debug, Serialize)]
struct AlertTags {
    raw: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cwe: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cve: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    compliance: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owasp: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    policies: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    wstg: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    misc: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct ChildAlert {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct Source {
    path: String,
    alert_id: String,
}

pub fn generate_alert_index(docs: &[AlertDoc]) -> (Option<String>, ValidationReport) {
    let mut report = ValidationReport::default();
    let mut by_id: BTreeMap<AlertId, &AlertDoc> = BTreeMap::new();
    let mut oids = BTreeSet::new();

    for doc in docs {
        if by_id.insert(doc.alert_id.clone(), doc).is_some() {
            report.error(
                doc.path.clone(),
                Some(doc.alert_id.to_string()),
                "duplicate alertid",
            );
        }
        let oid = encode_oid(&doc.alert_id);
        if !oids.insert(oid.clone()) {
            report.error(
                doc.path.clone(),
                Some(doc.alert_id.to_string()),
                format!("OID collision: {oid}"),
            );
        }
    }

    if report.has_errors() {
        return (None, report);
    }

    let mut entries = Vec::new();
    for doc in docs {
        let parent = if doc.alert_id.sub.is_some() {
            by_id.get(&doc.alert_id.parent_id()).copied()
        } else {
            None
        };

        validate_urls(doc, &mut report);

        let Some(name) = title_for(doc, parent) else {
            report.error(
                doc.path.clone(),
                Some(doc.alert_id.to_string()),
                "missing title/name",
            );
            continue;
        };

        entries.push(entry_for(doc, parent, name));
    }

    if report.has_errors() {
        return (None, report);
    }

    entries.sort_by(|left, right| {
        left.id
            .parse::<AlertId>()
            .expect("entry ids are parsed before rendering")
            .cmp(
                &right
                    .id
                    .parse::<AlertId>()
                    .expect("entry ids are parsed before rendering"),
            )
    });

    match serde_json::to_string_pretty(&AlertIndex {
        schema_version: 1,
        alerts: entries,
    }) {
        Ok(mut json) => {
            json.push('\n');
            (Some(json), report)
        }
        Err(err) => {
            report.error(
                "alert-index.json",
                None,
                format!("JSON serialization failed: {err}"),
            );
            (None, report)
        }
    }
}

fn entry_for(doc: &AlertDoc, parent: Option<&AlertDoc>, name: String) -> AlertIndexEntry {
    let alert_tags = categorize_alert_tags(&doc.alert_tags);
    let cwe = merge_prefixed(doc.cwe.as_deref(), "CWE", &alert_tags.cwe);
    let cve = alert_tags.cve.clone();
    let wasc = merge_prefixed(doc.wasc.as_deref(), "WASC", &[]);
    let alert_tags = if alert_tags.raw.is_empty() {
        None
    } else {
        Some(alert_tags)
    };

    AlertIndexEntry {
        id: doc.alert_id.to_string(),
        oid: encode_oid(&doc.alert_id),
        document_type: match doc.kind {
            AlertKind::Alert => "alert",
            AlertKind::AlertSet => "alertset",
        },
        name,
        description: optional_normalized(&doc.body),
        alert_index: doc.alert_index.clone(),
        alert_type: doc
            .alert_type
            .clone()
            .or_else(|| parent.and_then(|parent| parent.alert_type.clone())),
        status: doc
            .status
            .clone()
            .or_else(|| parent.and_then(|parent| parent.status.clone())),
        risk: doc.risk.clone(),
        cvss_base: doc.cvss_base.clone(),
        cvss_base_vector: doc.cvss_base_vector.clone(),
        severity_origin: doc.severity_origin.clone(),
        solution: doc.solution.as_deref().and_then(optional_normalized),
        other: doc.other.as_deref().and_then(optional_normalized),
        cwe,
        cve,
        wasc,
        references: references(&doc.references),
        tech_tags: sorted_unique(doc.tech_tags.iter().cloned()),
        alert_tags,
        code: doc.code.clone(),
        help: doc.help.clone(),
        link_text: doc.link_text.clone(),
        date: doc.date.clone(),
        last_modified: doc.last_modification.clone(),
        child_alerts: child_alerts(doc),
        source: Source {
            path: doc.path.display().to_string(),
            alert_id: doc.alert_id.to_string(),
        },
        raw_frontmatter: doc.raw_frontmatter.clone(),
    }
}

fn title_for(doc: &AlertDoc, parent: Option<&AlertDoc>) -> Option<String> {
    doc.title
        .clone()
        .or_else(|| doc.name.clone())
        .or_else(|| {
            parent.and_then(|parent| parent.child_names.get(&doc.alert_id.to_string()).cloned())
        })
        .or_else(|| parent.and_then(|parent| parent.title.clone()))
}

fn optional_normalized(value: &str) -> Option<String> {
    let value = normalize_text(value);
    if value.is_empty() { None } else { Some(value) }
}

fn references(values: &[String]) -> Vec<Reference> {
    sorted_unique(values.iter().cloned())
        .into_iter()
        .map(|value| Reference { kind: "url", value })
        .collect()
}

fn child_alerts(doc: &AlertDoc) -> Vec<ChildAlert> {
    doc.child_names
        .iter()
        .map(|(id, name)| ChildAlert {
            id: id.clone(),
            name: name.clone(),
        })
        .collect()
}

fn validate_urls(doc: &AlertDoc, report: &mut ValidationReport) {
    let mut urls = doc.references.clone();
    urls.extend(doc.help.clone());
    urls.extend(doc.code.clone());
    for url in urls {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            report.error(
                doc.path.clone(),
                Some(doc.alert_id.to_string()),
                format!("invalid URL: {url}"),
            );
        }
    }
}

fn merge_prefixed(value: Option<&str>, prefix: &str, tags: &[String]) -> Vec<String> {
    let mut values = BTreeSet::new();
    if let Some(value) = value {
        values.insert(format_prefixed(prefix, value));
    }
    values.extend(tags.iter().cloned());
    values.into_iter().collect()
}

fn format_prefixed(prefix: &str, value: &str) -> String {
    if value.starts_with(prefix) {
        value.to_string()
    } else {
        format!("{prefix}-{value}")
    }
}

fn sorted_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn categorize_alert_tags(tags: &[String]) -> AlertTags {
    let raw = sorted_unique(tags.iter().cloned());
    let mut cwe = BTreeSet::new();
    let mut cve = BTreeSet::new();
    let mut compliance = BTreeSet::new();
    let mut owasp = BTreeSet::new();
    let mut policies = BTreeSet::new();
    let mut wstg = BTreeSet::new();
    let mut misc = BTreeSet::new();

    for tag in &raw {
        if tag.starts_with("CWE-") {
            cwe.insert(tag.clone());
        } else if tag.starts_with("CVE-") {
            cve.insert(tag.clone());
        } else if tag.starts_with("OWASP_2017_")
            || tag.starts_with("OWASP_2021_")
            || tag.starts_with("OWASP_2025_")
            || tag.starts_with("API_2023_")
            || tag.starts_with("OWASP_2023_API")
        {
            owasp.insert(tag.clone());
        } else if tag.starts_with("WSTG-") {
            wstg.insert(tag.clone());
        } else if tag.starts_with("POLICY_") {
            policies.insert(tag.clone());
        } else if matches!(tag.as_str(), "PCI_DSS" | "HIPAA") {
            compliance.insert(tag.clone());
        } else {
            misc.insert(tag.clone());
        }
    }

    AlertTags {
        raw,
        cwe: cwe.into_iter().collect(),
        cve: cve.into_iter().collect(),
        compliance: compliance.into_iter().collect(),
        owasp: owasp.into_iter().collect(),
        policies: policies.into_iter().collect(),
        wstg: wstg.into_iter().collect(),
        misc: misc.into_iter().collect(),
    }
}

#[cfg(test)]
#[path = "alert_index_tests.rs"]
mod alert_index_tests;
