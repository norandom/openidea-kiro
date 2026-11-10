//! Scattered definitions and scattered state (code-locality-data 1 and 2).
//!
//! Two things an agent does when it cannot see the whole project: it
//! writes the same constant again in every file that needs it, and it
//! spells the states of one concept as literals wherever it branches on
//! them. Both are found by counting across files.
//!
//! - **Scattered definition**: the same name bound to the same initializer
//!   text at module level in more than `max_files` files.
//! - **Scattered state**: one subject compared with at least `min_states`
//!   distinct literal values, with the comparisons spread over more than
//!   `max_files` files.
//!
//! Both rules are textual. A subject is its spelling: two unrelated
//! variables called `status` are one subject here. That is why scattered
//! state is reported as information, not as a warning. The facts come from
//! the native crate; this module is pure.

use std::collections::{BTreeMap, BTreeSet};

use super::attribution::Attribution;
use super::report::{CodeFinding, Tag};
use crate::finding::{Finding, Severity, codes};

/// More files than this make a definition or a state scattered.
pub const DEFAULT_MAX_FILES: usize = 3;
/// Fewest distinct literal values that make a subject a state.
pub const DEFAULT_MIN_STATES: usize = 2;
/// Binding names that every module legitimately repeats.
pub const IGNORED_NAMES: &[&str] = &["log", "logger", "pytestmark", "__all__"];
/// Most other files named in one finding.
const MAX_OTHERS: usize = 3;

/// A module-level binding: a constant, a static, a top-level assignment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding {
    pub path: String,
    pub line: u32,
    pub name: String,
    /// The initializer text with whitespace collapsed. Nothing is evaluated.
    pub initializer: String,
}

/// A comparison of a subject with a literal: equality, inequality,
/// membership in a literal collection, or a match or switch arm.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Comparison {
    pub path: String,
    pub line: u32,
    /// The compared expression as written, whitespace removed.
    pub subject: String,
    /// The literal as written.
    pub literal: String,
}

/// The locality facts of a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalityFacts {
    pub bindings: Vec<Binding>,
    pub comparisons: Vec<Comparison>,
    /// Files the facts were taken from.
    pub files: usize,
}

fn others_text(here: &str, files: &BTreeSet<&str>) -> String {
    let named: Vec<&str> = files
        .iter()
        .copied()
        .filter(|path| *path != here)
        .take(MAX_OTHERS)
        .collect();
    let more = files.len().saturating_sub(1).saturating_sub(named.len());
    if more > 0 {
        format!("{} and {more} more", named.join(", "))
    } else {
        named.join(", ")
    }
}

/// Where a finding stands: the file and the first line of the repetition.
struct Site<'a> {
    path: &'a str,
    line: u32,
}

fn finding(
    code: &'static str,
    severity: Severity,
    site: &Site<'_>,
    message: String,
    hint: String,
) -> CodeFinding {
    let mut base = Finding::new(code, site.path.to_string(), message).severity(severity);
    base.line = usize::try_from(site.line).ok().filter(|line| *line > 0);
    CodeFinding {
        finding: base,
        tag: Tag::Collect,
        hint,
        metric: None,
        value: None,
        before: None,
        limit: None,
        limit_source: None,
        pre_existing: false,
        trust: None,
        entity: None,
        owners: Attribution::default(),
    }
}

/// File -> first line of one binding or one subject in that file.
type Places<'a> = BTreeMap<&'a str, u32>;

/// (name, initializer) -> the files that bind that name to that value.
fn definition_groups(facts: &LocalityFacts) -> BTreeMap<(&str, &str), Places<'_>> {
    let mut groups: BTreeMap<(&str, &str), Places<'_>> = BTreeMap::new();
    for binding in &facts.bindings {
        if IGNORED_NAMES.contains(&binding.name.as_str()) {
            continue;
        }
        let lines = groups
            .entry((binding.name.as_str(), binding.initializer.as_str()))
            .or_default();
        let line = lines.entry(binding.path.as_str()).or_insert(binding.line);
        *line = (*line).min(binding.line);
    }
    groups
}

/// The findings of one scattered (name, initializer) group: one per
/// selected file.
fn scattered_definition(
    name: &str,
    initializer: &str,
    files: &Places<'_>,
    selected: &dyn Fn(&str) -> bool,
) -> Vec<CodeFinding> {
    let paths: BTreeSet<&str> = files.keys().copied().collect();
    let mut out = Vec::new();
    for (path, line) in files {
        if !selected(path) {
            continue;
        }
        out.push(finding(
            codes::CODE_SCATTERED_DEFINITION,
            Severity::Warning,
            &Site { path, line: *line },
            format!(
                "`{name} = {initializer}` is defined in {} files; also in {}",
                files.len(),
                others_text(path, &paths)
            ),
            format!(
                "define `{name}` once in the module that owns it and import it in the other {} files",
                files.len().saturating_sub(1)
            ),
        ));
    }
    out
}

