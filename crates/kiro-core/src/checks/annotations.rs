//! Annotation reader for requirements documents (design component `AnnotationReader`,
//! requirements 1.6, 3.1, 4.9).
//!
//! Provides a typed view over a parsed requirements document for the four
//! constraint keys (`_Terms:_`, `_Guard:_`, `_Effect:_`, `_Conflicts:_`),
//! preserving placement and identifying misplaced or duplicate annotations.

use std::collections::BTreeSet;

use crate::md::Annotation;
use crate::requirements::{Criterion, Requirement, RequirementsDoc};
use crate::vocab::normalize_label;

pub const KEY_TERMS: &str = "Terms";
pub const KEY_GUARD: &str = "Guard";
pub const KEY_EFFECT: &str = "Effect";
pub const KEY_CONFLICTS: &str = "Conflicts";

/// A declared term name with its annotation line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredName {
    pub name: String,
    pub line: usize,
}

/// A parsed conflicts annotation with target criterion ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictList {
    pub ids: Vec<String>,
    pub line: usize,
}

/// Typed annotation view for a single criterion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedCriterion<'d> {
    pub criterion: &'d Criterion,
    pub requirement_id: String,
    pub terms: Vec<DeclaredName>,
    pub guard: Option<&'d Annotation>,
    pub effect: Option<&'d Annotation>,
    pub conflicts: Option<ConflictList>,
    pub duplicates: Vec<&'d Annotation>,
}

/// Typed annotation view for a single requirement and its criteria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedRequirement<'d> {
    pub requirement: &'d Requirement,
    pub terms: Vec<DeclaredName>,
    pub misplaced: Vec<&'d Annotation>,
    pub criteria: Vec<AnnotatedCriterion<'d>>,
}

/// Complete typed view over all constraint annotations in a requirements document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotated<'d> {
    pub requirements: Vec<AnnotatedRequirement<'d>>,
    pub any: bool,
    pub first_line: Option<usize>,
}

/// Splits a comma-separated annotation value, trimming entries and dropping empty ones.
#[must_use]
pub fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn collect_declared_terms(
    annotations: &[Annotation],
    seen: &mut BTreeSet<String>,
) -> Vec<DeclaredName> {
    let mut names = Vec::new();
    for ann in annotations {
        if ann.key.eq_ignore_ascii_case(KEY_TERMS) {
            for item in split_list(&ann.value) {
                let norm = normalize_label(&item);
                if seen.insert(norm) {
                    names.push(DeclaredName {
                        name: item,
                        line: ann.line,
                    });
                }
            }
        }
    }
    names
}

