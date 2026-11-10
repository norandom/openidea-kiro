//! The code gate's pure policy (spec `code-constraints`).
//!
//! Native measurement lives in the `kiro-code` crate and hands over
//! [`CodeFacts`]. Everything here is pure: limits, ownership, report. The
//! engine reads no file and parses no source code.
//!
//! Evaluation order: parse failures of selected files, then limits, then
//! ownership, then the unowned and coverage findings.

pub mod attribution;
pub mod config;
pub mod facts;
pub mod limits;
pub mod report;

use std::collections::{BTreeMap, BTreeSet};

pub use attribution::{
    Attribution, SelectionError, attribute, matches_any, spec_paths, task_paths,
};
pub use config::{CodeConfig, Limit, LimitSource};
pub use facts::{
    CodeFacts, Completeness, CoverageCell, EntityKey, FACTS_SCHEMA_VERSION, MetricId, Observation,
    ProducerId, Scope, metrics,
};
pub use limits::{Judgement, highest, judge};
pub use report::{CodeFinding, CodeReport, Coverage, SpecSummary, Tag};

use crate::finding::{Finding, Severity, codes};
use crate::source::Feature;

/// Everything one evaluation needs.
pub struct EvaluateInput<'a> {
    pub facts: &'a CodeFacts,
    pub config: &'a CodeConfig,
    /// Project-relative paths of the selected files, sorted.
    pub selected: &'a [String],
    /// Loaded specs in precedence (roadmap) order.
    pub specs: &'a [&'a Feature],
}

fn limit_hint(scope: Scope, metric: &str, limit: f64, owners: &Attribution) -> (Tag, String) {
    let limit = report::display_number(limit);
    let plan = match owners.components.first() {
        Some(component) => {
            format!("; change the File Structure Plan of component {component} first")
        }
        None => String::new(),
    };
    match (scope, metric) {
        (Scope::Routine, metrics::CYCLOMATIC) => (
            Tag::Simplify,
            format!(
                "return early with guard clauses or extract a routine until cyclomatic complexity is at most {limit}"
            ),
        ),
        (Scope::Routine, metrics::NESTING) => (
            Tag::Simplify,
            format!("return early or extract the inner block until nesting is at most {limit}"),
        ),
        (Scope::Routine, metrics::CODE_LINES | metrics::STATEMENTS) => (
            Tag::Simplify,
            format!(
                "extract a cohesive part into its own routine until the count is at most {limit}"
            ),
        ),
        (Scope::Routine, metrics::PARAMETERS) => (
            Tag::Simplify,
            format!(
                "group related parameters into one options record until at most {limit} remain"
            ),
        ),
        (Scope::Routine, metrics::LINES_PER_STATEMENT) => (
            Tag::Shrink,
            format!("write each statement on fewer lines until the average is at most {limit}"),
        ),
        (Scope::Type, _) => (
            Tag::Split,
            format!(
                "move a cohesive group of members into its own type until at most {limit} remain{plan}"
            ),
        ),
        (Scope::File, metrics::MAX_CYCLOMATIC) => (
            Tag::Simplify,
            format!(
                "simplify the most complex routine of this file until its cyclomatic complexity is at most {limit}"
            ),
        ),
        _ => (
            Tag::Split,
            format!(
                "move a cohesive group of declarations into its own file until the count is at most {limit}{plan}"
            ),
        ),
    }
}

fn limit_code(scope: Scope) -> &'static str {
    match scope {
        Scope::Routine => codes::CODE_ROUTINE_LIMIT,
        Scope::Type => codes::CODE_TYPE_LIMIT,
        Scope::File => codes::CODE_FILE_LIMIT,
    }
}

fn with_feature(finding: Finding, owners: &Attribution) -> Finding {
    match &owners.feature {
        Some(feature) => finding.feature(feature.clone()),
        None => finding,
    }
}

fn line_of(finding: Finding, line: u32) -> Finding {
    let mut finding = finding;
    finding.line = usize::try_from(line).ok().filter(|line| *line > 0);
    finding
}

