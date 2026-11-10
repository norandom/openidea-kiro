//! OpenTelemetry protocol JSON lines (quality-telemetry 3.2, 4.1 to 4.3).
//!
//! One gate run becomes three lines, one request object per signal, in the
//! OTLP JSON encoding: lower camel case keys, 64-bit integers and timestamps
//! as decimal strings, trace and span identifiers as lowercase hex. An
//! OpenTelemetry Collector's JSON file receiver, or an HTTP post, ships a
//! line unchanged.
//!
//! This is serialization of a published schema, not a protocol
//! implementation. The function is pure: resource, timestamps, durations
//! and identifiers are inputs. Tests decode every line with the official
//! protocol definitions.

use serde_json::{Value, json};

use super::model::{Attr, MetricPoint, MetricSnapshot, Unit};
use crate::finding::{Finding, Severity};

/// Instrumentation scope name written into every signal.
pub const SCOPE_NAME: &str = "kiro";
/// Default maximum of finding records per run.
pub const DEFAULT_MAX_RECORDS: usize = 2000;

/// Resource attributes of the run, in the order they are written.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceIdentity {
    pub attributes: Vec<(String, String)>,
}

/// One timed phase of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase {
    pub name: &'static str,
    pub start_unix_nanos: u64,
    pub end_unix_nanos: u64,
}

/// Identity and timing of one gate run, produced by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunIdentity {
    pub trace_id: [u8; 16],
    pub root_span_id: [u8; 8],
    pub start_unix_nanos: u64,
    pub end_unix_nanos: u64,
    pub phases: Vec<Phase>,
    /// `pass`, `fail` or another short verdict word.
    pub verdict: String,
}

/// The three lines of one run. No line contains a newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtlpLines {
    pub metrics: String,
    pub logs: String,
    pub traces: String,
    /// Finding records left out because `max_records` was reached.
    pub dropped_records: usize,
}

fn string_attr(key: &str, value: &str) -> Value {
    json!({ "key": key, "value": { "stringValue": value } })
}

fn int_attr(key: &str, value: u64) -> Value {
    json!({ "key": key, "value": { "intValue": value.to_string() } })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn resource(identity: &ResourceIdentity) -> Value {
    let attributes: Vec<Value> = identity
        .attributes
        .iter()
        .map(|(key, value)| string_attr(key, value))
        .collect();
    json!({ "attributes": attributes })
}

fn point_attrs(attrs: &[Attr]) -> Vec<Value> {
    attrs
        .iter()
        .map(|attr| string_attr(attr.key, &attr.value))
        .collect()
}

fn data_point(point: &MetricPoint, time: u64) -> Value {
    json!({
        "asDouble": point.value,
        "timeUnixNano": time.to_string(),
        "attributes": point_attrs(&point.attrs),
    })
}

/// Run-level points that only the mapping knows: phase durations and the
/// number of dropped finding records.
fn run_points(run: &RunIdentity, dropped: usize) -> Vec<MetricPoint> {
    let mut points: Vec<MetricPoint> = run
        .phases
        .iter()
        .map(|phase| {
            let nanos = phase.end_unix_nanos.saturating_sub(phase.start_unix_nanos);
            #[allow(clippy::cast_precision_loss)]
            let millis = nanos as f64 / 1_000_000.0;
            MetricPoint::new(
                "kiro.run.duration",
                Unit::Milliseconds,
                millis,
                vec![Attr::new("kiro.phase", phase.name)],
            )
        })
        .collect();
    #[allow(clippy::cast_precision_loss)]
    let dropped = dropped as f64;
    points.push(MetricPoint::new(
        "kiro.run.findings.dropped",
        Unit::Count,
        dropped,
        Vec::new(),
    ));
    points
}

fn metrics_line(
    snapshot: &MetricSnapshot,
    resource_value: &Value,
    run: &RunIdentity,
    dropped: usize,
) -> String {
    let extra = run_points(run, dropped);
    let mut metrics: Vec<Value> = Vec::new();
    let mut current: Option<(&'static str, Unit, Vec<Value>)> = None;
    // Snapshot points are sorted by name, so equal names are adjacent.
    for point in snapshot.points.iter().chain(extra.iter()) {
        match &mut current {
            Some((name, _, data)) if *name == point.name => {
                data.push(data_point(point, run.end_unix_nanos));
            }
            _ => {
                if let Some((name, unit, data)) = current.take() {
                    metrics.push(
                        json!({ "name": name, "unit": unit.ucum(), "gauge": { "dataPoints": data } }),
                    );
                }
                current = Some((
                    point.name,
                    point.unit,
                    vec![data_point(point, run.end_unix_nanos)],
                ));
            }
        }
    }
    if let Some((name, unit, data)) = current {
        metrics.push(json!({ "name": name, "unit": unit.ucum(), "gauge": { "dataPoints": data } }));
    }
    json!({
        "resourceMetrics": [{
            "resource": resource_value,
            "scopeMetrics": [{ "scope": { "name": SCOPE_NAME }, "metrics": metrics }],
        }]
    })
    .to_string()
}

const fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    }
}

