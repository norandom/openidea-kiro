//! Telemetry of a gate run (spec `quality-telemetry`).
//!
//! The metric snapshot is built from the report `check` produced; nothing
//! is checked twice. With a telemetry directory the run is written as three
//! OpenTelemetry protocol JSON lines for a Collector to ship. The gate
//! opens no network connection.

use kiro_core::finding::Finding;
use kiro_core::source::Feature;
use kiro_core::telemetry::{
    DEFAULT_MAX_RECORDS, MetricSnapshot, OtlpLines, ResourceIdentity, RunIdentity, SpecInput,
    SpecMetrics, otlp_lines, snapshot,
};

use crate::exit::{Exit, Failure};
use crate::fs;
use crate::report::GateReport;

/// Sources a run of this build is expected to have.
const EXPECTED_SOURCES: &[&str] = &["spec"];

/// File names inside the telemetry directory, one per signal.
pub const METRICS_FILE: &str = "metrics.jsonl";
pub const LOGS_FILE: &str = "logs.jsonl";
pub const TRACES_FILE: &str = "traces.jsonl";

/// Reduces a gate report and the features it checked to a snapshot.
///
/// `features` is in the order of `report.checked`, as
/// [`crate::check::run_with_features`] returns it.
#[must_use]
pub fn snapshot_of(report: &GateReport, features: &[Feature]) -> MetricSnapshot {
    let metrics = SpecMetrics {
        specs: features
            .iter()
            .zip(report.checked.iter())
            .map(|(feature, result)| SpecInput {
                feature,
                report: Some(&result.report),
                findings: Some(&result.findings),
            })
            .collect(),
    };
    snapshot(&[&metrics], EXPECTED_SOURCES)
}

/// Every finding of the run: vocabulary, roadmap, then per spec.
#[must_use]
pub fn all_findings(report: &GateReport) -> Vec<Finding> {
    report
        .vocabulary
        .findings
        .iter()
        .chain(report.roadmap.iter())
        .chain(
            report
                .checked
                .iter()
                .flat_map(|result| result.findings.iter()),
        )
        .cloned()
        .collect()
}

/// The verdict word written on the root span.
#[must_use]
pub fn verdict_word(exit: Exit) -> &'static str {
    match exit {
        Exit::Clean => "pass",
        Exit::Findings => "fail",
        Exit::Failure => "failure",
    }
}

/// Maps the run to its three lines.
#[must_use]
pub fn lines(
    report: &GateReport,
    features: &[Feature],
    identity: &ResourceIdentity,
    run: &RunIdentity,
) -> OtlpLines {
    otlp_lines(
        &snapshot_of(report, features),
        &all_findings(report),
        identity,
        run,
        DEFAULT_MAX_RECORDS,
    )
}

/// Appends the three lines to the telemetry directory, one write per file.
pub fn write(dir: &std::path::Path, lines: &OtlpLines) -> Result<(), Failure> {
    fs::append_line(&dir.join(METRICS_FILE), &lines.metrics)?;
    fs::append_line(&dir.join(LOGS_FILE), &lines.logs)?;
    fs::append_line(&dir.join(TRACES_FILE), &lines.traces)
}

/// Where and as what kind of run telemetry is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub dir: std::path::PathBuf,
    pub kind: String,
}

/// Resolves the telemetry settings of an invocation: the flag, then
/// `KIRO_TELEMETRY_DIR`. `None` means telemetry is off. A directory inside
/// the project or an invalid `KIRO_RUN_KIND` is a usage failure, reported
/// before the gate runs.
pub fn plan(
    root: &fs::ProjectRoot,
    flag: Option<&std::path::Path>,
    environment: &crate::telemetry_io::TelemetryEnvironment,
) -> Result<Option<Plan>, Failure> {
    let requested = match (flag, environment.dir.as_deref()) {
        (Some(dir), _) => dir.to_path_buf(),
        (None, Some(dir)) => std::path::PathBuf::from(dir),
        (None, None) => return Ok(None),
    };
    let kind = crate::telemetry_io::run_kind(environment)?.to_string();
    let dir = fs::telemetry_dir(root, &requested)?;
    Ok(Some(Plan { dir, kind }))
}

/// Where the telemetry of a run goes and what identifies the run: the
/// plan, the project the Git identity is read from, and the environment.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub plan: &'a Plan,
    pub root: &'a fs::ProjectRoot,
    pub environment: &'a crate::telemetry_io::TelemetryEnvironment,
}

/// Writes the telemetry of a finished run.
pub fn emit(
    target: Target<'_>,
    report: &GateReport,
    features: &[Feature],
    timer: crate::telemetry_io::RunTimer,
) -> Result<(), Failure> {
    let Target {
        plan,
        root,
        environment,
    } = target;
    let git = crate::telemetry_io::GitIdentity::read(root);
    let identity = crate::telemetry_io::resource_identity(environment, &git, &plan.kind);
    let run = timer.finish(git.revision.as_deref(), verdict_word(report.exit));
    write(&plan.dir, &lines(report, features, &identity, &run))
}

/// Writes the telemetry of a finished code check: the code metric points,
/// one record per finding with its hint, tag, component and tasks, and
/// the run's trace.
pub fn emit_code(
    target: Target<'_>,
    report: &kiro_core::code::CodeReport,
    verdict: Exit,
    timer: crate::telemetry_io::RunTimer,
) -> Result<(), Failure> {
    let Target {
        plan,
        root,
        environment,
    } = target;
    let git = crate::telemetry_io::GitIdentity::read(root);
    let identity = crate::telemetry_io::resource_identity(environment, &git, &plan.kind);
    let run = timer.finish(git.revision.as_deref(), verdict_word(verdict));
    let entries: Vec<kiro_core::telemetry::LogEntry<'_>> = report
        .findings
        .iter()
        .map(|entry| {
            let mut extra = vec![
                ("kiro.tag", entry.tag.label().to_string()),
                ("kiro.hint", entry.hint.clone()),
                ("kiro.pre_existing", entry.pre_existing.to_string()),
            ];
            if let Some(metric) = &entry.metric {
                extra.push(("kiro.metric", metric.clone()));
            }
            if !entry.owners.components.is_empty() {
                extra.push(("kiro.component", entry.owners.components.join(",")));
            }
            if !entry.owners.tasks.is_empty() {
                extra.push(("kiro.task", entry.owners.tasks.join(",")));
            }
            kiro_core::telemetry::LogEntry {
                finding: &entry.finding,
                extra,
            }
        })
        .collect();
    let snapshot = snapshot(&[report], &["code"]);
    let lines = kiro_core::telemetry::otlp_lines_from(
        &snapshot,
        &entries,
        &identity,
        &run,
        DEFAULT_MAX_RECORDS,
    );
    write(&plan.dir, &lines)
}
