//! Settings of the rules that count across files (clone-constraints 3.1,
//! code-locality-data 1.3): duplicate blocks, similar-routine families,
//! scattered definitions, scattered state and positional data access.
//!
//! They live in the same file as the limits, under `[rules.<name>]`:
//!
//! ```toml
//! [rules.duplicate_block]
//! min_lines = 20
//! reason = "generated tables repeat short runs"
//!
//! [rules.similar_family]
//! severity = "off"
//! reason = "not adopted yet"
//!
//! [rules.scattered_definition]
//! ignored_names = ["VERSION"]
//! reason = "every package states its own version"
//! ```
//!
//! A setting that makes a rule report less than the built-in one needs a
//! `reason`: a weaker severity, a higher threshold, an added ignored
//! name. A stricter setting needs none.

use serde::Deserialize;

use super::clones::DEFAULT_MIN_LINES;
use super::config::ConfigError;
use super::families::{DEFAULT_MIN_STATEMENTS, DEFAULT_THRESHOLD};
use super::locality::{DEFAULT_MAX_FILES, DEFAULT_MIN_STATES};
use super::report::CodeFinding;
use crate::finding::Severity;

/// Whether a rule reports, and how severe its findings are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub severity: Severity,
    /// `false` when the severity is `off`: the rule does not run.
    pub enabled: bool,
}

impl Level {
    const fn of(severity: Severity) -> Self {
        Self {
            severity,
            enabled: true,
        }
    }

    fn word(self) -> &'static str {
        if !self.enabled {
            return "off";
        }
        match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// How permissive the level is: a higher rank reports less.
    fn looseness(self) -> u8 {
        if !self.enabled {
            return 3;
        }
        match self.severity {
            Severity::Error => 0,
            Severity::Warning => 1,
            Severity::Info => 2,
        }
    }

    /// Gives the findings of a rule this level; none when the rule is off.
    #[must_use]
    pub fn apply(self, findings: Vec<CodeFinding>) -> Vec<CodeFinding> {
        if !self.enabled {
            return Vec::new();
        }
        findings
            .into_iter()
            .map(|mut entry| {
                entry.finding.severity = self.severity;
                entry
            })
            .collect()
    }
}

/// The effective settings of the rules that count across files.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleSettings {
    pub duplicate_block: Level,
    pub min_lines: usize,
    pub similar_family: Level,
    pub threshold: f64,
    pub min_statements: u32,
    /// Routine names ignored in addition to the built-in ones.
    pub ignored_routines: Vec<String>,
    pub scattered_definition: Level,
    /// Files a definition may stand in before it is scattered.
    pub definition_max_files: usize,
    /// Binding names ignored in addition to the built-in ones.
    pub ignored_bindings: Vec<String>,
    pub scattered_state: Level,
    pub state_max_files: usize,
    pub min_states: usize,
    pub positional_access: Level,
    /// The reasons given, by rule name, for `code config`.
    reasons: Vec<(&'static str, String)>,
}

impl Default for RuleSettings {
    fn default() -> Self {
        Self {
            duplicate_block: Level::of(Severity::Warning),
            min_lines: DEFAULT_MIN_LINES,
            similar_family: Level::of(Severity::Warning),
            threshold: DEFAULT_THRESHOLD,
            min_statements: DEFAULT_MIN_STATEMENTS,
            ignored_routines: Vec::new(),
            scattered_definition: Level::of(Severity::Warning),
            definition_max_files: DEFAULT_MAX_FILES,
            ignored_bindings: Vec::new(),
            scattered_state: Level::of(Severity::Info),
            state_max_files: DEFAULT_MAX_FILES,
            min_states: DEFAULT_MIN_STATES,
            positional_access: Level::of(Severity::Warning),
            reasons: Vec::new(),
        }
    }
}

/// One `[rules.<name>]` table. Each rule reads the keys it knows; a key
/// that a rule does not have is an error.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRule {
    severity: Option<String>,
    min_lines: Option<usize>,
    threshold: Option<f64>,
    min_statements: Option<u32>,
    max_files: Option<usize>,
    min_states: Option<usize>,
    ignored_names: Option<Vec<String>>,
    reason: Option<String>,
}

/// The `[rules]` table.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRules {
    duplicate_block: Option<RawRule>,
    similar_family: Option<RawRule>,
    scattered_definition: Option<RawRule>,
    scattered_state: Option<RawRule>,
    positional_access: Option<RawRule>,
}

