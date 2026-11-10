//! The code report: findings with an edit an agent can make, what changed
//! against the before side, and what the run measured
//! (code-constraints 6.5, 7.1, 8.2, 8.3, 9.4, 13.1, 13.2).

use super::attribution::Attribution;
use super::facts::EntityKey;
use crate::finding::{Finding, Severity};

/// Version of the JSON report's shape.
pub const REPORT_SCHEMA_VERSION: u32 = 1;
/// Longest hook summary, in lines.
pub const SUMMARY_MAX_LINES: usize = 25;
/// Rows shown under "worsened" and under "improved", the "more" row included.
const SUMMARY_ROWS: usize = 9;

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
    /// The value on the before side, when the entity is paired.
    pub before: Option<f64>,
    pub limit: Option<f64>,
    pub limit_source: Option<String>,
    /// The breach existed before and did not worsen: reported, not blocking.
    pub pre_existing: bool,
    /// How the entity was paired with the before side.
    pub trust: Option<&'static str>,
    pub entity: Option<EntityKey>,
    pub owners: Attribution,
}

/// A routine metric that differs from the before side.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub entity: EntityKey,
    pub metric: String,
    pub before: f64,
    pub after: f64,
    pub limit: Option<f64>,
    pub line: u32,
}

impl Change {
    /// A lower value is an improvement for every ratcheted metric.
    #[must_use]
    pub fn improved(&self) -> bool {
        self.after < self.before
    }
}

/// Signed growth over the accounting scope.
#[derive(Debug, Clone, PartialEq)]
pub enum NetDelta {
    Known {
        statements: f64,
        code_lines: f64,
    },
    /// Why no total can be given. A missing side is never counted as zero.
    Unavailable(String),
}

fn signed(value: f64) -> String {
    if value > 0.0 {
        format!("+{}", number(value))
    } else {
        number(value)
    }
}

impl NetDelta {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Known {
                statements,
                code_lines,
            } => format!(
                "{} statements, {} lines",
                signed(*statements),
                signed(*code_lines)
            ),
            Self::Unavailable(reason) => format!("unavailable ({reason})"),
        }
    }
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

/// Highest value and total of one routine metric over the judged files of
/// one spec, for the trend of a code base.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    /// `None` for files no spec maps.
    pub spec: Option<String>,
    pub metric: String,
    pub highest: f64,
    pub total: f64,
    /// Number of routines measured.
    pub routines: usize,
}

/// The result of one code check.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeReport {
    pub findings: Vec<CodeFinding>,
    pub coverage: Coverage,
    pub specs: Vec<SpecSummary>,
    /// Paired routine metrics that differ from the before side.
    pub changes: Vec<Change>,
    pub net: NetDelta,
    /// Highest and total of every routine metric per spec.
    pub measures: Vec<Measure>,
}

impl CodeReport {
    /// Counts of (error, warning, info) findings, pre-existing included.
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

    /// The findings that may block: everything that is not pre-existing.
    pub fn blocking_findings(&self) -> impl Iterator<Item = &Finding> {
        self.findings
            .iter()
            .filter(|entry| !entry.pre_existing)
            .map(|entry| &entry.finding)
    }

    /// Number of pre-existing findings.
    #[must_use]
    pub fn pre_existing(&self) -> usize {
        self.findings
            .iter()
            .filter(|entry| entry.pre_existing)
            .count()
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

/// A number as it appears in messages: integers without a fraction.
#[must_use]
pub fn display_number(value: f64) -> String {
    number(value)
}

fn owners_text(owners: &Attribution) -> String {
    let Some(feature) = &owners.feature else {
        return String::new();
    };
    let mut out = format!(" [spec {feature}");
    if !owners.components.is_empty() {
        out.push_str(&format!(", component {}", owners.components.join(", ")));
    }
    if !owners.tasks.is_empty() {
        out.push_str(&format!(", tasks {}", owners.tasks.join(", ")));
    }
    if !owners.others.is_empty() {
        out.push_str(&format!(", also {}", owners.others.join(", ")));
    }
    out.push(']');
    out
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
        out.push_str(&format!(
            "    {}: {}{}\n",
            entry.tag.label(),
            entry.hint,
            owners_text(&entry.owners)
        ));
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
        "Code: {} of {} selected files measured: {errors} errors, {warnings} warnings, {info} info",
        report.coverage.measured, report.coverage.selected
    ));
    let pre_existing = report.pre_existing();
    if pre_existing > 0 {
        out.push_str(&format!(" ({pre_existing} pre-existing, not blocking)"));
    }
    out.push('\n');
    out
}

fn entity_json(entity: &EntityKey) -> serde_json::Value {
    serde_json::json!({
        "scope": entity.scope.label(),
        "path": entity.path,
        "qualified_name": entity.qualified_name,
        "parameters": entity.parameters,
        "ordinal": entity.ordinal,
    })
}

