//! Evaluation of one run into the code report.
//!
//! Order: parse failures of selected files; limits, classified against the
//! before side when there is one; regressions within the limits; unowned
//! files; per-spec summary; coverage. Only regression and new breaches
//! block: a breach that existed before and did not worsen is reported as
//! pre-existing.

use std::collections::{BTreeMap, BTreeSet};

use super::attribution::{Attribution, attribute};
use super::config::CodeConfig;
use super::facts::{CodeFacts, Completeness, CoverageCell, EntityKey, Scope, metrics};
use super::limits::{Judgement, judge};
use super::ratchet::{Before, Earlier, Pairing, pair};
use super::report::{
    self, Change, CodeFinding, CodeReport, Coverage, Measure, NetDelta, SpecSummary, Tag,
};
use crate::finding::{Finding, Severity, codes};
use crate::source::Feature;

/// Routine metrics the ratchet compares. Container counts and the derived
/// lines per statement are judged on their limits only.
const RATCHETED: &[&str] = &[
    metrics::CYCLOMATIC,
    metrics::NESTING,
    metrics::CODE_LINES,
    metrics::STATEMENTS,
    metrics::PARAMETERS,
];
/// Size counts whose growth is excused when control flow got simpler.
const COUNTS: &[&str] = &[metrics::CODE_LINES, metrics::STATEMENTS];
/// The control-flow metrics that can excuse such growth.
const CONTROL_FLOW: &[&str] = &[metrics::CYCLOMATIC, metrics::NESTING];

/// Everything one evaluation needs.
pub struct EvaluateInput<'a> {
    pub facts: &'a CodeFacts,
    pub config: &'a CodeConfig,
    /// Project-relative paths of the selected files, sorted.
    pub selected: &'a [String],
    /// Loaded specs in precedence (roadmap) order.
    pub specs: &'a [&'a Feature],
    /// The side to compare with, when there is one.
    pub before: Option<Before<'a>>,
    /// Paths of the before side that belong to the accounting scope: the
    /// old paths of the selected files plus deleted files.
    pub before_paths: &'a [String],
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

fn subject(entity: &EntityKey) -> String {
    match entity.scope {
        Scope::File => "file".to_string(),
        scope => format!("{} `{}`", scope.label(), entity.qualified_name),
    }
}

fn located(finding: Finding, line: u32, owners: &Attribution) -> Finding {
    let mut finding = match &owners.feature {
        Some(feature) => finding.feature(feature.clone()),
        None => finding,
    };
    finding.line = usize::try_from(line).ok().filter(|line| *line > 0);
    finding
}

fn plain(finding: Finding, tag: Tag, hint: &str, owners: Attribution) -> CodeFinding {
    CodeFinding {
        finding,
        tag,
        hint: hint.to_string(),
        metric: None,
        value: None,
        before: None,
        limit: None,
        limit_source: None,
        pre_existing: false,
        trust: None,
        entity: None,
        owners,
    }
}

/// State shared by the steps of one evaluation.
struct Run<'a> {
    input: &'a EvaluateInput<'a>,
    ownership: BTreeMap<&'a str, Attribution>,
    pairing: Pairing,
    findings: Vec<CodeFinding>,
    coverage: Coverage,
}

impl<'a> Run<'a> {
    fn owners(&self, path: &str) -> Attribution {
        self.ownership.get(path).cloned().unwrap_or_default()
    }

    fn judged(&self, path: &str) -> bool {
        self.ownership.contains_key(path)
            && !self.coverage.ignored.iter().any(|ignored| ignored == path)
    }

    /// Whether growth of a size count of `entity` is excused: at least one
    /// control-flow metric improved and none worsened. A missing value
    /// excuses nothing.
    fn decomposed(&self, entity: &EntityKey) -> bool {
        let mut improved = false;
        for metric in CONTROL_FLOW {
            let (Some(now), Some(earlier)) = (
                self.input.facts.value(entity, metric),
                self.pairing.earlier(entity, metric),
            ) else {
                return false;
            };
            if now > earlier.value {
                return false;
            }
            if now < earlier.value {
                improved = true;
            }
        }
        improved
    }

    /// Whether the value is an eligible regression against `earlier`.
    fn regressed(&self, entity: &EntityKey, metric: &str, value: f64, earlier: Earlier) -> bool {
        entity.scope == Scope::Routine
            && RATCHETED.contains(&metric)
            && value > earlier.value
            && !(COUNTS.contains(&metric) && self.decomposed(entity))
    }