/// Reads one rule table: collects errors and notes whether any setting
/// reports less than the built-in one.
struct Reader<'a> {
    name: &'static str,
    raw: &'a RawRule,
    looser: bool,
    errors: &'a mut Vec<ConfigError>,
}

impl Reader<'_> {
    fn error(&mut self, key: &str, message: &str) {
        self.errors.push(ConfigError {
            key: format!("rules.{}.{key}", self.name),
            message: message.to_string(),
        });
    }

    fn level(&mut self, built_in: Level) -> Level {
        let Some(word) = self.raw.severity.as_deref() else {
            return built_in;
        };
        let level = match word {
            "error" => Level::of(Severity::Error),
            "warning" => Level::of(Severity::Warning),
            "info" => Level::of(Severity::Info),
            "off" => Level {
                enabled: false,
                ..built_in
            },
            _ => {
                self.error(
                    "severity",
                    "unknown severity; expected error, warning, info or off",
                );
                built_in
            }
        };
        self.looser |= level.looseness() > built_in.looseness();
        level
    }

    /// A whole-number setting; a higher value reports less.
    fn count(&mut self, key: &str, given: Option<usize>, built_in: usize) -> usize {
        let Some(value) = given else {
            return built_in;
        };
        if value == 0 {
            self.error(key, "must be at least 1");
            return built_in;
        }
        self.looser |= value > built_in;
        value
    }

    fn threshold(&mut self, built_in: f64) -> f64 {
        let Some(value) = self.raw.threshold else {
            return built_in;
        };
        if !(value > 0.0 && value <= 1.0) {
            self.error("threshold", "must be greater than 0 and at most 1");
            return built_in;
        }
        self.looser |= value > built_in;
        value
    }

    fn ignored_names(&mut self) -> Vec<String> {
        let names = self.raw.ignored_names.clone().unwrap_or_default();
        if names.iter().any(|name| name.trim().is_empty()) {
            self.error("ignored_names", "a name must not be empty");
        }
        self.looser |= !names.is_empty();
        names
    }

    /// Rejects keys the rule does not have.
    fn only(&mut self, allowed: &[&str]) {
        let given = [
            ("min_lines", self.raw.min_lines.is_some()),
            ("threshold", self.raw.threshold.is_some()),
            ("min_statements", self.raw.min_statements.is_some()),
            ("max_files", self.raw.max_files.is_some()),
            ("min_states", self.raw.min_states.is_some()),
            ("ignored_names", self.raw.ignored_names.is_some()),
        ];
        for (key, present) in given {
            if present && !allowed.contains(&key) {
                self.error(key, "this rule has no such setting");
            }
        }
    }

    /// Requires the reason when something reports less, and returns it.
    fn reason(&mut self) -> Option<String> {
        let reason = self
            .raw
            .reason
            .clone()
            .filter(|text| !text.trim().is_empty());
        if self.looser && reason.is_none() {
            self.errors.push(ConfigError {
                key: format!("rules.{}", self.name),
                message: "reports less than the built-in rule and needs a reason".to_string(),
            });
        }
        reason
    }
}

