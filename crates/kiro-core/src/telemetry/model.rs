//! Plain data of the metric snapshot (quality-telemetry 1.1, 1.3, 1.5, 1.6).
//!
//! Nothing here reads a clock, the network or the filesystem.

/// Unit of a metric value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unit {
    /// A number of things.
    Count,
    /// A share between 0 and 1.
    Ratio,
    /// Source lines.
    Lines,
    /// Source statements.
    Statements,
    /// A duration in milliseconds.
    Milliseconds,
    /// A dimensionless measured value, or a state encoded as 1.
    Value,
}

impl Unit {
    /// The unit string written into an OpenTelemetry metric (UCUM style).
    #[must_use]
    pub const fn ucum(self) -> &'static str {
        match self {
            Self::Count | Self::Ratio | Self::Value => "1",
            Self::Lines => "{line}",
            Self::Statements => "{statement}",
            Self::Milliseconds => "ms",
        }
    }

    /// The word used for the unit in the catalogue documentation.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Ratio => "ratio",
            Self::Lines => "lines",
            Self::Statements => "statements",
            Self::Milliseconds => "milliseconds",
            Self::Value => "value",
        }
    }
}

/// One attribute of a metric point. Keys come from the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Attr {
    pub key: &'static str,
    pub value: String,
}

impl Attr {
    #[must_use]
    pub fn new(key: &'static str, value: impl Into<String>) -> Self {
        Self {
            key,
            value: value.into(),
        }
    }
}

/// One named value of a gate run.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    pub name: &'static str,
    pub unit: Unit,
    pub value: f64,
    pub attrs: Vec<Attr>,
}

impl MetricPoint {
    #[must_use]
    pub fn new(name: &'static str, unit: Unit, value: f64, attrs: Vec<Attr>) -> Self {
        Self {
            name,
            unit,
            value,
            attrs,
        }
    }
}

/// Whether a source contributed to the snapshot, and what was left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCoverage {
    pub source: &'static str,
    /// `false` when the source was expected but no provider supplied it.
    pub present: bool,
    /// Why the source is absent, or which of its points were dropped.
    pub reason: Option<String>,
}

/// The metric points of one gate run, in a stable order.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricSnapshot {
    pub schema_version: u32,
    pub points: Vec<MetricPoint>,
    pub coverage: Vec<SourceCoverage>,
}

/// A report that contributes metric points.
///
/// Other specifications implement this over their own report types; the
/// snapshot never imports those types (quality-telemetry 1.6).
pub trait MetricProvider {
    /// The source name listed in the snapshot's coverage, such as `"spec"`.
    fn source(&self) -> &'static str;
    /// The points of this report. An absent value is omitted, never zero.
    fn points(&self) -> Vec<MetricPoint>;
}
