// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub path: PathBuf,
    pub alert_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn error(
        &mut self,
        path: impl Into<PathBuf>,
        alert_id: Option<String>,
        message: impl Into<String>,
    ) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            path: path.into(),
            alert_id,
            message: message.into(),
        });
    }

    pub fn warning(
        &mut self,
        path: impl Into<PathBuf>,
        alert_id: Option<String>,
        message: impl Into<String>,
    ) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            path: path.into(),
            alert_id,
            message: message.into(),
        });
    }

    pub fn extend(&mut self, other: ValidationReport) {
        self.diagnostics.extend(other.diagnostics);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count()
    }

    #[cfg(test)]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn print(&self) {
        for diagnostic in &self.diagnostics {
            let severity = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            match &diagnostic.alert_id {
                Some(alert_id) => eprintln!(
                    "{severity}: {} [{alert_id}]: {}",
                    diagnostic.path.display(),
                    diagnostic.message
                ),
                None => eprintln!(
                    "{severity}: {}: {}",
                    diagnostic.path.display(),
                    diagnostic.message
                ),
            }
        }
    }
}
