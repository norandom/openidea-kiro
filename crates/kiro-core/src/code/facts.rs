//! Versioned code observations (code-constraints 1.3, 5.1, 11.1, 11.2).
//!
//! A fact is one measured value of one entity, with the producer and the
//! metric definition that give it meaning. A value that was not measured is
//! absent and described by a coverage cell; it is never zero.

use serde::{Deserialize, Serialize};

/// Version of the fact model's shape and meaning.
pub const FACTS_SCHEMA_VERSION: u32 = 2;

/// Metric names of the base profile.
pub mod metrics {
    pub const CYCLOMATIC: &str = "cyclomatic";
    pub const NESTING: &str = "nesting";
    pub const CODE_LINES: &str = "code_lines";
    pub const STATEMENTS: &str = "statements";
    pub const PARAMETERS: &str = "parameters";
    /// Derived by the limits from code lines and statements.
    pub const LINES_PER_STATEMENT: &str = "lines_per_statement";
    pub const FUNCTIONS: &str = "functions";
    pub const TYPES: &str = "types";
    pub const MAX_CYCLOMATIC: &str = "max_cyclomatic";
    pub const METHODS: &str = "methods";
    pub const FIELDS: &str = "fields";
}

/// What kind of entity a value belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Routine,
    Type,
    File,
}

impl Scope {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Routine => "routine",
            Self::Type => "type",
            Self::File => "file",
        }
    }
}

/// Identity of a measured entity, stable across two measurements.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityKey {
    pub scope: Scope,
    /// Project-relative path with `/` separators.
    pub path: String,
    /// Nesting-qualified name; the path again for a file.
    pub qualified_name: String,
    /// Parameter list text with whitespace collapsed; empty for types and files.
    pub parameters: String,
    /// Document-order index among entities equal in the fields above.
    pub ordinal: u32,
}

/// A metric name with the version of its definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MetricId {
    pub name: String,
    pub definition_version: String,
}

/// Who produced a value: tool, version and grammar.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProducerId {
    pub name: String,
    pub version: String,
    pub grammar: String,
}

/// Whether a file or metric was measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Complete,
    Partial,
    Unavailable,
    Failed,
    NotApplicable,
}

/// One measured value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub entity: EntityKey,
    pub metric: MetricId,
    pub value: f64,
    pub producer: ProducerId,
    /// 1-based line where the entity starts.
    pub line: u32,
}

/// Measurement state of a file, or of one metric in a file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoverageCell {
    pub path: String,
    /// `None` covers every metric of the file.
    pub metric: Option<MetricId>,
    pub state: Completeness,
    pub reason: Option<String>,
    /// 1-based line of the first problem, for a failed file.
    pub line: Option<u32>,
}

/// The observations and coverage of one measurement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeFacts {
    pub schema_version: u32,
    pub observations: Vec<Observation>,
    pub coverage: Vec<CoverageCell>,
}

impl Default for CodeFacts {
    fn default() -> Self {
        Self {
            schema_version: FACTS_SCHEMA_VERSION,
            observations: Vec::new(),
            coverage: Vec::new(),
        }
    }
}

/// Whether a path is project-relative with `/` separators and no escape.
#[must_use]
pub fn is_valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl CodeFacts {
    /// Problems that make the facts unusable: nonfinite values, invalid
    /// paths and two different values for the same entity, metric and
    /// producer. An empty list means the facts are valid.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (index, observation) in self.observations.iter().enumerate() {
            if !observation.value.is_finite() {
                out.push(format!(
                    "{} {}: value is not finite",
                    observation.entity.path, observation.metric.name
                ));
            }
            if !is_valid_path(&observation.entity.path) {
                out.push(format!("invalid path `{}`", observation.entity.path));
            }
            let conflict = self.observations.iter().take(index).any(|earlier| {
                earlier.entity == observation.entity
                    && earlier.metric == observation.metric
                    && earlier.producer == observation.producer
                    && earlier.value.to_bits() != observation.value.to_bits()
            });
            if conflict {
                out.push(format!(
                    "{} {} {}: two different values",
                    observation.entity.path,
                    observation.entity.qualified_name,
                    observation.metric.name
                ));
            }
        }
        for cell in &self.coverage {
            if !is_valid_path(&cell.path) {
                out.push(format!("invalid path `{}`", cell.path));
            }
        }
        out
    }

    /// Puts observations and coverage in their canonical order.
    pub fn sort(&mut self) {
        self.observations.sort_by(|a, b| {
            (&a.entity, &a.metric, &a.producer).cmp(&(&b.entity, &b.metric, &b.producer))
        });
        self.coverage.sort();
    }

    /// The value of `metric` for `entity`, when it was measured.
    #[must_use]
    pub fn value(&self, entity: &EntityKey, metric: &str) -> Option<f64> {
        self.observations
            .iter()
            .find(|observation| &observation.entity == entity && observation.metric.name == metric)
            .map(|observation| observation.value)
    }

    /// The file-level coverage state of `path`, when one was recorded.
    #[must_use]
    pub fn file_state(&self, path: &str) -> Option<&CoverageCell> {
        self.coverage
            .iter()
            .find(|cell| cell.path == path && cell.metric.is_none())
    }
}