/// Collects all constraint annotations from a parsed requirements document.
#[must_use]
pub fn collect(doc: &RequirementsDoc) -> Annotated<'_> {
    let mut any = false;
    let mut first_line: Option<usize> = None;

    let update_line = |first: &mut Option<usize>, line: usize| {
        *first = Some(first.map_or(line, |prev| prev.min(line)));
    };

    let mut annotated_requirements = Vec::with_capacity(doc.requirements.len());

    for req in &doc.requirements {
        let mut req_seen_norm = BTreeSet::new();
        let req_terms = collect_declared_terms(&req.annotations, &mut req_seen_norm);

        let mut misplaced = Vec::new();
        for ann in &req.annotations {
            if ann.key.eq_ignore_ascii_case(KEY_TERMS) {
                any = true;
                update_line(&mut first_line, ann.line);
            } else if ann.key.eq_ignore_ascii_case(KEY_GUARD)
                || ann.key.eq_ignore_ascii_case(KEY_EFFECT)
                || ann.key.eq_ignore_ascii_case(KEY_CONFLICTS)
            {
                any = true;
                update_line(&mut first_line, ann.line);
                misplaced.push(ann);
            }
        }

        let mut annotated_criteria = Vec::with_capacity(req.criteria.len());
        for crit in &req.criteria {
            let mut crit_seen_norm = BTreeSet::new();
            let crit_terms = collect_declared_terms(&crit.annotations, &mut crit_seen_norm);

            let mut guard: Option<&Annotation> = None;
            let mut effect: Option<&Annotation> = None;
            let mut conflicts: Option<ConflictList> = None;
            let mut duplicates = Vec::new();

            for ann in &crit.annotations {
                if ann.key.eq_ignore_ascii_case(KEY_TERMS) {
                    any = true;
                    update_line(&mut first_line, ann.line);
                } else if ann.key.eq_ignore_ascii_case(KEY_GUARD) {
                    any = true;
                    update_line(&mut first_line, ann.line);
                    if guard.is_none() {
                        guard = Some(ann);
                    } else {
                        duplicates.push(ann);
                    }
                } else if ann.key.eq_ignore_ascii_case(KEY_EFFECT) {
                    any = true;
                    update_line(&mut first_line, ann.line);
                    if effect.is_none() {
                        effect = Some(ann);
                    } else {
                        duplicates.push(ann);
                    }
                } else if ann.key.eq_ignore_ascii_case(KEY_CONFLICTS) {
                    any = true;
                    update_line(&mut first_line, ann.line);
                    if conflicts.is_none() {
                        conflicts = Some(ConflictList {
                            ids: split_list(&ann.value),
                            line: ann.line,
                        });
                    } else {
                        duplicates.push(ann);
                    }
                }
            }

            annotated_criteria.push(AnnotatedCriterion {
                criterion: crit,
                requirement_id: req.id.clone(),
                terms: crit_terms,
                guard,
                effect,
                conflicts,
                duplicates,
            });
        }

        annotated_requirements.push(AnnotatedRequirement {
            requirement: req,
            terms: req_terms,
            misplaced,
            criteria: annotated_criteria,
        });
    }

    Annotated {
        requirements: annotated_requirements,
        any,
        first_line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requirements::parse;

    #[test]
    fn split_list_behavior() {
        assert_eq!(
            split_list("a, b,  c, "),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(split_list(""), Vec::<String>::new());
        assert_eq!(split_list("   "), Vec::<String>::new());
    }

    #[test]
    fn empty_document_has_no_annotations() {
        let text = "# Title\n\n### Requirement 1: A\n\n#### Acceptance Criteria\n\n1. Criterion.\n";
        let doc = parse(text, "test.md");
        let ann = collect(&doc);
        assert!(!ann.any);
        assert_eq!(ann.first_line, None);
        assert_eq!(ann.requirements.len(), 1);
        let Some(req) = ann.requirements.first() else {
            return;
        };
        assert!(req.terms.is_empty());
        assert!(req.misplaced.is_empty());
        assert_eq!(req.criteria.len(), 1);
        assert!(req.criteria.first().is_some_and(|c| c.guard.is_none()));
    }

    #[test]
    fn requirement_and_criterion_placement() {
        let text = r#"# Title

### Requirement 1: Control
_Terms: mode, depth, Mode_

#### Acceptance Criteria

1. The controller shall start.
   _Terms: armed, Armed_
   _Guard: mode = safe_
   _Effect: armed = true_
   _Conflicts: 1.2_
2. The controller shall stop.
"#;
        let doc = parse(text, "test.md");
        let ann = collect(&doc);
        assert!(ann.any);
        assert_eq!(ann.first_line, Some(4));

        let Some(req) = ann.requirements.first() else {
            return;
        };
        // Deduplicated: mode, depth
        assert_eq!(req.terms.len(), 2);
        assert_eq!(req.terms.first().map(|t| t.name.as_str()), Some("mode"));
        assert_eq!(req.terms.get(1).map(|t| t.name.as_str()), Some("depth"));
        assert!(req.misplaced.is_empty());

        let Some(c1) = req.criteria.first() else {
            return;
        };
        assert_eq!(c1.terms.len(), 1);
        assert_eq!(c1.terms.first().map(|t| t.name.as_str()), Some("armed"));
        assert_eq!(c1.guard.map(|g| g.value.as_str()), Some("mode = safe"));
        assert_eq!(c1.effect.map(|e| e.value.as_str()), Some("armed = true"));
        assert_eq!(
            c1.conflicts,
            Some(ConflictList {
                ids: vec!["1.2".into()],
                line: 12,
            })
        );
        assert!(c1.duplicates.is_empty());

        let Some(c2) = req.criteria.get(1) else {
            return;
        };
        assert!(c2.terms.is_empty());
        assert!(c2.guard.is_none());
        assert!(c2.effect.is_none());
        assert!(c2.conflicts.is_none());
    }

    #[test]
    fn duplicates_and_misplaced_annotations() {
        let text = r#"# Title

### Requirement 1: Faulty
_Guard: mode = safe_

#### Acceptance Criteria

1. The controller shall react.
   _Guard: mode = safe_
   _Guard: mode = normal_
   _Effect: armed = true_
   _Effect: armed = false_
   _Conflicts: 1.2_
   _Conflicts: 1.3_
"#;
        let doc = parse(text, "test.md");
        let ann = collect(&doc);
        assert!(ann.any);
        assert_eq!(ann.first_line, Some(4));

        let Some(req) = ann.requirements.first() else {
            return;
        };
        assert_eq!(req.misplaced.len(), 1);
        assert_eq!(req.misplaced.first().map(|a| a.key.as_str()), Some("Guard"));

        let Some(c) = req.criteria.first() else {
            return;
        };
        assert_eq!(c.guard.map(|g| g.value.as_str()), Some("mode = safe"));
        assert_eq!(c.effect.map(|e| e.value.as_str()), Some("armed = true"));
        assert_eq!(
            c.conflicts,
            Some(ConflictList {
                ids: vec!["1.2".into()],
                line: 13,
            })
        );
        // Duplicate Guard, Effect, Conflicts
        assert_eq!(c.duplicates.len(), 3);
        assert_eq!(c.duplicates.first().map(|a| a.key.as_str()), Some("Guard"));
        assert_eq!(c.duplicates.get(1).map(|a| a.key.as_str()), Some("Effect"));
        assert_eq!(
            c.duplicates.get(2).map(|a| a.key.as_str()),
            Some("Conflicts")
        );
    }
}
