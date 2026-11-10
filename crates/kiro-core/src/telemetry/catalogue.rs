//! The metric catalogue (quality-telemetry 1.2, 1.4).
//!
//! Every metric the gate may emit is declared here with its unit, the
//! attribute keys it may carry, the report it comes from and its meaning.
//! `docs/telemetry.md` carries the same table; a test keeps them equal.

use super::model::Unit;

/// Attribute keys a metric point may carry. All have bounded value sets;
/// routine names, file paths and message texts are never attribute keys.
pub const ATTRIBUTE_KEYS: &[&str] = &[
    "kiro.spec",
    "kiro.component",
    "kiro.task",
    "kiro.code",
    "kiro.severity",
    "kiro.metric",
    "kiro.language",
    "kiro.phase",
    "kiro.document",
    "kiro.status",
];

/// Name prefixes reserved for metrics that other specifications define.
pub const RESERVED_PREFIXES: &[(&str, &str)] = &[
    ("kiro.code.", "code-constraints"),
    ("kiro.clone.", "clone-constraints"),
    ("kiro.locality.", "code-locality-data"),
    ("kiro.evidence.", "verification-evidence"),
];

/// One catalogue entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricInfo {
    pub name: &'static str,
    pub unit: Unit,
    /// Attribute keys a point of this metric may carry.
    pub attributes: &'static [&'static str],
    /// The report the value is read from.
    pub source: &'static str,
    pub meaning: &'static str,
}

const SPEC: &[&str] = &["kiro.spec"];

