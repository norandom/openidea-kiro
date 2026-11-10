//! Metric points of a code check (code-constraints 13.3, 13.5).
//!
//! The code report hands its values to the telemetry snapshot through the
//! provider contract. The points are a view of the report: nothing is
//! measured or judged again, and the gate writes no metric file itself.
//!
//! Points carry the owning spec. Component and task are not attributes of
//! a metric, because they would multiply the series; they are attributes
//! of the finding records.

use std::collections::BTreeMap;

use super::report::CodeReport;
use crate::finding::Severity;
use crate::telemetry::{Attr, MetricPoint, MetricProvider, Unit};

/// The spec attribute of files no spec maps.
pub const UNOWNED: &str = "(none)";

fn count(value: usize) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let value = value as f64;
    value
}

fn spec_attr(spec: Option<&String>) -> Attr {
    Attr::new("kiro.spec", spec.map_or(UNOWNED, String::as_str))
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

impl MetricProvider for CodeReport {
    fn source(&self) -> &'static str {
        "code"
    }

    fn points(&self) -> Vec<MetricPoint> {
        let mut out = Vec::new();

        // Highest value, total and routine count per spec and metric.
        let mut routines: BTreeMap<Option<&String>, usize> = BTreeMap::new();
        for measure in &self.measures {
            let attrs = || {
                vec![
                    spec_attr(measure.spec.as_ref()),
                    Attr::new("kiro.metric", measure.metric.clone()),
                ]
            };
            out.push(MetricPoint::new(
                "kiro.code.highest",
                Unit::Value,
                measure.highest,
                attrs(),
            ));
            out.push(MetricPoint::new(
                "kiro.code.total",
                Unit::Value,
                measure.total,
                attrs(),
            ));
            let entry = routines.entry(measure.spec.as_ref()).or_insert(0);
            *entry = (*entry).max(measure.routines);
        }
        for (spec, value) in routines {
            out.push(MetricPoint::new(
                "kiro.code.routines",
                Unit::Count,
                count(value),
                vec![spec_attr(spec)],
            ));
        }

        // Findings that are not pre-existing, by spec, code and severity;
        // pre-existing ones by spec.
        let mut breaches: BTreeMap<(Option<&String>, &str, &'static str), usize> = BTreeMap::new();
        let mut pre_existing: BTreeMap<Option<&String>, usize> = BTreeMap::new();
        for entry in &self.findings {
            let spec = entry.owners.feature.as_ref();
            if entry.pre_existing {
                *pre_existing.entry(spec).or_insert(0) += 1;
            } else if entry.metric.is_some() {
                let key = (
                    spec,
                    entry.finding.code,
                    severity_label(entry.finding.severity),
                );
                *breaches.entry(key).or_insert(0) += 1;
            }
        }
        for ((spec, code, severity), value) in breaches {
            out.push(MetricPoint::new(
                "kiro.code.breaches",
                Unit::Count,
                count(value),
                vec![
                    spec_attr(spec),
                    Attr::new("kiro.code", code),
                    Attr::new("kiro.severity", severity),
                ],
            ));
        }
        for (spec, value) in pre_existing {
            out.push(MetricPoint::new(
                "kiro.code.pre_existing",
                Unit::Count,
                count(value),
                vec![spec_attr(spec)],
            ));
        }

        // Routine metrics that differ from the before side.
        let mut changed: BTreeMap<(&str, &'static str), usize> = BTreeMap::new();
        for change in &self.changes {
            let status = if change.improved() {
                "improved"
            } else {
                "worsened"
            };
            *changed.entry((change.metric.as_str(), status)).or_insert(0) += 1;
        }
        for ((metric, status), value) in changed {
            out.push(MetricPoint::new(
                "kiro.code.changed",
                Unit::Count,
                count(value),
                vec![
                    Attr::new("kiro.metric", metric),
                    Attr::new("kiro.status", status),
                ],
            ));
        }

        // What the run measured.
        let files = [
            ("measured", self.coverage.measured),
            ("failed", self.coverage.failed.len()),
            ("unmeasured", self.coverage.unmeasured.len()),
            ("ignored", self.coverage.ignored.len()),
        ];
        for (status, value) in files {
            out.push(MetricPoint::new(
                "kiro.code.files",
                Unit::Count,
                count(value),
                vec![Attr::new("kiro.status", status)],
            ));
        }
        out
    }
}
