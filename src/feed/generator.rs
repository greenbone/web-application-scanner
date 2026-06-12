// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::{
    alert_doc::{AlertDoc, AlertId, AlertKind},
    nasl::{
        NaslMetadata, Tag, Xref, encode_oid, normalize_body_summary, normalize_text, render,
        solution_type,
    },
    validation::ValidationReport,
};

#[derive(Debug, Clone)]
pub struct GenerateConfig {
    pub version_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedVt {
    pub alert_id: AlertId,
    pub file_name: String,
    pub oid: String,
    pub rendered: String,
}

#[derive(Debug)]
struct Candidate<'a> {
    doc: &'a AlertDoc,
    parent: Option<&'a AlertDoc>,
    parent_derived: bool,
}

pub fn generate_vts(
    mut docs: Vec<AlertDoc>,
    config: &GenerateConfig,
) -> (Vec<GeneratedVt>, ValidationReport) {
    docs.sort_by(|left, right| left.alert_id.cmp(&right.alert_id));
    let mut report = ValidationReport::default();
    let mut by_id: BTreeMap<AlertId, &AlertDoc> = BTreeMap::new();
    let mut seen = BTreeSet::new();

    for doc in &docs {
        if !seen.insert(doc.alert_id.clone()) {
            report.error(
                doc.path.clone(),
                Some(doc.alert_id.to_string()),
                "duplicate alertid",
            );
            continue;
        }
        by_id.insert(doc.alert_id.clone(), doc);
    }

    if report.has_errors() {
        return (Vec::new(), report);
    }

    let mut child_count_by_base: HashMap<u32, usize> = HashMap::new();
    for doc in &docs {
        if doc.kind == AlertKind::Alert && doc.alert_id.sub.is_some() {
            *child_count_by_base.entry(doc.alert_id.base).or_default() += 1;
        }
    }

    let mut candidates = Vec::new();
    for doc in &docs {
        match doc.kind {
            AlertKind::Alert => {
                let parent = if doc.alert_id.sub.is_some() {
                    let parent = by_id.get(&doc.alert_id.parent_id()).copied();
                    if parent.is_none() {
                        report.warning(
                            doc.path.clone(),
                            Some(doc.alert_id.to_string()),
                            "missing parent alertset for leaf alert",
                        );
                    }
                    parent
                } else {
                    None
                };
                candidates.push(Candidate {
                    doc,
                    parent,
                    parent_derived: false,
                });
            }
            AlertKind::AlertSet => {
                if child_count_by_base
                    .get(&doc.alert_id.base)
                    .copied()
                    .unwrap_or_default()
                    == 0
                {
                    candidates.push(Candidate {
                        doc,
                        parent: None,
                        parent_derived: true,
                    });
                }
            }
        }
    }

    let mut generated = Vec::new();
    let mut oids = BTreeSet::new();
    for candidate in candidates {
        if let Some(vt) = generate_candidate(candidate, config, &mut report) {
            if !oids.insert(vt.oid.clone()) {
                report.error(
                    vt.file_name.clone(),
                    Some(vt.alert_id.to_string()),
                    format!("OID collision: {}", vt.oid),
                );
            } else {
                generated.push(vt);
            }
        }
    }

    if report.has_errors() {
        return (Vec::new(), report);
    }

    generated.sort_by(|left, right| left.alert_id.cmp(&right.alert_id));
    (generated, report)
}

