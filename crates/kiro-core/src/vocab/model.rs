//! Model types for vocabulary grounding (design component `VocabModel`).
//!
//! Owns `Kind`, `Term`, `Topic`, `Hit`, `Vocabulary` value types,
//! label normalisation, id validation, and vocabulary constants.

use std::collections::BTreeMap;

pub const VOCABULARY_PATH: &str = ".kiro/vocabulary.toml";
pub const SCHEMA_VERSION: &str = "2";

/// Kind of a vocabulary term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Entity,
    Bool,
    Enum { values: Vec<String> },
    Int { min: Option<i64>, max: Option<i64> },
}

impl Kind {
    /// Returns the static name of the kind ("entity", "bool", "enum", "int").
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Entity => "entity",
            Kind::Bool => "bool",
            Kind::Enum { .. } => "enum",
            Kind::Int { .. } => "int",
        }
    }
}

/// A validated vocabulary term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub id: String,
    pub topic: String,
    pub label: String,
    pub definition: String,
    pub kind: Kind,
    /// Validated same-topic ids in declaration order (ignored relations removed).
    pub broader: Vec<String>,
    /// Alternative labels as written.
    pub alt_labels: Vec<String>,
    pub deprecated: bool,
    /// Replacement term id as written (validity reported by findings).
    pub replaced_by: Option<String>,
    /// 1-based line of `[terms.<id>]`.
    pub line: usize,
}

/// A declared or implicit vocabulary topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    pub id: String,
    pub label: Option<String>,
    pub description: Option<String>,
    /// 1-based line of `[topics.<id>]`, or `None` if implicit.
    pub line: Option<usize>,
    pub term_count: usize,
}

/// A cross-topic or in-topic hit identifying a term by its topic and id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hit {
    pub topic: String,
    pub id: String,
}

/// An indexed immutable vocabulary value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vocabulary {
    pub(crate) terms: BTreeMap<String, Term>,
    pub(crate) topics: BTreeMap<String, Topic>,
    pub(crate) labels: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub(crate) ancestors: BTreeMap<String, Vec<String>>,
}

impl Vocabulary {
    /// Creates a new `Vocabulary` from its indexed parts.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn new(
        terms: BTreeMap<String, Term>,
        topics: BTreeMap<String, Topic>,
        labels: BTreeMap<String, BTreeMap<String, Vec<String>>>,
        ancestors: BTreeMap<String, Vec<String>>,
    ) -> Self {
        Self {
            terms,
            topics,
            labels,
            ancestors,
        }
    }

    /// Number of terms in the vocabulary.
    #[must_use]
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether the vocabulary contains no terms.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

/// Normalises a label: trims leading/trailing whitespace, collapses internal
/// whitespace to a single ASCII space, and converts to Unicode lower-case.
#[must_use]
pub fn normalize_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Checks whether `s` matches `[a-z0-9][a-z0-9-]*`.
#[must_use]
pub fn is_id(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_label_cases() {
        // Trim and inner whitespace
        assert_eq!(normalize_label("  hello   world  "), "hello world");
        // Tabs and newlines
        assert_eq!(normalize_label("hello\t\nworld"), "hello world");
        // Mixed casing
        assert_eq!(
            normalize_label("Acceptance Criterion"),
            "acceptance criterion"
        );
        // Unicode case conversion
        assert_eq!(normalize_label("  ÜBER   GRÖSSEN  "), "über grössen");
        // Empty string
        assert_eq!(normalize_label("   "), "");
    }

    #[test]
    fn is_id_rule_table() {
        // Valid
        assert!(is_id("a"));
        assert!(is_id("a1"));
        assert!(is_id("sdd-task"));
        assert!(is_id("acceptance-criterion-1"));
        assert!(is_id("0-start"));

        // Invalid: empty
        assert!(!is_id(""));
        // Invalid: uppercase
        assert!(!is_id("Task"));
        assert!(!is_id("sdd-Task"));
        // Invalid: leading hyphen
        assert!(!is_id("-task"));
        // Invalid: underscore
        assert!(!is_id("sdd_task"));
        // Invalid: spaces or dots
        assert!(!is_id("sdd task"));
        assert!(!is_id("sdd.task"));
    }

    #[test]
    fn kind_name_mapping() {
        assert_eq!(Kind::Entity.name(), "entity");
        assert_eq!(Kind::Bool.name(), "bool");
        assert_eq!(
            Kind::Enum {
                values: vec!["a".into()]
            }
            .name(),
            "enum"
        );
        assert_eq!(
            Kind::Int {
                min: None,
                max: None
            }
            .name(),
            "int"
        );
    }

    #[test]
    fn vocabulary_len_and_is_empty() {
        let empty = Vocabulary::default();
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }
}