    /// Step 1: selected files that do not parse, or that no profile covers.
    fn file_states(&mut self) {
        let states: BTreeMap<&str, &CoverageCell> = self
            .input
            .facts
            .coverage
            .iter()
            .filter(|cell| cell.metric.is_none())
            .map(|cell| (cell.path.as_str(), cell))
            .collect();
        for path in self.input.selected {
            let owners = self.owners(path);
            if owners.feature.is_none() {
                self.coverage.unowned = self.coverage.unowned.saturating_add(1);
            }
            match states.get(path.as_str()).copied() {
                Some(cell) if cell.state == Completeness::Failed => {
                    self.coverage.failed.push(path.clone());
                    let reason = cell.reason.as_deref().unwrap_or("syntax error");
                    let finding = Finding::new(
                        codes::CODE_PARSE_FAILED,
                        path.clone(),
                        format!("{reason}; the file cannot be reported as checked"),
                    );
                    self.findings.push(plain(
                        located(finding, cell.line.unwrap_or(0), &owners),
                        Tag::Fix,
                        "repair the syntax so the file parses, then run the check again",
                        owners,
                    ));
                }
                Some(cell) if cell.state == Completeness::Complete => {
                    self.coverage.measured = self.coverage.measured.saturating_add(1);
                }
                _ => self.coverage.unmeasured.push(path.clone()),
            }
        }
    }

    /// Step 2: values above their limit, classified against the before side.
    fn limits(&mut self) {
        let judgements = judge(self.input.facts, self.input.config, &|path| {
            self.judged(path)
        });
        for judgement in judgements {
            let finding = self.limit_finding(judgement);
            self.findings.push(finding);
        }
    }

    fn limit_finding(&self, judgement: Judgement) -> CodeFinding {
        let entity = &judgement.entity;
        let owners = self.owners(&entity.path);
        let (tag, hint) = limit_hint(entity.scope, &judgement.metric, judgement.limit, &owners);
        let earlier = self.pairing.earlier(entity, &judgement.metric);
        let value = report::display_number(judgement.value);
        let limit = report::display_number(judgement.limit);
        let source = judgement.limit_source.label();
        let regression = earlier.is_some_and(|earlier| {
            self.regressed(entity, &judgement.metric, judgement.value, earlier)
        });
        let pre_existing = !regression
            && earlier.is_some_and(|earlier| {
                earlier.trust.excuses_debt() && judgement.value <= earlier.value
            });
        let (code, severity, message) = match earlier {
            Some(earlier) if regression => (
                codes::CODE_RATCHET,
                Severity::Error,
                format!(
                    "{}: {} rose from {} to {value}, limit {limit} ({source})",
                    subject(entity),
                    judgement.metric,
                    report::display_number(earlier.value)
                ),
            ),
            Some(earlier) if pre_existing => (
                limit_code(entity.scope),
                judgement.severity,
                format!(
                    "{}: {} is {value}, limit {limit} ({source}); pre-existing, was {} before",
                    subject(entity),
                    judgement.metric,
                    report::display_number(earlier.value)
                ),
            ),
            _ => (
                limit_code(entity.scope),
                judgement.severity,
                format!(
                    "{}: {} is {value}, limit {limit} ({source})",
                    subject(entity),
                    judgement.metric
                ),
            ),
        };
        let finding = Finding::new(code, entity.path.clone(), message).severity(severity);
        CodeFinding {
            finding: located(finding, judgement.line, &owners),
            tag,
            hint,
            metric: Some(judgement.metric),
            value: Some(judgement.value),
            before: earlier.map(|earlier| earlier.value),
            limit: Some(judgement.limit),
            limit_source: Some(source),
            pre_existing,
            trust: earlier.map(|earlier| earlier.trust.label()),
            entity: Some(judgement.entity),
            owners,
        }
    }

