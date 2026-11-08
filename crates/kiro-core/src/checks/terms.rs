//! Term scope resolution, term checks and suggestions (design component `TermCheck`,
//! requirements 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 4.2).
//!
//! Provides resolution of declared terms strictly within declared topics,
//! term reference lookup for guards and effects, findings for deprecated,
//! mismatched, unknown, or ambiguous terms, term suggestions, and the
//! `CHECK_UNSCOPED` finding helper.

use std::collections::{BTreeMap, BTreeSet};

use crate::checks::annotations::Annotated;
use crate::checks::text::{contains_phrase, mask_literals};
use crate::finding::{Finding, Severity, codes};
use crate::vocab::model::{Hit, Kind, Vocabulary, normalize_label};
use crate::vocab::scope::Lookup;

/// The outcome of resolving a declared term within declared topics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Found {
        id: String,
        topic: String,
        kind: Kind,
        deprecated: bool,
        replacement: Option<String>,
    },
    NotInTopic {
        hits: Vec<Hit>,
    },
    Unknown,
    Ambiguous {
        hits: Vec<Hit>,
    },
}

/// A declared term with its original line and resolution outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTerm {
    pub name: String,
    pub line: usize,
    pub resolved: Resolved,
}

/// Scoped declared terms indexed by requirement and criterion id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TermScope {
    requirement: BTreeMap<String, Vec<DeclaredTerm>>,
    criterion: BTreeMap<String, Vec<DeclaredTerm>>,
}

impl TermScope {
    /// Returns the requirement's declared terms followed by the criterion's own.
    #[must_use]
    pub fn declared(&self, requirement_id: &str, criterion_id: &str) -> Vec<&DeclaredTerm> {
        let mut list = Vec::new();
        if let Some(req_terms) = self.requirement.get(requirement_id) {
            list.extend(req_terms.iter());
        }
        if let Some(crit_terms) = self.criterion.get(criterion_id) {
            list.extend(crit_terms.iter());
        }
        list
    }

    /// Matches a term reference against declared terms in `declared` order.
    ///
    /// Matches if `reference` (trimmed, ASCII lowercase) equals a declared term's
    /// resolved id, or equals `normalize_label(name)` with every space replaced by `-`.
    #[must_use]
    pub fn reference(
        &self,
        requirement_id: &str,
        criterion_id: &str,
        reference: &str,
    ) -> Option<&DeclaredTerm> {
        let ref_norm = reference.trim().to_ascii_lowercase();
        for term in self.declared(requirement_id, criterion_id) {
            if let Resolved::Found { id, .. } = &term.resolved
                && id.to_ascii_lowercase() == ref_norm
            {
                return Some(term);
            }
            let label_hyphenated = normalize_label(&term.name).replace(' ', "-");
            if label_hyphenated == ref_norm {
                return Some(term);
            }
        }
        None
    }

    /// Returns the set of all resolved term ids found on a requirement and all its criteria.
    #[must_use]
    pub fn found_ids(&self, requirement_id: &str) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        if let Some(req_terms) = self.requirement.get(requirement_id) {
            for t in req_terms {
                if let Resolved::Found { id, .. } = &t.resolved {
                    ids.insert(id.clone());
                }
            }
        }
        for (crit_id, crit_terms) in &self.criterion {
            if crit_id.starts_with(requirement_id) && crit_id.contains('.') {
                for t in crit_terms {
                    if let Resolved::Found { id, .. } = &t.resolved {
                        ids.insert(id.clone());
                    }
                }
            }
        }
        ids
    }
}

fn convert_lookup(lookup: Lookup<'_>) -> Resolved {
    match lookup {
        Lookup::Found {
            term, replacement, ..
        } => Resolved::Found {
            id: term.id.clone(),
            topic: term.topic.clone(),
            kind: term.kind.clone(),
            deprecated: term.deprecated,
            replacement: replacement
                .map(|r| r.id.clone())
                .or_else(|| term.replaced_by.clone()),
        },
        Lookup::NotInTopic { hits } => Resolved::NotInTopic { hits },
        Lookup::Ambiguous { hits } => Resolved::Ambiguous { hits },
        Lookup::Unknown | Lookup::Unscoped => Resolved::Unknown,
    }
}

