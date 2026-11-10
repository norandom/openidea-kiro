//! Metric snapshot of a gate run (spec `quality-telemetry`).
//!
//! Reports are reduced to one list of named values. The list is pure data:
//! building it reads no clock, network or filesystem. A source that did not
//! run contributes nothing and is listed as absent; it is never reported as
//! zero.

pub mod catalogue;
pub mod model;
pub mod otlp;
pub mod spec;

pub use catalogue::{
    ATTRIBUTE_KEYS, METRICS, MetricInfo, RESERVED_PREFIXES, catalogue_markdown, field_name, lookup,
    stream_name,
};
pub use model::{Attr, MetricPoint, MetricProvider, MetricSnapshot, SourceCoverage, Unit};
pub use otlp::{
    DEFAULT_MAX_RECORDS, OtlpLines, Phase, ResourceIdentity, RunIdentity, SCOPE_NAME, otlp_lines,
};
pub use spec::{SpecInput, SpecMetrics};

/// Version of the snapshot's shape and meaning.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Why a point cannot enter a snapshot.
fn rejection(point: &MetricPoint) -> Option<String> {
    let Some(info) = lookup(point.name) else {
        return Some(format!("{}: not in the catalogue", point.name));
    };
    if info.unit != point.unit {
        return Some(format!(
            "{}: unit {} differs from the catalogue's {}",
            point.name,
            point.unit.label(),
            info.unit.label()
        ));
    }
    if !point.value.is_finite() {
        return Some(format!("{}: value is not finite", point.name));
    }
    for (index, attr) in point.attrs.iter().enumerate() {
        if !info.attributes.contains(&attr.key) {
            return Some(format!(
                "{}: attribute {} is not listed for this metric",
                point.name, attr.key
            ));
        }
        if point
            .attrs
            .iter()
            .take(index)
            .any(|earlier| earlier.key == attr.key)
        {
            return Some(format!(
                "{}: attribute {} occurs twice",
                point.name, attr.key
            ));
        }
    }
    None
}

fn identity(point: &MetricPoint) -> (&'static str, &[Attr]) {
    (point.name, point.attrs.as_slice())
}

/// Builds the snapshot of one run.
///
/// `expected` names the sources this run should have; one without a
/// provider is listed as absent. Points are validated against the
/// catalogue: an unknown name, a unit or attribute the catalogue does not
/// list, a nonfinite value or a second point with the same name and
/// attributes is dropped and named in the coverage of its source. The
/// result is sorted by name, then attributes, so equal input gives an
/// equal snapshot.
#[must_use]
pub fn snapshot(providers: &[&dyn MetricProvider], expected: &[&'static str]) -> MetricSnapshot {
    let mut points: Vec<MetricPoint> = Vec::new();
    let mut coverage: Vec<SourceCoverage> = Vec::new();

    for provider in providers {
        let source = provider.source();
        let mut dropped: Vec<String> = Vec::new();
        for mut point in provider.points() {
            point.attrs.sort();
            if let Some(reason) = rejection(&point) {
                dropped.push(reason);
            } else if points.iter().any(|kept| identity(kept) == identity(&point)) {
                dropped.push(format!("{}: duplicate point", point.name));
            } else {
                points.push(point);
            }
        }
        let reason = if dropped.is_empty() {
            None
        } else {
            Some(format!(
                "dropped {} point(s): {}",
                dropped.len(),
                dropped.join("; ")
            ))
        };
        match coverage.iter_mut().find(|entry| entry.source == source) {
            Some(entry) => {
                if let Some(reason) = reason {
                    entry.reason = Some(match entry.reason.take() {
                        Some(previous) => format!("{previous}; {reason}"),
                        None => reason,
                    });
                }
            }
            None => coverage.push(SourceCoverage {
                source,
                present: true,
                reason,
            }),
        }
    }

    for source in expected {
        if !coverage.iter().any(|entry| entry.source == *source) {
            coverage.push(SourceCoverage {
                source,
                present: false,
                reason: Some("no report of this source in the run".to_string()),
            });
        }
    }

    points.sort_by(|a, b| identity(a).cmp(&identity(b)));
    coverage.sort_by(|a, b| a.source.cmp(b.source));

    MetricSnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        points,
        coverage,
    }
}

fn attrs_text(attrs: &[Attr]) -> String {
    attrs
        .iter()
        .map(|attr| format!("{}={}", attr.key, attr.value))
        .collect::<Vec<_>>()
        .join(",")
}

/// The snapshot as plain text: one point per line, then the coverage.
#[must_use]
pub fn snapshot_text(snapshot: &MetricSnapshot) -> String {
    let mut out = String::new();
    for point in &snapshot.points {
        out.push_str(&format!(
            "{}{{{}}} {} {}\n",
            point.name,
            attrs_text(&point.attrs),
            point.value,
            point.unit.label()
        ));
    }
    for entry in &snapshot.coverage {
        let state = if entry.present { "present" } else { "absent" };
        match &entry.reason {
            Some(reason) => out.push_str(&format!("source {}: {state} ({reason})\n", entry.source)),
            None => out.push_str(&format!("source {}: {state}\n", entry.source)),
        }
    }
    out
}

/// The snapshot as a JSON document with a trailing newline.
#[must_use]
pub fn snapshot_json(snapshot: &MetricSnapshot) -> String {
    let points: Vec<serde_json::Value> = snapshot
        .points
        .iter()
        .map(|point| {
            let attrs: serde_json::Map<String, serde_json::Value> = point
                .attrs
                .iter()
                .map(|attr| {
                    (
                        attr.key.to_string(),
                        serde_json::Value::String(attr.value.clone()),
                    )
                })
                .collect();
            serde_json::json!({
                "name": point.name,
                "unit": point.unit.label(),
                "value": point.value,
                "attributes": attrs,
            })
        })
        .collect();
    let coverage: Vec<serde_json::Value> = snapshot
        .coverage
        .iter()
        .map(|entry| {
            serde_json::json!({
                "source": entry.source,
                "present": entry.present,
                "reason": entry.reason,
            })
        })
        .collect();
    let document = serde_json::json!({
        "schema_version": snapshot.schema_version,
        "points": points,
        "coverage": coverage,
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}
