//! Configuration of the code gate (code-constraints requirement 2).
//!
//! The limits, their severities, path scopes and ignores live in one
//! committed file, `.kiro/code-rules.toml`. Without the file the built-in
//! limits apply. Every scope, every ignore and every setting looser than
//! the built-in one needs a `reason`, so that the gate's judgement is
//! reviewable and nobody relaxes it silently.
//!
//! The built-in limits are candidate values carried over from the archived
//! scitools-hook project, where they were measured with a different
//! analyzer. A limit blocks only where it is calibrated on this project's
//! corpus: the five routine metrics in Rust and Python report as errors,
//! every other limit as a warning.
//!
//! ```toml
//! [limits.routine.cyclomatic]
//! max = 12
//! reason = "state machines in the parser"
//!
//! [limits.file.code_lines]
//! severity = "error"
//!
//! [[scopes]]
//! paths = ["crates/kiro-core/src/checks/"]
//! reason = "finitization is one algorithm"
//! [scopes.limits.routine.code_lines]
//! max = 120
//!
//! [[ignores]]
//! paths = ["crates/kiro-code/fixtures/"]
//! reason = "fixtures are measured by tests, not judged"
//! ```

use std::collections::BTreeMap;

use serde::Deserialize;

use super::facts::{Scope, is_valid_path, metrics};
use super::rules::{RawRules, RuleSettings};
use crate::finding::Severity;

/// Project-relative path of the configuration file.
pub const CONFIG_PATH: &str = ".kiro/code-rules.toml";

/// One limit: the value above which a finding is reported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    /// A value equal to the maximum passes; only a greater value breaches.
    pub maximum: f64,
    pub severity: Severity,
    /// `false` when the severity is `off`: the metric is not judged.
    pub enabled: bool,
    /// The metric is judged only when the entity holds at least this many
    /// statements (the lines-per-statement floor).
    pub minimum_population: Option<u32>,
    /// `true` when the configuration names the severity. Otherwise the
    /// severity follows the calibration: see [`calibrated`].
    pub severity_set: bool,
}

/// The routine metrics whose limits are calibrated (owner decision
/// 2026-11-10 on the corpus record in the spec's research.md).
pub const CALIBRATED_ROUTINE_METRICS: &[&str] = &[
    metrics::CYCLOMATIC,
    metrics::NESTING,
    metrics::CODE_LINES,
    metrics::STATEMENTS,
    metrics::PARAMETERS,
];
/// File extensions of the calibrated languages: Rust and Python.
pub const CALIBRATED_EXTENSIONS: &[&str] = &["rs", "py", "pyi"];

/// Whether a limit blocks by default for this metric in this file: a
/// calibrated routine metric in a calibrated language. Everything else
/// stays a warning until its own calibration.
#[must_use]
pub fn calibrated(scope: Scope, metric: &str, path: &str) -> bool {
    calibrated_metric(scope, metric)
        && path
            .rsplit_once('.')
            .is_some_and(|(_, extension)| CALIBRATED_EXTENSIONS.contains(&extension))
}

fn calibrated_metric(scope: Scope, metric: &str) -> bool {
    scope == Scope::Routine && CALIBRATED_ROUTINE_METRICS.contains(&metric)
}

impl Limit {
    const fn warning(maximum: f64) -> Self {
        Self {
            maximum,
            severity: Severity::Warning,
            enabled: true,
            minimum_population: None,
            severity_set: false,
        }
    }

    /// The severity as `code config` shows it for a metric.
    fn severity_text(&self, scope: Scope, metric: &str) -> &'static str {
        if !self.severity_set && calibrated_metric(scope, metric) {
            "error in Rust and Python, warning in other languages"
        } else {
            self.severity_word()
        }
    }

    fn severity_word(&self) -> &'static str {
        if !self.enabled {
            return "off";
        }
        match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// How permissive the severity is: a higher rank is looser.
    fn looseness(&self) -> u8 {
        if !self.enabled {
            return 3;
        }
        match self.severity {
            Severity::Error => 0,
            Severity::Warning => 1,
            Severity::Info => 2,
        }
    }
}