/// Every metric, sorted by name.
pub const METRICS: &[MetricInfo] = &[
    MetricInfo {
        name: "kiro.code.breaches",
        unit: Unit::Count,
        attributes: &["kiro.spec", "kiro.code", "kiro.severity"],
        source: "code",
        meaning: "Code findings that are not pre-existing, by finding code and severity",
    },
    MetricInfo {
        name: "kiro.code.changed",
        unit: Unit::Count,
        attributes: &["kiro.metric", "kiro.status"],
        source: "code",
        meaning: "Routine metrics that differ from the before side: improved or worsened",
    },
    MetricInfo {
        name: "kiro.code.files",
        unit: Unit::Count,
        attributes: &["kiro.status"],
        source: "code",
        meaning: "Selected files by state: measured, failed, unmeasured, ignored",
    },
    MetricInfo {
        name: "kiro.code.highest",
        unit: Unit::Value,
        attributes: &["kiro.spec", "kiro.metric"],
        source: "code",
        meaning: "Highest value of a routine metric over the judged files of a spec",
    },
    MetricInfo {
        name: "kiro.code.pre_existing",
        unit: Unit::Count,
        attributes: SPEC,
        source: "code",
        meaning: "Breaches that existed before and did not worsen",
    },
    MetricInfo {
        name: "kiro.code.routines",
        unit: Unit::Count,
        attributes: SPEC,
        source: "code",
        meaning: "Routines measured in the judged files of a spec",
    },
    MetricInfo {
        name: "kiro.code.total",
        unit: Unit::Value,
        attributes: &["kiro.spec", "kiro.metric"],
        source: "code",
        meaning: "Sum of a routine metric over the judged files of a spec",
    },
    MetricInfo {
        name: "kiro.run.duration",
        unit: Unit::Milliseconds,
        attributes: &["kiro.phase"],
        source: "run",
        meaning: "Duration of one phase of a gate run",
    },
    MetricInfo {
        name: "kiro.run.findings.dropped",
        unit: Unit::Count,
        attributes: &[],
        source: "run",
        meaning: "Finding records left out because the per-run maximum was reached",
    },
    MetricInfo {
        name: "kiro.spec.approved",
        unit: Unit::Value,
        attributes: &["kiro.spec", "kiro.document"],
        source: "status",
        meaning: "1 when the document of a spec is approved, otherwise 0",
    },
    MetricInfo {
        name: "kiro.spec.contradiction.pairs",
        unit: Unit::Count,
        attributes: &["kiro.spec", "kiro.status"],
        source: "check",
        meaning: "Criterion pairs by contradiction search outcome: considered, found, refused",
    },
    MetricInfo {
        name: "kiro.spec.criteria.checked",
        unit: Unit::Count,
        attributes: SPEC,
        source: "check",
        meaning: "Acceptance criteria the constraint checks covered",
    },
    MetricInfo {
        name: "kiro.spec.criteria.total",
        unit: Unit::Count,
        attributes: SPEC,
        source: "check",
        meaning: "Acceptance criteria of a spec",
    },
    MetricInfo {
        name: "kiro.spec.findings",
        unit: Unit::Count,
        attributes: &["kiro.spec", "kiro.code", "kiro.severity"],
        source: "check",
        meaning: "Findings by finding code and severity",
    },
    MetricInfo {
        name: "kiro.spec.phase",
        unit: Unit::Value,
        attributes: &["kiro.spec", "kiro.phase"],
        source: "status",
        meaning: "1 for the phase a spec is in",
    },
    MetricInfo {
        name: "kiro.spec.tasks.done",
        unit: Unit::Count,
        attributes: SPEC,
        source: "status",
        meaning: "Completed tasks of a spec",
    },
    MetricInfo {
        name: "kiro.spec.tasks.total",
        unit: Unit::Count,
        attributes: SPEC,
        source: "status",
        meaning: "Tasks of a spec",
    },
    MetricInfo {
        name: "kiro.spec.terms",
        unit: Unit::Count,
        attributes: &["kiro.spec", "kiro.status"],
        source: "check",
        meaning: "Terms by state: declared, resolved, deprecated, mismatched, undeclared, ambiguous, suggested",
    },
    MetricInfo {
        name: "kiro.trace.components.unmapped",
        unit: Unit::Count,
        attributes: SPEC,
        source: "join",
        meaning: "Design components with no file",
    },
    MetricInfo {
        name: "kiro.trace.criteria.untasked",
        unit: Unit::Count,
        attributes: SPEC,
        source: "join",
        meaning: "Acceptance criteria referenced by no task",
    },
    MetricInfo {
        name: "kiro.trace.files.multi_owned",
        unit: Unit::Count,
        attributes: SPEC,
        source: "join",
        meaning: "Files mapped by more than one spec",
    },
    MetricInfo {
        name: "kiro.trace.tasks.unknown_boundary",
        unit: Unit::Count,
        attributes: SPEC,
        source: "join",
        meaning: "Tasks naming a boundary that no design component matches",
    },
];

/// The catalogue entry of a metric name.
#[must_use]
pub fn lookup(name: &str) -> Option<&'static MetricInfo> {
    METRICS.iter().find(|info| info.name == name)
}

/// The name of the metrics stream OpenObserve creates for a metric: dots
/// become underscores (observed in the T0 prototype, see research.md).
#[must_use]
pub fn stream_name(metric: &str) -> String {
    metric.replace('.', "_")
}

/// The field name OpenObserve gives an attribute key in metrics and logs.
/// In traces, resource attributes additionally carry a `service_` prefix.
#[must_use]
pub fn field_name(attribute: &str) -> String {
    attribute.replace('.', "_")
}

/// The catalogue as the Markdown table that `docs/telemetry.md` contains.
#[must_use]
pub fn catalogue_markdown() -> String {
    let mut out = String::from(
        "| Metric | Unit | Attributes | Source | Meaning |\n| --- | --- | --- | --- | --- |\n",
    );
    for info in METRICS {
        let attributes = if info.attributes.is_empty() {
            "none".to_string()
        } else {
            info.attributes
                .iter()
                .map(|key| format!("`{key}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            info.name,
            info.unit.label(),
            attributes,
            info.source,
            info.meaning
        ));
    }
    out
}
