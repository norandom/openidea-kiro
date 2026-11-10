//! The code report: findings with an edit an agent can make, and what the
//! run measured (code-constraints 6.5, 7.1, 8.2, 8.3, 9.4).

use super::attribution::Attribution;
use super::facts::EntityKey;
use crate::finding::{Finding, Severity};

/// Version of the JSON report's shape.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// The kind of edit a hint asks for. Exactly one opens every hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
    Simplify,
    Split,
    Delete,
    Shrink,
    Collect,
    Name,
    /// Reserved for findings about the run itself.
    Fix,
}

impl Tag {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Simplify => "simplify",
            Self::Split => "split",
            Self::Delete => "delete",
            Self::Shrink => "shrink",
            Self::Collect => "collect",
            Self::Name => "name",
            Self::Fix => "fix",
        }
    }
}

/// A finding of the code gate.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeFinding {
    /// The base finding: severity, code, feature, file, line, message.
    pub finding: Finding,
    pub tag: Tag,
    /// What to change and what replaces it, in one line, without the tag.
    pub hint: String,
    pub metric: Option<String>,
    pub value: Option<f64>,
    pub limit: Option<f64>,
    pub limit_source: Option<String>,
    pub entity: Option<EntityKey>,
    pub owners: Attribution,
}

/// What the run measured and what it left out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Coverage {
    /// Files in the selection.
    pub selected: usize,
    /// Selected files that were measured completely.
    pub measured: usize,
    /// Selected files that do not parse.
    pub failed: Vec<String>,
    /// Selected files in a language no profile covers.
    pub unmeasured: Vec<String>,
    /// Selected files the configuration excludes from judgement.
    pub ignored: Vec<String>,
    /// Selected files mapped by no spec.
    pub unowned: usize,
    /// The side the ratchet compares with, or why there is none.
    pub before_side: String,
}

/// Findings per spec, in roadmap order; unowned files last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSummary {
    /// `None` for files no spec maps.
    pub spec: Option<String>,
    pub files: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

/// The result of one code check.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeReport {
    pub findings: Vec<CodeFinding>,
    pub coverage: Coverage,
    pub specs: Vec<SpecSummary>,
}

impl CodeReport {
    /// Counts of (error, warning, info) findings.
    #[must_use]
    pub fn counts(&self) -> (usize, usize, usize) {
        let count = |severity: Severity| {
            self.findings
                .iter()
                .filter(|entry| entry.finding.severity == severity)
                .count()
        };
        (
            count(Severity::Error),
            count(Severity::Warning),
            count(Severity::Info),
        )
    }

    /// The base findings, for the shared verdict.
    pub fn base_findings(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter().map(|entry| &entry.finding)
    }
}

fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "ERROR",
        Severity::Warning => "WARNING",
        Severity::Info => "INFO",
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

/// The report as text: one finding per two lines, then the summaries.
#[must_use]
pub fn text(report: &CodeReport) -> String {
    let mut out = String::new();
    for entry in &report.findings {
        let finding = &entry.finding;
        match finding.line {
            Some(line) => out.push_str(&format!("{}:{line}: ", finding.file)),
            None => out.push_str(&format!("{}: ", finding.file)),
        }
        out.push_str(&format!(
            "{} {} {}\n",
            severity_word(finding.severity),
            finding.code,
            finding.message
        ));
        out.push_str(&format!("    {}: {}", entry.tag.label(), entry.hint));
        if let Some(feature) = &entry.owners.feature {
            out.push_str(&format!(" [spec {feature}"));
            if !entry.owners.components.is_empty() {
                out.push_str(&format!(
                    ", component {}",
                    entry.owners.components.join(", ")
                ));
            }
            if !entry.owners.tasks.is_empty() {
                out.push_str(&format!(", tasks {}", entry.owners.tasks.join(", ")));
            }
            if !entry.owners.others.is_empty() {
                out.push_str(&format!(", also {}", entry.owners.others.join(", ")));
            }
            out.push(']');
        }
        out.push('\n');
    }
    if !report.findings.is_empty() {
        out.push('\n');
    }
    for summary in &report.specs {
        let name = summary.spec.as_deref().unwrap_or("(no spec)");
        out.push_str(&format!(
            "{name}: {} files, {} errors, {} warnings, {} info\n",
            summary.files, summary.errors, summary.warnings, summary.info
        ));
    }
    let (errors, warnings, info) = report.counts();
    out.push_str(&format!(
        "Code: {} of {} selected files measured: {errors} errors, {warnings} warnings, {info} info\n",
        report.coverage.measured, report.coverage.selected
    ));
    out
}

/// The report as a JSON document with a trailing newline. No timestamps
/// and no absolute paths; the order is the report's order.
#[must_use]
pub fn json(report: &CodeReport) -> String {
    let findings: Vec<serde_json::Value> = report
        .findings
        .iter()
        .map(|entry| {
            let finding = &entry.finding;
            serde_json::json!({
                "severity": severity_label(finding.severity),
                "code": finding.code,
                "feature": finding.feature,
                "file": finding.file,
                "line": finding.line,
                "message": finding.message,
                "tag": entry.tag.label(),
                "hint": entry.hint,
                "metric": entry.metric,
                "value": entry.value,
                "before": serde_json::Value::Null,
                "limit": entry.limit,
                "limit_source": entry.limit_source,
                "pre_existing": false,
                "entity": entry.entity.as_ref().map(|entity| serde_json::json!({
                    "scope": entity.scope.label(),
                    "path": entity.path,
                    "qualified_name": entity.qualified_name,
                    "parameters": entity.parameters,
                    "ordinal": entity.ordinal,
                })),
                "components": entry.owners.components,
                "tasks": entry.owners.tasks,
                "other_specs": entry.owners.others,
            })
        })
        .collect();
    let specs: Vec<serde_json::Value> = report
        .specs
        .iter()
        .map(|summary| {
            serde_json::json!({
                "spec": summary.spec,
                "files": summary.files,
                "errors": summary.errors,
                "warnings": summary.warnings,
                "info": summary.info,
            })
        })
        .collect();
    let (errors, warnings, info) = report.counts();
    let document = serde_json::json!({
        "schema_version": REPORT_SCHEMA_VERSION,
        "findings": findings,
        "specs": specs,
        "coverage": {
            "selected": report.coverage.selected,
            "measured": report.coverage.measured,
            "failed": report.coverage.failed,
            "unmeasured": report.coverage.unmeasured,
            "unowned": report.coverage.unowned,
            "ignored": report.coverage.ignored,
            "before_side": report.coverage.before_side,
        },
        "summary": { "errors": errors, "warnings": warnings, "info": info },
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

/// A number as it appears in messages: integers without a fraction.
#[must_use]
pub fn display_number(value: f64) -> String {
    number(value)
}