    /// Step 3: paired routine metrics that changed. A value that rose and
    /// stayed within its limit is a warning; every change is recorded for
    /// the summary.
    fn changes(&mut self) -> Vec<Change> {
        let mut changes = Vec::new();
        let mut warnings = Vec::new();
        for observation in &self.input.facts.observations {
            let entity = &observation.entity;
            let metric = observation.metric.name.as_str();
            if entity.scope != Scope::Routine
                || !RATCHETED.contains(&metric)
                || !self.judged(&entity.path)
            {
                continue;
            }
            let Some(earlier) = self.pairing.earlier(entity, metric) else {
                continue;
            };
            if observation.value.to_bits() == earlier.value.to_bits() {
                continue;
            }
            let limit = self
                .input
                .config
                .limit_for(entity.scope, metric, &entity.path);
            changes.push(Change {
                entity: entity.clone(),
                metric: metric.to_string(),
                before: earlier.value,
                after: observation.value,
                limit: limit.as_ref().map(|(limit, _)| limit.maximum),
                line: observation.line,
            });
            let Some((limit, source)) = limit else {
                continue;
            };
            let within = observation.value <= limit.maximum;
            if within && self.regressed(entity, metric, observation.value, earlier) {
                let owners = self.owners(&entity.path);
                let (tag, hint) = limit_hint(entity.scope, metric, limit.maximum, &owners);
                let finding = Finding::new(
                    codes::CODE_RATCHET,
                    entity.path.clone(),
                    format!(
                        "{}: {metric} rose from {} to {}, within the limit {} ({})",
                        subject(entity),
                        report::display_number(earlier.value),
                        report::display_number(observation.value),
                        report::display_number(limit.maximum),
                        source.label()
                    ),
                )
                .severity(Severity::Warning);
                warnings.push(CodeFinding {
                    finding: located(finding, observation.line, &owners),
                    tag,
                    hint,
                    metric: Some(metric.to_string()),
                    value: Some(observation.value),
                    before: Some(earlier.value),
                    limit: Some(limit.maximum),
                    limit_source: Some(source.label()),
                    pre_existing: false,
                    trust: Some(earlier.trust.label()),
                    entity: Some(entity.clone()),
                    owners,
                });
            }
        }
        self.findings.append(&mut warnings);
        changes
    }

    /// Step 4: selected files with a finding that no spec maps, once each.
    fn unowned(&mut self) {
        let unowned: BTreeSet<String> = self
            .findings
            .iter()
            .filter(|entry| entry.owners.feature.is_none())
            .map(|entry| entry.finding.file.clone())
            .collect();
        for path in unowned {
            self.findings.push(plain(
                Finding::new(
                    codes::CODE_FILE_UNOWNED,
                    path,
                    "the file has findings and is mapped by no spec's File Structure Plan",
                ),
                Tag::Fix,
                "add the file to the File Structure Plan of the design that owns it",
                Attribution::default(),
            ));
        }
    }

    /// Step 5: findings per spec in precedence order, unowned last.
    fn summaries(&self) -> Vec<SpecSummary> {
        let mut names: Vec<Option<String>> = self
            .input
            .specs
            .iter()
            .map(|feature| Some(feature.name.clone()))
            .collect();
        names.push(None);
        let mut specs = Vec::new();
        for name in names {
            let files = self
                .ownership
                .values()
                .filter(|owners| owners.feature == name)
                .count();
            if files == 0 {
                continue;
            }
            let count = |severity: Severity| {
                self.findings
                    .iter()
                    .filter(|entry| {
                        entry.owners.feature == name && entry.finding.severity == severity
                    })
                    .count()
            };
            specs.push(SpecSummary {
                spec: name.clone(),
                files,
                errors: count(Severity::Error),
                warnings: count(Severity::Warning),
                info: count(Severity::Info),
            });
        }
        specs
    }

    /// Highest value and total of every ratcheted routine metric over the
    /// judged files of each spec.
    fn measures(&self) -> Vec<Measure> {
        let mut by_key: BTreeMap<(Option<String>, &str), Measure> = BTreeMap::new();
        for observation in &self.input.facts.observations {
            let entity = &observation.entity;
            let metric = observation.metric.name.as_str();
            if entity.scope != Scope::Routine
                || !RATCHETED.contains(&metric)
                || !self.judged(&entity.path)
            {
                continue;
            }
            let spec = self.owners(&entity.path).feature;
            let entry = by_key
                .entry((spec.clone(), metric))
                .or_insert_with(|| Measure {
                    spec,
                    metric: metric.to_string(),
                    highest: observation.value,
                    total: 0.0,
                    routines: 0,
                });
            entry.highest = entry.highest.max(observation.value);
            entry.total += observation.value;
            entry.routines = entry.routines.saturating_add(1);
        }
        by_key.into_values().collect()
    }