const fn severity_number(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 17,
        Severity::Warning => 13,
        Severity::Info => 9,
    }
}

const fn severity_text(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "ERROR",
        Severity::Warning => "WARN",
        Severity::Info => "INFO",
    }
}

const fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn log_record(finding: &Finding, time: u64) -> Value {
    let mut attributes = vec![
        string_attr("kiro.code", finding.code),
        string_attr("kiro.severity", severity_label(finding.severity)),
        string_attr("kiro.file", &finding.file),
    ];
    if let Some(feature) = &finding.feature {
        attributes.push(string_attr("kiro.spec", feature));
    }
    if let Some(line) = finding.line {
        attributes.push(int_attr(
            "kiro.line",
            u64::try_from(line).unwrap_or(u64::MAX),
        ));
    }
    json!({
        "timeUnixNano": time.to_string(),
        "severityNumber": severity_number(finding.severity),
        "severityText": severity_text(finding.severity),
        "body": { "stringValue": finding.message },
        "attributes": attributes,
    })
}

fn logs_line(
    findings: &[Finding],
    resource_value: &Value,
    run: &RunIdentity,
    max_records: usize,
) -> (String, usize) {
    // Errors first, then warnings, then information; the given order within.
    let mut ordered: Vec<&Finding> = findings.iter().collect();
    ordered.sort_by_key(|finding| severity_rank(finding.severity));
    let dropped = ordered.len().saturating_sub(max_records);
    let records: Vec<Value> = ordered
        .into_iter()
        .take(max_records)
        .map(|finding| log_record(finding, run.end_unix_nanos))
        .collect();
    let line = json!({
        "resourceLogs": [{
            "resource": resource_value,
            "scopeLogs": [{ "scope": { "name": SCOPE_NAME }, "logRecords": records }],
        }]
    })
    .to_string();
    (line, dropped)
}

/// Span identifier of the phase at `index`, derived from the root span.
fn phase_span_id(root: [u8; 8], index: usize) -> [u8; 8] {
    let mut id = root;
    let offset = u64::try_from(index).unwrap_or(u64::MAX).wrapping_add(1);
    let value = u64::from_be_bytes(id).wrapping_add(offset);
    id = value.to_be_bytes();
    if id == [0; 8] {
        // The all-zero identifier is invalid in OTLP.
        id = [0, 0, 0, 0, 0, 0, 0, 1];
    }
    id
}

fn traces_line(
    resource_value: &Value,
    run: &RunIdentity,
    findings: &[Finding],
    dropped: usize,
) -> String {
    let count = |severity: Severity| {
        u64::try_from(
            findings
                .iter()
                .filter(|finding| finding.severity == severity)
                .count(),
        )
        .unwrap_or(u64::MAX)
    };
    let trace_id = hex(&run.trace_id);
    let root_id = hex(&run.root_span_id);
    let mut spans = vec![json!({
        "traceId": trace_id,
        "spanId": root_id,
        "name": "kiro.gate",
        "kind": 1,
        "startTimeUnixNano": run.start_unix_nanos.to_string(),
        "endTimeUnixNano": run.end_unix_nanos.to_string(),
        "attributes": [
            string_attr("kiro.verdict", &run.verdict),
            int_attr("kiro.findings.errors", count(Severity::Error)),
            int_attr("kiro.findings.warnings", count(Severity::Warning)),
            int_attr("kiro.findings.info", count(Severity::Info)),
            int_attr("kiro.findings.dropped", u64::try_from(dropped).unwrap_or(u64::MAX)),
        ],
    })];
    for (index, phase) in run.phases.iter().enumerate() {
        spans.push(json!({
            "traceId": trace_id,
            "spanId": hex(&phase_span_id(run.root_span_id, index)),
            "parentSpanId": root_id,
            "name": phase.name,
            "kind": 1,
            "startTimeUnixNano": phase.start_unix_nanos.to_string(),
            "endTimeUnixNano": phase.end_unix_nanos.to_string(),
        }));
    }
    json!({
        "resourceSpans": [{
            "resource": resource_value,
            "scopeSpans": [{ "scope": { "name": SCOPE_NAME }, "spans": spans }],
        }]
    })
    .to_string()
}

/// Maps one run to its three OTLP JSON lines.
///
/// Gauges: one metric per name with one data point per snapshot point,
/// plus `kiro.run.duration` per phase and `kiro.run.findings.dropped`.
/// Log records: one per finding, errors first, at most `max_records`.
/// Trace: a root span `kiro.gate` with the verdict and finding counts and
/// one child span per phase.
#[must_use]
pub fn otlp_lines(
    snapshot: &MetricSnapshot,
    findings: &[Finding],
    identity: &ResourceIdentity,
    run: &RunIdentity,
    max_records: usize,
) -> OtlpLines {
    let resource_value = resource(identity);
    let (logs, dropped_records) = logs_line(findings, &resource_value, run, max_records);
    OtlpLines {
        metrics: metrics_line(snapshot, &resource_value, run, dropped_records),
        logs,
        traces: traces_line(&resource_value, run, findings, dropped_records),
        dropped_records,
    }
}
