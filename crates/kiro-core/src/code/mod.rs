//! The code gate's pure policy (spec `code-constraints`).
//!
//! Native measurement lives in the `kiro-code` crate and hands over
//! [`CodeFacts`]. Everything here is pure: configuration, limits, pairing
//! with a before side, ownership, report. The engine reads no file and
//! parses no source code.

pub mod attribution;
pub mod clones;
pub mod config;
pub mod evaluate;
pub mod facts;
pub mod limits;
pub mod ratchet;
pub mod report;
pub mod telemetry;

pub use attribution::{
    Attribution, SelectionError, attribute, matches_any, spec_paths, task_paths,
};
pub use config::{
    CONFIG_PATH, CodeConfig, ConfigError, IgnoreRule, Limit, LimitSource, parse as parse_config,
};
pub use evaluate::{EvaluateInput, evaluate};
pub use facts::{
    CodeFacts, Completeness, CoverageCell, EntityKey, FACTS_SCHEMA_VERSION, Fingerprint, MetricId,
    Observation, ProducerId, Scope, metrics,
};
pub use limits::{Judgement, highest, judge};
pub use ratchet::{Before, Earlier, Pairing, Trust, pair};
pub use report::{Change, CodeFinding, CodeReport, Coverage, Measure, NetDelta, SpecSummary, Tag};