/// Where a limit came from, shown in findings and in `code config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitSource {
    Default,
    File,
    /// The scope path that matched.
    Scope(String),
}

impl LimitSource {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Default => "default".to_string(),
            Self::File => "file".to_string(),
            Self::Scope(path) => format!("scope {path}"),
        }
    }
}

/// A problem in the configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// The key the problem is at, such as `limits.routine.cyclomatic`.
    pub key: String,
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key, self.message)
    }
}

type LimitKey = (Scope, &'static str);

#[derive(Debug, Clone, PartialEq)]
struct Override {
    limit: Limit,
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ScopeRule {
    paths: Vec<String>,
    reason: String,
    limits: BTreeMap<LimitKey, Limit>,
}

/// Files that are measured but not judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreRule {
    pub paths: Vec<String>,
    pub reason: String,
}

/// The effective configuration of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeConfig {
    defaults: BTreeMap<LimitKey, Limit>,
    file: BTreeMap<LimitKey, Override>,
    scopes: Vec<ScopeRule>,
    ignores: Vec<IgnoreRule>,
    rules: RuleSettings,
}

fn built_in() -> BTreeMap<LimitKey, Limit> {
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
    limits
}

impl Default for CodeConfig {
    fn default() -> Self {
        Self {
            defaults: built_in(),
            file: BTreeMap::new(),
            scopes: Vec::new(),
            ignores: Vec::new(),
            rules: RuleSettings::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimit {
    max: Option<f64>,
    severity: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimits {
    #[serde(default)]
    routine: BTreeMap<String, RawLimit>,
    #[serde(default, rename = "type")]
    types: BTreeMap<String, RawLimit>,
    #[serde(default)]
    file: BTreeMap<String, RawLimit>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScope {
    #[serde(default)]
    paths: Vec<String>,
    reason: Option<String>,
    #[serde(default)]
    limits: RawLimits,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIgnore {
    #[serde(default)]
    paths: Vec<String>,
    reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[serde(default)]
    limits: RawLimits,
    #[serde(default)]
    scopes: Vec<RawScope>,
    #[serde(default)]
    ignores: Vec<RawIgnore>,
    #[serde(default)]
    rules: RawRules,
}

fn has_reason(reason: Option<&String>) -> bool {
    reason.is_some_and(|text| !text.trim().is_empty())
}

/// A scope or ignore path: project-relative, `/` separators, an optional
/// trailing `/` for a directory.
fn valid_rule_path(path: &str) -> bool {
    is_valid_path(path.strip_suffix('/').unwrap_or(path))
}

/// Whether a rule path covers a file: the file itself, or a directory
/// prefix ending at a `/` boundary.
fn covers(rule: &str, file: &str) -> bool {
    let rule = rule.strip_suffix('/').unwrap_or(rule);
    file == rule
        || file
            .strip_prefix(rule)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn specificity(rule: &str) -> usize {
    rule.strip_suffix('/').unwrap_or(rule).split('/').count()
}

struct Parser {
    defaults: BTreeMap<LimitKey, Limit>,
    errors: Vec<ConfigError>,
}

impl Parser {
    fn error(&mut self, key: String, message: impl Into<String>) {
        self.errors.push(ConfigError {
            key,
            message: message.into(),
        });
    }

    /// Resolves one raw limit against the built-in one.
    fn limit(&mut self, key: &str, base: Limit, raw: &RawLimit) -> Limit {
        let mut limit = base;
        if let Some(max) = raw.max {
            if max.is_finite() && max >= 0.0 {
                limit.maximum = max;
            } else {
                self.error(
                    format!("{key}.max"),
                    "must be a finite number of at least 0",
                );
            }
        }
        if let Some(word) = &raw.severity {
            limit.severity_set = true;
            match word.as_str() {
                "error" => (limit.severity, limit.enabled) = (Severity::Error, true),
                "warning" => (limit.severity, limit.enabled) = (Severity::Warning, true),
                "info" => (limit.severity, limit.enabled) = (Severity::Info, true),
                "off" => limit.enabled = false,
                _ => self.error(
                    format!("{key}.severity"),
                    "unknown severity; expected error, warning, info or off",
                ),
            }
        }
        limit
    }

    fn limits(
        &mut self,
        prefix: &str,
        raw: &RawLimits,
        reason_required: bool,
    ) -> BTreeMap<LimitKey, Override> {
        let mut out = BTreeMap::new();
        let groups = [
            (Scope::Routine, "routine", &raw.routine),
            (Scope::Type, "type", &raw.types),
            (Scope::File, "file", &raw.file),
        ];
        for (scope, word, entries) in groups {
            for (name, entry) in entries {
                let key = format!("{prefix}.{word}.{name}");
                let known = self
                    .defaults
                    .iter()
                    .find(|((default_scope, default_name), _)| {
                        *default_scope == scope && *default_name == name.as_str()
                    })
                    .map(|((_, static_name), base)| (*static_name, *base));
                let Some((static_name, base)) = known else {
                    self.error(key, format!("unknown {word} metric"));
                    continue;
                };
                let limit = self.limit(&key, base, entry);
                // A calibrated metric blocks by default, so naming any
                // weaker severity for it is a relaxation.
                let strictest = if calibrated_metric(scope, static_name) {
                    0
                } else {
                    base.looseness()
                };
                let looser = limit.maximum > base.maximum
                    || (limit.severity_set && limit.looseness() > strictest);
                if reason_required && looser && !has_reason(entry.reason.as_ref()) {
                    self.error(key, "is looser than the built-in limit and needs a reason");
                }
                out.insert(
                    (scope, static_name),
                    Override {
                        limit,
                        reason: entry.reason.clone(),
                    },
                );
            }
        }
        out
    }

    fn paths(&mut self, key: &str, paths: &[String]) {
        if paths.is_empty() {
            self.error(format!("{key}.paths"), "needs at least one path");
        }
        for path in paths {
            if !valid_rule_path(path) {
                self.error(
                    format!("{key}.paths"),
                    format!("`{path}` is not a project-relative path with `/` separators"),
                );
            }
        }
    }
}

/// Parses the configuration file. `None` means the file is absent: the
/// built-in limits apply. Every problem is returned, each with its key.
pub fn parse(text: Option<&str>) -> Result<CodeConfig, Vec<ConfigError>> {
    let Some(text) = text else {
        return Ok(CodeConfig::default());
    };
    let raw: RawFile = toml::from_str(text).map_err(|error| {
        vec![ConfigError {
            key: CONFIG_PATH.to_string(),
            message: error.to_string().trim().replace('\n', " "),
        }]
    })?;
    let mut parser = Parser {
        defaults: built_in(),
        errors: Vec::new(),
    };
    let file = parser.limits("limits", &raw.limits, true);

    let mut scopes = Vec::new();
    for (index, scope) in raw.scopes.iter().enumerate() {
        let key = format!("scopes[{index}]");
        parser.paths(&key, &scope.paths);
        if !has_reason(scope.reason.as_ref()) {
            parser.error(key.clone(), "every scope needs a reason");
        }
        // The scope's reason covers its limits.
        let limits = parser
            .limits(&format!("{key}.limits"), &scope.limits, false)
            .into_iter()
            .map(|(limit_key, entry)| (limit_key, entry.limit))
            .collect();
        scopes.push(ScopeRule {
            paths: scope.paths.clone(),
            reason: scope.reason.clone().unwrap_or_default(),
            limits,
        });
    }

    let mut ignores = Vec::new();
    for (index, ignore) in raw.ignores.iter().enumerate() {
        let key = format!("ignores[{index}]");
        parser.paths(&key, &ignore.paths);
        if !has_reason(ignore.reason.as_ref()) {
            parser.error(key, "every ignore needs a reason");
        }
        ignores.push(IgnoreRule {
            paths: ignore.paths.clone(),
            reason: ignore.reason.clone().unwrap_or_default(),
        });
    }

    let rules = RuleSettings::parse(&raw.rules, &mut parser.errors);
    if !parser.errors.is_empty() {
        return Err(parser.errors);
    }
    Ok(CodeConfig {
        defaults: parser.defaults,
        file,
        scopes,
        ignores,
        rules,
    })
}

fn fnv1a(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The entry of `metric` in `limits`. The keys hold static names, so a
/// borrowed name is compared entry by entry.
fn lookup<'a, V>(limits: &'a BTreeMap<LimitKey, V>, scope: Scope, metric: &str) -> Option<&'a V> {
    limits
        .iter()
        .find(|((limit_scope, name), _)| *limit_scope == scope && *name == metric)
        .map(|(_, value)| value)
}

impl CodeConfig {
    /// The limit of `metric` for an entity in `path`: the most specific
    /// matching scope wins (the later one on a tie), then the file, then
    /// the built-in limit. `None` when the metric has no limit or is off.
    #[must_use]
    pub fn limit_for(
        &self,
        scope: Scope,
        metric: &str,
        path: &str,
    ) -> Option<(Limit, LimitSource)> {
        let (mut limit, source) =
            if let Some((limit, rule_path)) = self.scope_limit(scope, metric, path) {
                (limit, LimitSource::Scope(rule_path.to_string()))
            } else if let Some(entry) = lookup(&self.file, scope, metric) {
                (entry.limit, LimitSource::File)
            } else {
                (
                    *lookup(&self.defaults, scope, metric)?,
                    LimitSource::Default,
                )
            };
        if !limit.severity_set && calibrated(scope, metric, path) {
            limit.severity = Severity::Error;
        }
        limit.enabled.then_some((limit, source))
    }

    /// The scope limit of `metric` for `path` and the rule path it comes
    /// from: the most specific covering path wins, the later one on a tie.
    fn scope_limit(&self, scope: Scope, metric: &str, path: &str) -> Option<(Limit, &str)> {
        let mut best: Option<(usize, Limit, &str)> = None;
        for rule in &self.scopes {
            let Some(limit) = lookup(&rule.limits, scope, metric) else {
                continue;
            };
            for rule_path in rule.paths.iter().filter(|rule| covers(rule, path)) {
                let depth = specificity(rule_path);
                if best.is_none_or(|(current, _, _)| depth >= current) {
                    best = Some((depth, *limit, rule_path.as_str()));
                }
            }
        }
        best.map(|(_, limit, rule_path)| (limit, rule_path))
    }

    /// The settings of the rules that count across files.
    #[must_use]
    pub fn rules(&self) -> &RuleSettings {
        &self.rules
    }

    /// The ignore rule that covers `path`, if any.
    #[must_use]
    pub fn ignored(&self, path: &str) -> Option<&IgnoreRule> {
        self.ignores
            .iter()
            .find(|rule| rule.paths.iter().any(|rule_path| covers(rule_path, path)))
    }

    /// Every effective value with where it came from and its reason, one
    /// per line, in a stable order (code-constraints 2.6).
    #[must_use]
    pub fn effective_text(&self) -> String {
        let mut out = String::new();
        for ((scope, name), base) in &self.defaults {
            let (limit, origin, reason) = match self.file.get(&(*scope, *name)) {
                Some(entry) => (entry.limit, "file", entry.reason.as_deref()),
                None => (*base, "default", None),
            };
            out.push_str(&format!(
                "{} {name}: max {}, {} ({origin})",
                scope.label(),
                limit.maximum,
                limit.severity_text(*scope, name)
            ));
            if let Some(reason) = reason {
                out.push_str(&format!(" reason: {reason}"));
            }
            out.push('\n');
        }
        for rule in &self.scopes {
            for ((scope, name), limit) in &rule.limits {
                out.push_str(&format!(
                    "scope {}: {} {name}: max {}, {} reason: {}\n",
                    rule.paths.join(", "),
                    scope.label(),
                    limit.maximum,
                    limit.severity_text(*scope, name),
                    rule.reason
                ));
            }
        }
        out.push_str(&self.rules.effective_text());
        for rule in &self.ignores {
            out.push_str(&format!(
                "ignore {} reason: {}\n",
                rule.paths.join(", "),
                rule.reason
            ));
        }
        out
    }

    /// A stable digest of the effective configuration.
    #[must_use]
    pub fn digest(&self) -> String {
        format!("{:016x}", fnv1a(&self.effective_text()))
    }
}