/// Evaluates the facts of a run into the code report.
#[must_use]
pub fn evaluate(input: &EvaluateInput<'_>) -> CodeReport {
    let selected: BTreeSet<&str> = input.selected.iter().map(String::as_str).collect();
    let is_selected = |path: &str| selected.contains(path);
    let mut findings: Vec<CodeFinding> = Vec::new();
    let mut coverage = Coverage {
        selected: input.selected.len(),
        before_side: "none: no baseline and no revision given, so nothing is ratcheted".to_string(),
        ..Coverage::default()
    };

    // Ownership and file state are resolved once per selected file.
    let ownership: BTreeMap<&str, Attribution> = input
        .selected
        .iter()
        .map(|path| (path.as_str(), attribute(path, input.specs)))
        .collect();
    let owners_of = |path: &str| ownership.get(path).cloned().unwrap_or_default();
    let states: BTreeMap<&str, &CoverageCell> = input
        .facts
        .coverage
        .iter()
        .filter(|cell| cell.metric.is_none())
        .map(|cell| (cell.path.as_str(), cell))
        .collect();

    // 1. Selected files that do not parse, or that no profile covers.
    for path in input.selected {
        let owners = owners_of(path);
        if owners.feature.is_none() {
            coverage.unowned = coverage.unowned.saturating_add(1);
        }
        match states.get(path.as_str()).copied() {
            Some(cell) if cell.state == Completeness::Failed => {
                coverage.failed.push(path.clone());
                let reason = cell.reason.as_deref().unwrap_or("syntax error");
                let finding = Finding::new(
                    codes::CODE_PARSE_FAILED,
                    path.clone(),
                    format!("{reason}; the file cannot be reported as checked"),
                );
                findings.push(CodeFinding {
                    finding: with_feature(line_of(finding, cell.line.unwrap_or(0)), &owners),
                    tag: Tag::Fix,
                    hint: "repair the syntax so the file parses, then run the check again"
                        .to_string(),
                    metric: None,
                    value: None,
                    limit: None,
                    limit_source: None,
                    entity: None,
                    owners,
                });
            }
            Some(cell) if cell.state == Completeness::Complete => {
                coverage.measured = coverage.measured.saturating_add(1);
            }
            _ => coverage.unmeasured.push(path.clone()),
        }
    }

    // 2. Limits, with ownership.
    for judgement in judge(input.facts, input.config, &is_selected) {
        let owners = owners_of(&judgement.entity.path);
        let (tag, hint) = limit_hint(
            judgement.entity.scope,
            &judgement.metric,
            judgement.limit,
            &owners,
        );
        let subject = match judgement.entity.scope {
            Scope::File => "file".to_string(),
            scope => format!("{} `{}`", scope.label(), judgement.entity.qualified_name),
        };
        let finding = Finding::new(
            limit_code(judgement.entity.scope),
            judgement.entity.path.clone(),
            format!(
                "{subject}: {} is {}, limit {} ({})",
                judgement.metric,
                report::display_number(judgement.value),
                report::display_number(judgement.limit),
                judgement.limit_source.label()
            ),
        )
        .severity(judgement.severity);
        findings.push(CodeFinding {
            finding: with_feature(line_of(finding, judgement.line), &owners),
            tag,
            hint,
            metric: Some(judgement.metric),
            value: Some(judgement.value),
            limit: Some(judgement.limit),
            limit_source: Some(judgement.limit_source.label()),
            entity: Some(judgement.entity),
            owners,
        });
    }

    // 3. Selected files with a finding that no spec maps, once per file.
    let unowned: BTreeSet<String> = findings
        .iter()
        .filter(|entry| entry.owners.feature.is_none())
        .map(|entry| entry.finding.file.clone())
        .collect();
    for path in unowned {
        findings.push(CodeFinding {
            finding: Finding::new(
                codes::CODE_FILE_UNOWNED,
                path,
                "the file has findings and is mapped by no spec's File Structure Plan",
            ),
            tag: Tag::Fix,
            hint: "add the file to the File Structure Plan of the design that owns it".to_string(),
            metric: None,
            value: None,
            limit: None,
            limit_source: None,
            entity: None,
            owners: Attribution::default(),
        });
    }

    findings.sort_by(|a, b| {
        (&a.finding.file, a.finding.line, a.finding.code, &a.metric).cmp(&(
            &b.finding.file,
            b.finding.line,
            b.finding.code,
            &b.metric,
        ))
    });

    // 4. Per-spec summary in precedence order, unowned last.
    let mut specs: Vec<SpecSummary> = Vec::new();
    let mut names: Vec<Option<String>> = input
        .specs
        .iter()
        .map(|feature| Some(feature.name.clone()))
        .collect();
    names.push(None);
    for name in names {
        let files = input
            .selected
            .iter()
            .filter(|path| {
                ownership
                    .get(path.as_str())
                    .is_some_and(|owners| owners.feature == name)
            })
            .count();
        let count = |severity: Severity| {
            findings
                .iter()
                .filter(|entry| entry.owners.feature == name && entry.finding.severity == severity)
                .count()
        };
        if files > 0 {
            specs.push(SpecSummary {
                spec: name.clone(),
                files,
                errors: count(Severity::Error),
                warnings: count(Severity::Warning),
                info: count(Severity::Info),
            });
        }
    }

    // 5. What the run measured: always reported, so a clean report is never
    //    mistaken for a checked one.
    let message = format!(
        "{} files selected: {} measured, {} failed to parse, {} not measured (no profile for the language), {} mapped by no spec; before side: {}",
        coverage.selected,
        coverage.measured,
        coverage.failed.len(),
        coverage.unmeasured.len(),
        coverage.unowned,
        coverage.before_side
    );
    findings.push(CodeFinding {
        finding: Finding::new(codes::CODE_COVERAGE, ".kiro/code-rules.toml", message),
        tag: Tag::Fix,
        hint: "read this line before trusting a clean report; unmeasured files were not judged"
            .to_string(),
        metric: None,
        value: None,
        limit: None,
        limit_source: None,
        entity: None,
        owners: Attribution::default(),
    });

    CodeReport {
        findings,
        coverage,
        specs,
    }
}
