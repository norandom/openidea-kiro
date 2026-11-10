//! Specification and traceability metrics (quality-telemetry requirement 2).
//!
//! Reads values the engine already computed: no parsing and no new check.
//! A value whose source is missing for a spec is omitted, never zero.

use std::collections::BTreeMap;

use super::model::{Attr, MetricPoint, MetricProvider, Unit};
use crate::checks::CheckReport;
use crate::design::normalize_design_path;
use crate::finding::{Finding, Severity};
use crate::join::coverage;
use crate::source::Feature;
use crate::spec::phase_label;

/// What one spec contributes.
#[derive(Debug, Clone, Copy)]
pub struct SpecInput<'a> {
    /// The loaded feature: metadata, requirements, design and tasks.
    pub feature: &'a Feature,
    /// The check report of this run, when the checks ran for this spec.
    pub report: Option<&'a CheckReport>,
    /// The findings reported for this spec, when the checks ran.
    pub findings: Option<&'a [Finding]>,
}

/// Provider of the `kiro.spec.*` and `kiro.trace.*` metrics.
#[derive(Debug, Clone, Default)]
pub struct SpecMetrics<'a> {
    pub specs: Vec<SpecInput<'a>>,
}

fn count(name: &'static str, spec: &str, value: usize) -> MetricPoint {
    #[allow(clippy::cast_precision_loss)]
    let value = value as f64;
    MetricPoint::new(name, Unit::Count, value, vec![Attr::new("kiro.spec", spec)])
}

fn count_by(
    name: &'static str,
    spec: &str,
    key: &'static str,
    label: &str,
    value: usize,
) -> MetricPoint {
    let mut point = count(name, spec, value);
    point.attrs.push(Attr::new(key, label));
    point
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn status_points(feature: &Feature, out: &mut Vec<MetricPoint>) {
    let spec = feature.name.as_str();
    if let Some(meta) = &feature.meta {
        out.push(MetricPoint::new(
            "kiro.spec.phase",
            Unit::Value,
            1.0,
            vec![
                Attr::new("kiro.spec", spec),
                Attr::new("kiro.phase", phase_label(Some(meta))),
            ],
        ));
        let documents = [
            ("requirements", meta.approvals.requirements.approved),
            ("design", meta.approvals.design.approved),
            ("tasks", meta.approvals.tasks.approved),
        ];
        for (document, approved) in documents {
            out.push(MetricPoint::new(
                "kiro.spec.approved",
                Unit::Value,
                if approved { 1.0 } else { 0.0 },
                vec![
                    Attr::new("kiro.spec", spec),
                    Attr::new("kiro.document", document),
                ],
            ));
        }
    }
    if let Some(tasks) = &feature.tasks {
        out.push(count("kiro.spec.tasks.total", spec, tasks.counts.total));
        out.push(count("kiro.spec.tasks.done", spec, tasks.counts.checked));
    }
    if let Some(requirements) = &feature.requirements {
        let total = requirements.criteria().count();
        let checked = requirements
            .criteria()
            .filter(|criterion| !criterion.annotations.is_empty())
            .count();
        out.push(count("kiro.spec.criteria.total", spec, total));
        out.push(count("kiro.spec.criteria.checked", spec, checked));
    }
}

fn check_points(input: &SpecInput<'_>, out: &mut Vec<MetricPoint>) {
    let spec = input.feature.name.as_str();
    if let Some(findings) = input.findings {
        let mut by_code: BTreeMap<(&str, &'static str), usize> = BTreeMap::new();
        for finding in findings {
            let key = (finding.code, severity_label(finding.severity));
            let entry = by_code.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        }
        for ((code, severity), value) in by_code {
            out.push(MetricPoint::new(
                "kiro.spec.findings",
                Unit::Count,
                {
                    #[allow(clippy::cast_precision_loss)]
                    let value = value as f64;
                    value
                },
                vec![
                    Attr::new("kiro.spec", spec),
                    Attr::new("kiro.code", code),
                    Attr::new("kiro.severity", severity),
                ],
            ));
        }
    }
    let Some(report) = input.report else {
        return;
    };
    if let Some(terms) = &report.terms {
        let states = [
            ("declared", terms.declared),
            ("resolved", terms.resolved),
            ("deprecated", terms.deprecated),
            ("mismatched", terms.mismatched),
            ("undeclared", terms.undeclared),
            ("ambiguous", terms.ambiguous),
            ("suggested", terms.suggested),
        ];
        for (state, value) in states {
            out.push(count_by(
                "kiro.spec.terms",
                spec,
                "kiro.status",
                state,
                value,
            ));
        }
    }
    if let Some(search) = &report.contradictions {
        let outcomes = [
            ("considered", search.pairs_considered),
            ("found", search.contradictions),
            ("refused", search.refused_pairs),
        ];
        for (outcome, value) in outcomes {
            out.push(count_by(
                "kiro.spec.contradiction.pairs",
                spec,
                "kiro.status",
                outcome,
                value,
            ));
        }
    }
}

/// Whether a task's requirement reference covers a criterion: the exact
/// criterion id, or the id of the criterion's requirement.
fn reference_covers(reference: &str, criterion: &str) -> bool {
    reference == criterion
        || criterion
            .rsplit_once('.')
            .is_some_and(|(requirement, _)| requirement == reference)
}

fn trace_points(feature: &Feature, others: &[&Feature], out: &mut Vec<MetricPoint>) {
    let spec = feature.name.as_str();
    if let (Some(requirements), Some(tasks)) = (&feature.requirements, &feature.tasks) {
        let untasked = requirements
            .criteria()
            .filter(|criterion| {
                !tasks.all().any(|task| {
                    task.requirements
                        .iter()
                        .any(|reference| reference_covers(reference, &criterion.id))
                })
            })
            .count();
        out.push(count("kiro.trace.criteria.untasked", spec, untasked));
    }
    let Some(design) = &feature.design else {
        return;
    };
    let covered = coverage(Some(design), feature.tasks.as_ref());
    if feature.tasks.is_some() {
        out.push(count(
            "kiro.trace.tasks.unknown_boundary",
            spec,
            covered.unknown_boundary_tasks.len(),
        ));
    }
    let unmapped = covered
        .components
        .iter()
        .filter(|component| component.files.is_empty())
        .count();
    out.push(count("kiro.trace.components.unmapped", spec, unmapped));

    let mut own: Vec<String> = design
        .files
        .iter()
        .map(|file| normalize_design_path(&file.path))
        .collect();
    own.sort();
    own.dedup();
    let multi_owned = own
        .iter()
        .filter(|path| {
            others.iter().any(|other| {
                other.design.as_ref().is_some_and(|other_design| {
                    other_design
                        .files
                        .iter()
                        .any(|file| &normalize_design_path(&file.path) == *path)
                })
            })
        })
        .count();
    out.push(count("kiro.trace.files.multi_owned", spec, multi_owned));
}

impl MetricProvider for SpecMetrics<'_> {
    fn source(&self) -> &'static str {
        "spec"
    }

    fn points(&self) -> Vec<MetricPoint> {
        let mut out = Vec::new();
        for input in &self.specs {
            let others: Vec<&Feature> = self
                .specs
                .iter()
                .map(|other| other.feature)
                .filter(|other| other.name != input.feature.name)
                .collect();
            status_points(input.feature, &mut out);
            check_points(input, &mut out);
            trace_points(input.feature, &others, &mut out);
        }
        out
    }
}