/// Resolves every declared term in `annotated` against `vocabulary` strictly inside `topics`.
#[must_use]
pub fn resolve_scope(
    annotated: &Annotated<'_>,
    vocabulary: &Vocabulary,
    topics: &[String],
) -> TermScope {
    let mut scope = TermScope::default();

    for req in &annotated.requirements {
        let mut req_terms = Vec::with_capacity(req.terms.len());
        for decl in &req.terms {
            let lookup = vocabulary.resolve(&decl.name, topics);
            req_terms.push(DeclaredTerm {
                name: decl.name.clone(),
                line: decl.line,
                resolved: convert_lookup(lookup),
            });
        }
        scope
            .requirement
            .insert(req.requirement.id.clone(), req_terms);

        for crit in &req.criteria {
            let mut crit_terms = Vec::with_capacity(crit.terms.len());
            for decl in &crit.terms {
                let lookup = vocabulary.resolve(&decl.name, topics);
                crit_terms.push(DeclaredTerm {
                    name: decl.name.clone(),
                    line: decl.line,
                    resolved: convert_lookup(lookup),
                });
            }
            scope
                .criterion
                .insert(crit.criterion.id.clone(), crit_terms);
        }
    }

    scope
}

/// Structural summary of the term check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TermsSummary {
    pub declared: usize,
    pub resolved: usize,
    pub deprecated: usize,
    pub mismatched: usize,
    pub undeclared: usize,
    pub ambiguous: usize,
    pub suggested: usize,
}