    /// Signed statement and line deltas over the accounting scope, or why
    /// they cannot be computed. A total is given only when both sides are
    /// complete; a missing side is never counted as zero.
    fn net(&self) -> NetDelta {
        let Some(before) = &self.input.before else {
            return NetDelta::Unavailable("no before side".to_string());
        };
        if !self.coverage.failed.is_empty() || !self.coverage.unmeasured.is_empty() {
            return NetDelta::Unavailable(
                "a selected file was not measured completely".to_string(),
            );
        }
        let before_paths: BTreeSet<&str> =
            self.input.before_paths.iter().map(String::as_str).collect();
        let incomplete = before.facts.coverage.iter().any(|cell| {
            cell.metric.is_none()
                && before_paths.contains(cell.path.as_str())
                && cell.state != Completeness::Complete
        });
        if incomplete {
            return NetDelta::Unavailable(
                "a file of the before side was not measured completely".to_string(),
            );
        }
        let total = |facts: &CodeFacts, wanted: &dyn Fn(&str) -> bool, metric: &str| -> f64 {
            facts
                .observations
                .iter()
                .filter(|observation| {
                    observation.entity.scope == Scope::Routine
                        && observation.metric.name == metric
                        && wanted(&observation.entity.path)
                })
                .map(|observation| observation.value)
                .sum()
        };
        let after = |path: &str| self.ownership.contains_key(path);
        let earlier = |path: &str| before_paths.contains(path);
        NetDelta::Known {
            statements: total(self.input.facts, &after, metrics::STATEMENTS)
                - total(before.facts, &earlier, metrics::STATEMENTS),
            code_lines: total(self.input.facts, &after, metrics::CODE_LINES)
                - total(before.facts, &earlier, metrics::CODE_LINES),
        }
    }
}

/// Evaluates the facts of a run into the code report.
#[must_use]
pub fn evaluate(input: &EvaluateInput<'_>) -> CodeReport {
    let coverage = Coverage {
        selected: input.selected.len(),
        before_side: match &input.before {
            Some(before) => before.identity.to_string(),
            None => "none: no baseline and no revision given, so nothing is ratcheted".to_string(),
        },
        // A file the configuration ignores is measured but not judged.
        ignored: input
            .selected
            .iter()
            .filter(|path| input.config.ignored(path).is_some())
            .cloned()
            .collect(),
        ..Coverage::default()
    };
    let mut run = Run {
        input,
        // Ownership is resolved once per selected file.
        ownership: input
            .selected
            .iter()
            .map(|path| (path.as_str(), attribute(path, input.specs)))
            .collect(),
        pairing: input
            .before
            .as_ref()
            .map(|before| pair(before, input.facts))
            .unwrap_or_default(),
        findings: Vec::new(),
        coverage,
    };
    run.file_states();
    run.limits();
    let changes = run.changes();
    run.unowned();
    run.findings.sort_by(|a, b| {
        (&a.finding.file, a.finding.line, a.finding.code, &a.metric).cmp(&(
            &b.finding.file,
            b.finding.line,
            b.finding.code,
            &b.metric,
        ))
    });
    let specs = run.summaries();
    let net = run.net();

    // What the run measured: always reported, so a clean report is never
    // mistaken for a checked one.
    let coverage = &run.coverage;
    let message = format!(
        "{} files selected: {} measured, {} failed to parse, {} not measured (no profile for the language), {} ignored by configuration, {} mapped by no spec; configuration {}; before side: {}; {} entities paired; net: {}",
        coverage.selected,
        coverage.measured,
        coverage.failed.len(),
        coverage.unmeasured.len(),
        coverage.ignored.len(),
        coverage.unowned,
        input.config.digest(),
        coverage.before_side,
        run.pairing.len(),
        net.label()
    );
    run.findings.push(plain(
        Finding::new(codes::CODE_COVERAGE, super::config::CONFIG_PATH, message),
        Tag::Fix,
        "read this line before trusting a clean report; unmeasured files were not judged",
        Attribution::default(),
    ));

    let measures = run.measures();
    CodeReport {
        findings: run.findings,
        coverage: run.coverage,
        specs,
        changes,
        net,
        measures,
    }
}