fn finding_json(entry: &CodeFinding) -> serde_json::Value {
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
        "before": entry.before,
        "limit": entry.limit,
        "limit_source": entry.limit_source,
        "pre_existing": entry.pre_existing,
        "match": entry.trust,
        "entity": entry.entity.as_ref().map(entity_json),
        "components": entry.owners.components,
        "tasks": entry.owners.tasks,
        "other_specs": entry.owners.others,
    })
}

fn spec_json(summary: &SpecSummary) -> serde_json::Value {
    serde_json::json!({
        "spec": summary.spec,
        "files": summary.files,
        "errors": summary.errors,
        "warnings": summary.warnings,
        "info": summary.info,
    })
}

fn change_json(change: &Change) -> serde_json::Value {
    serde_json::json!({
        "entity": entity_json(&change.entity),
        "metric": change.metric,
        "before": change.before,
        "after": change.after,
        "limit": change.limit,
        "improved": change.improved(),
    })
}

fn net_json(net: &NetDelta) -> serde_json::Value {
    match net {
        NetDelta::Known {
            statements,
            code_lines,
        } => serde_json::json!({ "statements": statements, "code_lines": code_lines }),
        NetDelta::Unavailable(reason) => serde_json::json!({ "unavailable": reason }),
    }
}

/// The report as a JSON document with a trailing newline. No timestamps
/// and no absolute paths; the order is the report's order.
#[must_use]
pub fn json(report: &CodeReport) -> String {
    let findings: Vec<serde_json::Value> = report.findings.iter().map(finding_json).collect();
    let specs: Vec<serde_json::Value> = report.specs.iter().map(spec_json).collect();
    let changes: Vec<serde_json::Value> = report.changes.iter().map(change_json).collect();
    let net = net_json(&report.net);
    let (errors, warnings, info) = report.counts();
    let document = serde_json::json!({
        "schema_version": REPORT_SCHEMA_VERSION,
        "findings": findings,
        "specs": specs,
        "changes": changes,
        "net": net,
        "coverage": {
            "selected": report.coverage.selected,
            "measured": report.coverage.measured,
            "failed": report.coverage.failed,
            "unmeasured": report.coverage.unmeasured,
            "ignored": report.coverage.ignored,
            "unowned": report.coverage.unowned,
            "before_side": report.coverage.before_side,
        },
        "summary": {
            "errors": errors,
            "warnings": warnings,
            "info": info,
            "pre_existing": report.pre_existing(),
        },
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

/// Printable ASCII only: anything else becomes `?`.
fn ascii(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .collect()
}

fn change_rows(changes: &[&Change]) -> Vec<String> {
    let mut rows: Vec<String> = changes
        .iter()
        .map(|change| {
            let limit = change
                .limit
                .map(|limit| format!(" (limit {})", number(limit)))
                .unwrap_or_default();
            ascii(&format!(
                "  {}:{} {} {} {} -> {}{limit}",
                change.entity.path,
                change.line,
                change.entity.qualified_name,
                change.metric,
                number(change.before),
                number(change.after)
            ))
        })
        .collect();
    if rows.len() > SUMMARY_ROWS {
        let shown = SUMMARY_ROWS.saturating_sub(1);
        let hidden = rows.len().saturating_sub(shown);
        rows.truncate(shown);
        rows.push(format!("  ... and {hidden} more"));
    }
    rows
}

/// The hook summary: what the change improved and worsened, in at most
/// [`SUMMARY_MAX_LINES`] lines of printable ASCII in a fixed order, so that
/// it reads the same in a pre-commit or pre-push hook on every platform.
///
/// `verdict` is `PASS` or `FAIL`; `fail_on` is the threshold word;
/// `blocking` is the number of findings at or above the threshold.
#[must_use]
pub fn summary(report: &CodeReport, verdict: &str, fail_on: &str, blocking: usize) -> String {
    let (_, warnings, _) = report.counts();
    let mut lines = vec![
        ascii(&format!("kiro code check: {verdict} (fail-on {fail_on})")),
        ascii(&format!("before side: {}", report.coverage.before_side)),
        format!(
            "findings: {blocking} blocking, {warnings} warnings, {} pre-existing",
            report.pre_existing()
        ),
    ];
    let worsened: Vec<&Change> = report
        .changes
        .iter()
        .filter(|change| !change.improved())
        .collect();
    let improved: Vec<&Change> = report
        .changes
        .iter()
        .filter(|change| change.improved())
        .collect();
    lines.push(format!("worsened: {}", worsened.len()));
    lines.extend(change_rows(&worsened));
    lines.push(format!("improved: {}", improved.len()));
    lines.extend(change_rows(&improved));
    lines.push(ascii(&format!("net: {}", report.net.label())));
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