fn generate_candidate(
    candidate: Candidate<'_>,
    config: &GenerateConfig,
    report: &mut ValidationReport,
) -> Option<GeneratedVt> {
    let doc = candidate.doc;
    let alert_id = doc.alert_id.to_string();
    let error_count_before = report.error_count();
    let title = title_for(doc, candidate.parent);
    if title.is_none() {
        report.error(
            doc.path.clone(),
            Some(alert_id.clone()),
            "missing title/name",
        );
    }

    validate_urls(doc, report);
    validate_required_fields(&candidate, report);

    let oid = encode_oid(&doc.alert_id);

    if report.error_count() > error_count_before {
        return None;
    }

    let title = title?;
    let mut xrefs = BTreeSet::new();
    insert_xref(&mut xrefs, "ZAP-Alert-ID", &alert_id);
    if let Some(value) = &doc.alert_index {
        insert_xref(&mut xrefs, "ZAP-Alert-Index", value);
    } else {
        report.warning(
            doc.path.clone(),
            Some(alert_id.clone()),
            "missing alertindex",
        );
    }
    let alert_type = doc.alert_type.as_deref().or_else(|| {
        candidate
            .parent
            .and_then(|parent| parent.alert_type.as_deref())
    });
    if let Some(alert_type) = alert_type {
        insert_xref(&mut xrefs, "ZAP-Alert-Type", alert_type);
        warn_unknown_alert_type(doc, alert_type, report);
    } else {
        report.warning(
            doc.path.clone(),
            Some(alert_id.clone()),
            "missing alerttype",
        );
    }
    let status = doc
        .status
        .as_deref()
        .or_else(|| candidate.parent.and_then(|parent| parent.status.as_deref()));
    if let Some(status) = status {
        insert_xref(&mut xrefs, "ZAP-Status", status);
    } else {
        report.warning(doc.path.clone(), Some(alert_id.clone()), "missing status");
    }
    if let Some(parent) = candidate.parent
        && let Some(title) = &parent.title
    {
        insert_xref(&mut xrefs, "ZAP-Alert-Set", title);
    }

    let mut cves = BTreeSet::new();
    add_structured_xrefs(doc, &mut xrefs, &mut cves, report);

    for reference in &doc.references {
        insert_xref(&mut xrefs, "URL", reference);
    }
    if let Some(help) = &doc.help {
        insert_xref(&mut xrefs, "URL", help);
    }
    let code = doc
        .code
        .as_deref()
        .or_else(|| candidate.parent.and_then(|parent| parent.code.as_deref()));
    if let Some(code) = code {
        insert_xref(&mut xrefs, "URL", code);
    }
    for tech_tag in &doc.tech_tags {
        insert_xref(&mut xrefs, "ZAP-Technology", tech_tag);
    }

    let mut tags = Vec::new();
    tags.push(Tag {
        name: "summary".to_string(),
        value: normalize_body_summary(&doc.body, &title),
    });
    if let Some(other) = &doc.other {
        let other = normalize_text(other);
        if !other.is_empty() {
            tags.push(Tag {
                name: "vuldetect".to_string(),
                value: other,
            });
        }
    }
    if let Some(solution) = &doc.solution {
        tags.push(Tag {
            name: "solution".to_string(),
            value: normalize_text(solution),
        });
    }
    if let Some(solution_type) = solution_type(status, doc.solution.as_deref()) {
        tags.push(Tag {
            name: "solution_type".to_string(),
            value: solution_type.to_string(),
        });
    }

    let metadata = NaslMetadata {
        oid: oid.clone(),
        version_date: config.version_date.clone(),
        creation_date: doc
            .date
            .clone()
            .unwrap_or_else(|| config.version_date.clone()),
        last_modification: doc
            .last_modification
            .clone()
            .unwrap_or_else(|| config.version_date.clone()),
        name: title,
        copyright_year: config.version_date.chars().take(4).collect(),
        cvss_base: doc.cvss_base.clone(),
        cvss_base_vector: doc.cvss_base_vector.clone(),
        severity_origin: doc.severity_origin.clone(),
        xrefs,
        cves,
        tags,
    };

    Some(GeneratedVt {
        alert_id: doc.alert_id.clone(),
        file_name: format!("{}.nasl", doc.alert_id.file_stem()),
        oid,
        rendered: render(&metadata),
    })
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

fn validate_required_fields(candidate: &Candidate<'_>, report: &mut ValidationReport) {
    let doc = candidate.doc;
    let alert_id = doc.alert_id.to_string();

    if doc.risk.is_none() {
        let source = if candidate.parent_derived {
            "parent-derived alertset VT"
        } else {
            "generated alert"
        };
        report.warning(
            doc.path.clone(),
            Some(alert_id.clone()),
            format!("missing risk for {source}"),
        );
    }

    if doc.solution.is_none() {
        let source = if candidate.parent_derived {
            "parent-derived alertset VT"
        } else {
            "generated alert"
        };
        report.warning(
            doc.path.clone(),
            Some(alert_id),
            format!("missing solution for {source}"),
        );
    }
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

fn add_structured_xrefs(
    doc: &AlertDoc,
    xrefs: &mut BTreeSet<Xref>,
    cves: &mut BTreeSet<String>,
    report: &mut ValidationReport,
) {
    if let Some(cwe) = &doc.cwe {
        let value = format_prefixed("CWE", cwe);
        insert_xref(xrefs, "CWE", &value);
    }
    if let Some(wasc) = &doc.wasc {
        let value = format_prefixed("WASC", wasc);
        insert_xref(xrefs, "WASC", &value);
    }

    for tag in &doc.alert_tags {
        if tag.starts_with("CVE-") {
            cves.insert(tag.clone());
        } else if tag.starts_with("CWE-") {
            insert_xref(xrefs, "CWE", tag);
        } else if tag.starts_with("WSTG-") {
            insert_xref(xrefs, "OWASP-WSTG", tag);
        } else if tag.starts_with("OWASP_2017_")
            || tag.starts_with("OWASP_2021_")
            || tag.starts_with("OWASP_2025_")
        {
            insert_xref(xrefs, "OWASP", tag);
        } else if tag.starts_with("API_2023_") || tag.starts_with("OWASP_2023_API") {
            insert_xref(xrefs, "OWASP-API", tag);
        } else if is_omitted_alert_tag(tag) {
        } else {
            report.warning(
                doc.path.clone(),
                Some(doc.alert_id.to_string()),
                format!("unknown alert tag: {tag}"),
            );
        }
    }
}

fn format_prefixed(prefix: &str, value: &str) -> String {
    if value.starts_with(prefix) {
        value.to_string()
    } else {
        format!("{prefix}-{value}")
    }
}

fn is_omitted_alert_tag(tag: &str) -> bool {
    tag == "PCI_DSS"
        || tag == "HIPAA"
        || tag == "CUSTOM_PAYLOADS"
        || tag == "OUT_OF_BAND"
        || tag == "TEST_TIMING"
        || tag == "SYSTEMIC"
        || tag == "TOOL_PTK"
        || tag.starts_with("POLICY_")
}

fn warn_unknown_alert_type(doc: &AlertDoc, alert_type: &str, report: &mut ValidationReport) {
    match alert_type {
        "Passive" | "Client Passive" | "WebSocket Passive" | "Script Passive" | "Active"
        | "Script Active" | "Script Httpsender" | "Tool" => {}
        _ => report.warning(
            doc.path.clone(),
            Some(doc.alert_id.to_string()),
            format!("unknown alerttype: {alert_type}"),
        ),
    }
}

fn insert_xref(xrefs: &mut BTreeSet<Xref>, name: &str, value: &str) {
    xrefs.insert(Xref {
        name: name.to_string(),
        value: normalize_text(value),
    });
}

#[cfg(test)]
#[path = "generator_tests.rs"]
mod generator_tests;