/// Checks declared terms for errors/warnings and emits term suggestions.
#[must_use]
pub fn check(
    annotated: &Annotated<'_>,
    scope: &TermScope,
    vocabulary: &Vocabulary,
    topics: &[String],
    feature: &str,
    file: &str,
    suggest: bool,
) -> (Vec<Finding>, TermsSummary) {
    let mut findings = Vec::new();
    let mut summary = TermsSummary::default();

    let decl_topics_str = topics.join(", ");

    // 1. Process all declared terms across requirements and criteria
    for req in &annotated.requirements {
        let req_declared = scope
            .requirement
            .get(&req.requirement.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);

        for decl in req_declared {
            process_declared_term(
                decl,
                &decl_topics_str,
                file,
                feature,
                &mut findings,
                &mut summary,
            );
        }

        for crit in &req.criteria {
            let crit_declared = scope
                .criterion
                .get(&crit.criterion.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            for decl in crit_declared {
                process_declared_term(
                    decl,
                    &decl_topics_str,
                    file,
                    feature,
                    &mut findings,
                    &mut summary,
                );
            }
        }
    }

    // 2. Term suggestions
    if suggest {
        let visible_terms = vocabulary.visible(topics);
        for req in &annotated.requirements {
            for term in &visible_terms {
                let mut phrases = Vec::with_capacity(1 + term.alt_labels.len());
                phrases.push(normalize_label(&term.label));
                for alt in &term.alt_labels {
                    phrases.push(normalize_label(alt));
                }

                let mut matching_criteria = Vec::new();

                for crit in &req.criteria {
                    // Check if term.id is declared on the requirement or on this criterion
                    let declared_terms = scope.declared(&req.requirement.id, &crit.criterion.id);
                    let is_declared = declared_terms.iter().any(|d| match &d.resolved {
                        Resolved::Found { id, .. } => id == &term.id,
                        _ => false,
                    });

                    if is_declared {
                        continue;
                    }

                    let masked = mask_literals(&crit.criterion.text);
                    let phrase_matches = phrases.iter().any(|p| contains_phrase(&masked, p));
                    if phrase_matches {
                        matching_criteria.push(crit.criterion.id.clone());
                    }
                }

                if !matching_criteria.is_empty() {
                    summary.suggested += 1;
                    findings.push(
                        Finding::new(
                            codes::TERM_SUGGESTED,
                            file,
                            format!(
                                "{} (term {}) occurs in {} but is not declared in _Terms:_",
                                term.label,
                                term.id,
                                matching_criteria.join(", ")
                            ),
                        )
                        .severity(Severity::Info)
                        .at(req.requirement.line)
                        .feature(feature),
                    );
                }
            }
        }
    }

    (findings, summary)
}

fn process_declared_term(
    decl: &DeclaredTerm,
    decl_topics_str: &str,
    file: &str,
    feature: &str,
    findings: &mut Vec<Finding>,
    summary: &mut TermsSummary,
) {
    summary.declared += 1;
    match &decl.resolved {
        Resolved::Found {
            deprecated,
            replacement,
            ..
        } => {
            summary.resolved += 1;
            if *deprecated {
                summary.deprecated += 1;
                let msg = match replacement {
                    Some(r) => format!("{} is deprecated; use {r}", decl.name),
                    None => format!(
                        "{} is deprecated; no replacement in the declared topics",
                        decl.name
                    ),
                };
                findings.push(
                    Finding::new(codes::TERM_DEPRECATED, file, msg)
                        .at(decl.line)
                        .feature(feature),
                );
            }
        }
        Resolved::NotInTopic { hits } => {
            summary.mismatched += 1;
            let hit_topics: BTreeSet<&str> = hits.iter().map(|h| h.topic.as_str()).collect();
            let hit_topics_str = hit_topics.into_iter().collect::<Vec<_>>().join(", ");
            findings.push(
                Finding::new(
                    codes::TERM_TOPIC_MISMATCH,
                    file,
                    format!(
                        "{} exists only in topic(s) {}; spec declares {}; cross-topic matches are never resolved",
                        decl.name, hit_topics_str, decl_topics_str
                    ),
                )
                .at(decl.line)
                .feature(feature),
            );
        }
        Resolved::Unknown => {
            summary.undeclared += 1;
            findings.push(
                Finding::new(
                    codes::TERM_UNDECLARED,
                    file,
                    format!("{} is not a term in topics {}", decl.name, decl_topics_str),
                )
                .at(decl.line)
                .feature(feature),
            );
        }
        Resolved::Ambiguous { hits } => {
            summary.ambiguous += 1;
            let hit_strs: Vec<String> = hits
                .iter()
                .map(|h| format!("{}.{}", h.topic, h.id))
                .collect();
            findings.push(
                Finding::new(
                    codes::TERM_AMBIGUOUS,
                    file,
                    format!(
                        "{} matches {}; use the term id",
                        decl.name,
                        hit_strs.join(", ")
                    ),
                )
                .at(decl.line)
                .feature(feature),
            );
        }
    }
}

/// Helper that creates a `CHECK_UNSCOPED` finding.
#[must_use]
pub fn unscoped_finding(feature: &str, file: &str, line: usize, reason: &str) -> Finding {
    Finding::new(
        codes::CHECK_UNSCOPED,
        file,
        format!("annotations present but not checked: {reason}"),
    )
    .at(line)
    .feature(feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::annotations::collect;
    use crate::requirements::parse;

    fn test_vocab() -> Vocabulary {
        let toml = r#"
schema_version = "2"

[topics.ops]
label = "Operations"

[topics.pm]
label = "Project Management"

[topics.hr]
label = "Human Resources"

[terms.mode]
topic = "ops"
label = "Mode"
definition = "Operating mode."
kind = "enum"
values = ["normal", "safe"]

[terms.legacy-mode]
topic = "ops"
label = "Legacy mode"
definition = "Old mode."
kind = "enum"
values = ["off", "on"]
deprecated = true
replaced_by = "mode"

[terms.level]
topic = "ops"
label = "Level"
definition = "Ops level."
kind = "int"

[terms.pm-level]
topic = "pm"
label = "Level"
definition = "PM level."
kind = "int"

[terms.sprint]
topic = "pm"
label = "Sprint"
definition = "Sprint entity."
kind = "entity"

[terms.shift]
topic = "hr"
label = "Shift"
definition = "Shift entity."
kind = "entity"
"#;
        let loaded = crate::vocab::parse(toml, "vocabulary.toml");
        loaded.vocabulary
    }

    #[test]
    fn resolution_outcomes_and_findings() {
        let vocab = test_vocab();
        let topics = vec!["ops".to_string(), "pm".to_string()];

        let doc_text = r#"# Title

### Requirement 1: Test terms
_Terms: shift, ghost, Level, legacy-mode, mode_

#### Acceptance Criteria

1. The controller shall run.
"#;
        let doc = parse(doc_text, "test.md");
        let ann = collect(&doc);
        let scope = resolve_scope(&ann, &vocab, &topics);

        let (findings, summary) = check(&ann, &scope, &vocab, &topics, "test", "test.md", false);
        assert_eq!(summary.declared, 5);
        assert_eq!(summary.resolved, 2); // mode, legacy-mode
        assert_eq!(summary.deprecated, 1); // legacy-mode
        assert_eq!(summary.mismatched, 1); // shift
        assert_eq!(summary.undeclared, 1); // ghost
        assert_eq!(summary.ambiguous, 1); // Level

        assert_eq!(findings.len(), 4);
        assert!(
            findings
                .iter()
                .any(|f| f.code == codes::TERM_TOPIC_MISMATCH)
        );
        assert!(findings.iter().any(|f| f.code == codes::TERM_UNDECLARED));
        assert!(findings.iter().any(|f| f.code == codes::TERM_AMBIGUOUS));
        assert!(findings.iter().any(|f| f.code == codes::TERM_DEPRECATED));
    }

    #[test]
    fn reference_rule_by_id_and_hyphenated_label() {
        let vocab = test_vocab();
        let topics = vec!["ops".to_string()];

        let doc_text = r#"# Title

### Requirement 1: Test
_Terms: mode, Legacy mode_

#### Acceptance Criteria

1. The controller shall start.
   _Terms: level_
"#;
        let doc = parse(doc_text, "test.md");
        let ann = collect(&doc);
        let scope = resolve_scope(&ann, &vocab, &topics);

        // Match by resolved id
        assert!(scope.reference("1", "1.1", "mode").is_some());
        assert!(scope.reference("1", "1.1", "MODE").is_some());

        // Match by hyphenated label
        assert!(scope.reference("1", "1.1", "legacy-mode").is_some());

        // Match criterion-level declared term
        assert!(scope.reference("1", "1.1", "level").is_some());

        // Undeclared reference
        assert!(scope.reference("1", "1.1", "sprint").is_none());
    }

    #[test]
    fn term_suggestions_bounded_and_excluded_when_declared() {
        let vocab = test_vocab();
        let topics = vec!["ops".to_string(), "pm".to_string()];

        let doc_text = r#"# Title

### Requirement 1: Suggestions
_Terms: mode_

#### Acceptance Criteria

1. The controller shall coordinate with the sprint.
2. The controller shall monitor the sprint.
"#;
        let doc = parse(doc_text, "test.md");
        let ann = collect(&doc);
        let scope = resolve_scope(&ann, &vocab, &topics);

        let (findings, summary) = check(&ann, &scope, &vocab, &topics, "test", "test.md", true);
        assert_eq!(summary.suggested, 1);
        assert_eq!(findings.len(), 1);
        let f = match findings.first() {
            Some(f) => f,
            None => {
                assert_eq!(findings.len(), 1);
                return;
            }
        };
        assert_eq!(f.code, codes::TERM_SUGGESTED);
        assert!(
            f.message
                .contains("Sprint (term sprint) occurs in 1.1, 1.2")
        );
    }

    #[test]
    fn unscoped_finding_helper() {
        let f = unscoped_finding("feat", "spec.json", 12, "spec declares no topics");
        assert_eq!(f.code, codes::CHECK_UNSCOPED);
        assert_eq!(f.line, Some(12));
        assert_eq!(f.feature.as_deref(), Some("feat"));
        assert_eq!(
            f.message,
            "annotations present but not checked: spec declares no topics"
        );
    }
}
