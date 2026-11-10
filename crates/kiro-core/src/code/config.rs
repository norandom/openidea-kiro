//! Limits of the code gate (code-constraints 2.1, 2.2, 3.1, 3.2, 4.1, 4.2).
//!
//! The built-in limits are candidate values carried over from the archived
//! scitools-hook project, where they were measured with a different
//! analyzer. They are priors: until a metric definition and language are
//! calibrated on this project's corpus every limit reports as a warning.

use std::collections::BTreeMap;

use super::facts::{Scope, metrics};
use crate::finding::Severity;

/// One limit: the value above which a finding is reported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    /// A value equal to the maximum passes; only a greater value breaches.
    pub maximum: f64,
    pub severity: Severity,
    /// The metric is judged only when the entity holds at least this many
    /// statements (the lines-per-statement floor).
    pub minimum_population: Option<u32>,
}

impl Limit {
    const fn warning(maximum: f64) -> Self {
        Self {
            maximum,
            severity: Severity::Warning,
            minimum_population: None,
        }
    }
}

/// Where a limit came from, shown in findings and in `code config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitSource {
    Default,
}

impl LimitSource {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Default => "default",
        }
    }
}

/// The effective configuration of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeConfig {
    limits: BTreeMap<(Scope, &'static str), Limit>,
}

impl Default for CodeConfig {
    fn default() -> Self {
        let mut limits = BTreeMap::new();
        let routine = [
            (metrics::CYCLOMATIC, 10.0),
            (metrics::NESTING, 3.0),
            (metrics::CODE_LINES, 60.0),
            (metrics::STATEMENTS, 40.0),
            (metrics::PARAMETERS, 5.0),
        ];
        for (name, maximum) in routine {
            limits.insert((Scope::Routine, name), Limit::warning(maximum));
        }
        limits.insert(
            (Scope::Routine, metrics::LINES_PER_STATEMENT),
            Limit {
                minimum_population: Some(5),
                ..Limit::warning(4.0)
            },
        );
        let file = [
            (metrics::CODE_LINES, 500.0),
            (metrics::FUNCTIONS, 25.0),
            (metrics::TYPES, 8.0),
            (metrics::MAX_CYCLOMATIC, 10.0),
        ];
        for (name, maximum) in file {
            limits.insert((Scope::File, name), Limit::warning(maximum));
        }
        for (name, maximum) in [(metrics::METHODS, 20.0), (metrics::FIELDS, 10.0)] {
            limits.insert((Scope::Type, name), Limit::warning(maximum));
        }
        Self { limits }
    }
}

impl CodeConfig {
    /// The limit of `metric` in `scope`, when one is configured.
    #[must_use]
    pub fn limit(&self, scope: Scope, metric: &str) -> Option<(&Limit, LimitSource)> {
        self.limits
            .iter()
            .find(|((limit_scope, name), _)| *limit_scope == scope && *name == metric)
            .map(|(_, limit)| (limit, LimitSource::Default))
    }

    /// Every configured limit in a stable order.
    pub fn limits(&self) -> impl Iterator<Item = (Scope, &'static str, &Limit)> {
        self.limits
            .iter()
            .map(|((scope, name), limit)| (*scope, *name, limit))
    }
}