/// One name, several scattered values: reported once per selected file,
/// never merged.
fn ambiguous_definition(
    name: &str,
    values: &[(&str, &Places<'_>)],
    selected: &dyn Fn(&str) -> bool,
    max_files: usize,
) -> Vec<CodeFinding> {
    let mut out = Vec::new();
    if values.len() < 2 {
        return out;
    }
    let mut reported: BTreeSet<&str> = BTreeSet::new();
    for (_, files) in values {
        for (path, line) in *files {
            if selected(path) && reported.insert(path) {
                out.push(CodeFinding {
                    tag: Tag::Name,
                    ..finding(
                        codes::CODE_DEFINITION_AMBIGUOUS,
                        Severity::Info,
                        &Site { path, line: *line },
                        format!(
                            "`{name}` is defined with {} different values, each in more than {max_files} files; they were not merged",
                            values.len()
                        ),
                        format!(
                            "give each meaning of `{name}` its own name before collecting the definitions"
                        ),
                    )
                });
            }
        }
    }
    out
}

/// Scattered definitions: one finding per selected file that repeats a
/// binding found, with the same initializer, in more than `max_files`
/// files. When one name carries two or more such values, an information
/// finding says that they were not merged.
#[must_use]
pub fn definition_findings(
    facts: &LocalityFacts,
    selected: &dyn Fn(&str) -> bool,
    max_files: usize,
) -> Vec<CodeFinding> {
    let groups = definition_groups(facts);
    let mut out = Vec::new();
    // name -> the scattered (initializer, files) groups of that name.
    let mut scattered_values: BTreeMap<&str, Vec<(&str, &Places<'_>)>> = BTreeMap::new();
    for ((name, initializer), files) in &groups {
        if files.len() <= max_files {
            continue;
        }
        scattered_values
            .entry(name)
            .or_default()
            .push((initializer, files));
        out.extend(scattered_definition(name, initializer, files, selected));
    }
    for (name, values) in scattered_values {
        out.extend(ambiguous_definition(name, &values, selected, max_files));
    }
    out
}

/// The first literals of a subject, with the count of the ones left out.
fn literals_text(literals: &BTreeSet<&str>) -> String {
    let values: Vec<&str> = literals.iter().copied().take(4).collect();
    let more = literals.len().saturating_sub(values.len());
    if more > 0 {
        format!("{} and {more} more", values.join(", "))
    } else {
        values.join(", ")
    }
}

/// Scattered state: one finding per subject per selected file, when the
/// subject is compared with at least `min_states` distinct literals and
/// the comparisons stand in more than `max_files` files.
#[must_use]
pub fn state_findings(
    facts: &LocalityFacts,
    selected: &dyn Fn(&str) -> bool,
    max_files: usize,
    min_states: usize,
) -> Vec<CodeFinding> {
    struct Subject<'a> {
        literals: BTreeSet<&'a str>,
        files: BTreeMap<&'a str, u32>,
    }
    let mut subjects: BTreeMap<&str, Subject<'_>> = BTreeMap::new();
    for comparison in &facts.comparisons {
        let entry = subjects
            .entry(comparison.subject.as_str())
            .or_insert_with(|| Subject {
                literals: BTreeSet::new(),
                files: BTreeMap::new(),
            });
        entry.literals.insert(comparison.literal.as_str());
        let line = entry
            .files
            .entry(comparison.path.as_str())
            .or_insert(comparison.line);
        *line = (*line).min(comparison.line);
    }
    let mut out = Vec::new();
    for (subject, entry) in &subjects {
        if entry.literals.len() < min_states || entry.files.len() <= max_files {
            continue;
        }
        let paths: BTreeSet<&str> = entry.files.keys().copied().collect();
        let listed = literals_text(&entry.literals);
        for (path, line) in &entry.files {
            if !selected(path) {
                continue;
            }
            out.push(finding(
                codes::CODE_SCATTERED_STATE,
                Severity::Info,
                &Site { path, line: *line },
                format!(
                    "`{subject}` is compared with {} literal values ({listed}) in {} files; also in {}",
                    entry.literals.len(),
                    entry.files.len(),
                    others_text(path, &paths)
                ),
                format!(
                    "declare the values of `{subject}` once, as an enumeration or named constants, and put the decisions on them in one place"
                ),
            ));
        }
    }
    out
}