impl RuleSettings {
    fn read<'a>(
        name: &'static str,
        raw: Option<&'a RawRule>,
        errors: &'a mut Vec<ConfigError>,
    ) -> Option<Reader<'a>> {
        raw.map(|raw| Reader {
            name,
            raw,
            looser: false,
            errors,
        })
    }

    fn keep(&mut self, name: &'static str, reason: Option<String>) {
        if let Some(reason) = reason {
            self.reasons.push((name, reason));
        }
    }

    fn duplicate_block(&mut self, raw: Option<&RawRule>, errors: &mut Vec<ConfigError>) {
        let Some(mut reader) = Self::read("duplicate_block", raw, errors) else {
            return;
        };
        reader.only(&["min_lines"]);
        self.duplicate_block = reader.level(self.duplicate_block);
        self.min_lines = reader.count("min_lines", reader.raw.min_lines, self.min_lines);
        let reason = reader.reason();
        self.keep("duplicate_block", reason);
    }

    fn similar_family(&mut self, raw: Option<&RawRule>, errors: &mut Vec<ConfigError>) {
        let Some(mut reader) = Self::read("similar_family", raw, errors) else {
            return;
        };
        reader.only(&["threshold", "min_statements", "ignored_names"]);
        self.similar_family = reader.level(self.similar_family);
        self.threshold = reader.threshold(self.threshold);
        let floor = reader
            .raw
            .min_statements
            .and_then(|v| usize::try_from(v).ok());
        let built_in = usize::try_from(self.min_statements).unwrap_or(usize::MAX);
        let floor = reader.count("min_statements", floor, built_in);
        self.min_statements = u32::try_from(floor).unwrap_or(u32::MAX);
        self.ignored_routines = reader.ignored_names();
        let reason = reader.reason();
        self.keep("similar_family", reason);
    }

    fn scattered_definition(&mut self, raw: Option<&RawRule>, errors: &mut Vec<ConfigError>) {
        let Some(mut reader) = Self::read("scattered_definition", raw, errors) else {
            return;
        };
        reader.only(&["max_files", "ignored_names"]);
        self.scattered_definition = reader.level(self.scattered_definition);
        self.definition_max_files =
            reader.count("max_files", reader.raw.max_files, self.definition_max_files);
        self.ignored_bindings = reader.ignored_names();
        let reason = reader.reason();
        self.keep("scattered_definition", reason);
    }

    fn scattered_state(&mut self, raw: Option<&RawRule>, errors: &mut Vec<ConfigError>) {
        let Some(mut reader) = Self::read("scattered_state", raw, errors) else {
            return;
        };
        reader.only(&["max_files", "min_states"]);
        self.scattered_state = reader.level(self.scattered_state);
        self.state_max_files =
            reader.count("max_files", reader.raw.max_files, self.state_max_files);
        self.min_states = reader.count("min_states", reader.raw.min_states, self.min_states);
        let reason = reader.reason();
        self.keep("scattered_state", reason);
    }

    fn positional_access(&mut self, raw: Option<&RawRule>, errors: &mut Vec<ConfigError>) {
        let Some(mut reader) = Self::read("positional_access", raw, errors) else {
            return;
        };
        reader.only(&[]);
        self.positional_access = reader.level(self.positional_access);
        let reason = reader.reason();
        self.keep("positional_access", reason);
    }

    /// Resolves the `[rules]` table against the built-in settings.
    pub(super) fn parse(raw: &RawRules, errors: &mut Vec<ConfigError>) -> Self {
        let mut settings = Self::default();
        settings.duplicate_block(raw.duplicate_block.as_ref(), errors);
        settings.similar_family(raw.similar_family.as_ref(), errors);
        settings.scattered_definition(raw.scattered_definition.as_ref(), errors);
        settings.scattered_state(raw.scattered_state.as_ref(), errors);
        settings.positional_access(raw.positional_access.as_ref(), errors);
        settings
    }

    fn reason_of(&self, name: &str) -> String {
        self.reasons
            .iter()
            .find(|(rule, _)| *rule == name)
            .map(|(_, reason)| format!(" reason: {reason}"))
            .unwrap_or_default()
    }

    /// Every effective rule setting, one rule per line, in a fixed order.
    #[must_use]
    pub fn effective_text(&self) -> String {
        let names = |list: &[String]| {
            if list.is_empty() {
                String::new()
            } else {
                format!(", ignored names {}", list.join(" "))
            }
        };
        let lines = [
            (
                "duplicate_block",
                self.duplicate_block,
                format!("min_lines {}", self.min_lines),
            ),
            (
                "similar_family",
                self.similar_family,
                format!(
                    "threshold {}, min_statements {}{}",
                    self.threshold,
                    self.min_statements,
                    names(&self.ignored_routines)
                ),
            ),
            (
                "scattered_definition",
                self.scattered_definition,
                format!(
                    "max_files {}{}",
                    self.definition_max_files,
                    names(&self.ignored_bindings)
                ),
            ),
            (
                "scattered_state",
                self.scattered_state,
                format!(
                    "max_files {}, min_states {}",
                    self.state_max_files, self.min_states
                ),
            ),
            ("positional_access", self.positional_access, String::new()),
        ];
        lines
            .into_iter()
            .map(|(name, level, values)| {
                let values = if values.is_empty() {
                    String::new()
                } else {
                    format!(", {values}")
                };
                format!(
                    "rule {name}: {}{values}{}\n",
                    level.word(),
                    self.reason_of(name)
                )
            })
            .collect()
    }
}
