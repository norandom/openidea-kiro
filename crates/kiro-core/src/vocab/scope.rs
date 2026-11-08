//! Scoped vocabulary queries and topic derivation (design component `VocabScope`).
//!
//! Provides topic-scoped resolution (`resolve`, `preferred_label`, `visible`, `topics`),
//! spec topic derivation (`spec_topics`), and scope findings (`scope_findings`).

use std::collections::HashSet;

use crate::finding::{Finding, codes};
use crate::spec::SpecMeta;
use crate::vocab::model::{Hit, Term, Topic, Vocabulary, normalize_label};
use crate::vocab::{Loaded, Status};

/// The mechanism through which a query matched a term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchVia {
    Id,
    Label,
    AltLabel(String),
}

/// The result of resolving a query within a list of topics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup<'v> {
    Found {
        term: &'v Term,
        via: MatchVia,
        broader: Vec<&'v Term>,
        replacement: Option<&'v Term>,
    },
    NotInTopic {
        hits: Vec<Hit>,
    },
    Ambiguous {
        hits: Vec<Hit>,
    },
    Unknown,
    Unscoped,
}

impl Vocabulary {
    /// Resolves `query` within `topics`. Compares `normalize_label(query)` against ids,
    /// normalized labels and normalized alt labels of terms in `topics` (deduplicated, order-insensitive).
    #[must_use]
    pub fn resolve(&self, query: &str, topics: &[String]) -> Lookup<'_> {
        self.resolve_internal(query, topics, true)
    }

    /// Resolves `id` within `topics` with label matching disabled: matches only via `Id`.
    #[must_use]
    pub fn preferred_label(&self, id: &str, topics: &[String]) -> Lookup<'_> {
        self.resolve_internal(id, topics, false)
    }

    fn resolve_internal(&self, query: &str, topics: &[String], match_labels: bool) -> Lookup<'_> {
        if topics.is_empty() {
            return Lookup::Unscoped;
        }

        let q = if match_labels {
            normalize_label(query)
        } else {
            query.trim().to_string()
        };

        if q.is_empty() {
            return Lookup::Unknown;
        }

        let search_set: HashSet<&str> = topics.iter().map(String::as_str).collect();

        // 1. Search within declared topics
        let mut in_scope_matches: Vec<(&Term, MatchVia)> = Vec::new();
        for term in self.terms.values() {
            if !search_set.contains(term.topic.as_str()) {
                continue;
            }

            if term.id == q {
                in_scope_matches.push((term, MatchVia::Id));
            } else if match_labels {
                if normalize_label(&term.label) == q {
                    in_scope_matches.push((term, MatchVia::Label));
                } else if let Some(alt) = term.alt_labels.iter().find(|a| normalize_label(a) == q) {
                    in_scope_matches.push((term, MatchVia::AltLabel(alt.clone())));
                }
            }
        }

        // Sort matches deterministically by (topic, id)
        in_scope_matches.sort_by_key(|(t, _)| (&t.topic, &t.id));

        if in_scope_matches.len() == 1 {
            let (term, via) = in_scope_matches.remove(0);

            // Compute broader closure
            let broader: Vec<&Term> = self.ancestors.get(&term.id).map_or_else(Vec::new, |ids| {
                ids.iter().filter_map(|id| self.terms.get(id)).collect()
            });

            // Compute in-scope replacement if deprecated
            let replacement = if term.deprecated {
                term.replaced_by.as_ref().and_then(|rep_id| {
                    self.terms
                        .get(rep_id)
                        .filter(|rep_term| search_set.contains(rep_term.topic.as_str()))
                })
            } else {
                None
            };

            return Lookup::Found {
                term,
                via,
                broader,
                replacement,
            };
        }

        if in_scope_matches.len() > 1 {
            let hits = in_scope_matches
                .into_iter()
                .map(|(t, _)| Hit {
                    topic: t.topic.clone(),
                    id: t.id.clone(),
                })
                .collect();
            return Lookup::Ambiguous { hits };
        }

        // 2. Not found in scope: search other topics
        let mut out_of_scope_hits: Vec<Hit> = Vec::new();
        for term in self.terms.values() {
            if search_set.contains(term.topic.as_str()) {
                continue;
            }

            let matches = term.id == q
                || (match_labels
                    && (normalize_label(&term.label) == q
                        || term.alt_labels.iter().any(|a| normalize_label(a) == q)));

            if matches {
                out_of_scope_hits.push(Hit {
                    topic: term.topic.clone(),
                    id: term.id.clone(),
                });
            }
        }

        out_of_scope_hits.sort();
        out_of_scope_hits.dedup();

        if !out_of_scope_hits.is_empty() {
            Lookup::NotInTopic {
                hits: out_of_scope_hits,
            }
        } else {
            Lookup::Unknown
        }
    }

    /// Terms whose topic is in `topics`, ordered by (topic, id).
    #[must_use]
    pub fn visible(&self, topics: &[String]) -> Vec<&Term> {
        let search_set: HashSet<&str> = topics.iter().map(String::as_str).collect();
        let mut result: Vec<&Term> = self
            .terms
            .values()
            .filter(|t| search_set.contains(t.topic.as_str()))
            .collect();
        result.sort_by_key(|t| (&t.topic, &t.id));
        result
    }

    /// Declared or implicit topics ordered by id.
    #[must_use]
    pub fn topics(&self) -> Vec<&Topic> {
        self.topics.values().collect()
    }

    /// Finds a topic by id.
    #[must_use]
    pub fn topic(&self, id: &str) -> Option<&Topic> {
        self.topics.get(id)
    }
}

/// Derives a spec's declared topics from `SpecMeta`: trimmed, non-empty, deduplicated, declaration order.
#[must_use]
pub fn spec_topics(meta: Option<&SpecMeta>) -> Vec<String> {
    let Some(meta) = meta else {
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for t in &meta.topics {
        let trimmed = t.trim();
        if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
            result.push(trimmed.to_string());
        }
    }
    result
}

/// Generates scope findings: `TOPICS_UNDECLARED` (info) when `topics` is empty;
/// `TOPIC_UNKNOWN` (warning) per declared topic absent from the vocabulary when `loaded.status == Status::Loaded`.
#[must_use]
pub fn scope_findings(
    spec: &str,
    spec_json_path: &str,
    topics: &[String],
    loaded: &Loaded,
) -> Vec<Finding> {
    if topics.is_empty() {
        return vec![
            Finding::new(
                codes::TOPICS_UNDECLARED,
                spec_json_path,
                format!("spec `{spec}` declares no topics: add `topics = [...]` to spec.json"),
            )
            .feature(spec),
        ];
    }

    let mut findings = Vec::new();
    if loaded.status == Status::Loaded {
        for t in topics {
            if loaded.vocabulary.topic(t).is_none() {
                findings.push(
                    Finding::new(
                        codes::TOPIC_UNKNOWN,
                        spec_json_path,
                        format!("spec `{spec}` declares unknown topic `{t}`"),
                    )
                    .feature(spec),
                );
            }
        }
    }
    findings
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use crate::vocab::index::build;
    use crate::vocab::parse::parse_raw;

    fn fixture_vocab() -> Vocabulary {
        let content = include_str!("../../fixtures/vocab/valid.toml");
        let raw = parse_raw(content, ".kiro/vocabulary.toml").unwrap();
        let (vocab, _) = build(raw, content, ".kiro/vocabulary.toml");
        vocab
    }

    #[test]
    fn resolve_unscoped_on_empty_topics() {
        let vocab = fixture_vocab();
        let res = vocab.resolve("task", &[]);
        assert_eq!(res, Lookup::Unscoped);
    }

    #[test]
    fn resolve_found_via_id_label_alt_label() {
        let vocab = fixture_vocab();
        let sdd = vec!["sdd".to_string()];

        // Match via Id (whitespace padded)
        let res = vocab.resolve("  task  ", &sdd);
        match res {
            Lookup::Found {
                term, via, broader, ..
            } => {
                assert_eq!(term.id, "task");
                assert_eq!(via, MatchVia::Id);
                assert_eq!(broader.len(), 1);
                assert_eq!(broader[0].id, "spec");
            }
            other => panic!("expected Found, got {other:?}"),
        }

        // Match via Label
        let res = vocab.resolve("Acceptance criterion", &sdd);
        match res {
            Lookup::Found {
                term, via, broader, ..
            } => {
                assert_eq!(term.id, "acceptance-criterion");
                assert_eq!(via, MatchVia::Label);
                // Closure depth 2: requirement, spec
                assert_eq!(broader.len(), 2);
                assert_eq!(broader[0].id, "requirement");
                assert_eq!(broader[1].id, "spec");
            }
            other => panic!("expected Found, got {other:?}"),
        }

        // Match via AltLabel
        let res = vocab.resolve("AC", &sdd);
        match res {
            Lookup::Found { term, via, .. } => {
                assert_eq!(term.id, "acceptance-criterion");
                assert_eq!(via, MatchVia::AltLabel("AC".into()));
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn resolve_deprecated_in_scope_and_out_of_scope_replacement() {
        let vocab = fixture_vocab();
        let sdd = vec!["sdd".to_string()];

        // In scope replacement (old-task -> task in sdd)
        match vocab.resolve("old-task", &sdd) {
            Lookup::Found {
                term, replacement, ..
            } => {
                assert!(term.deprecated);
                assert_eq!(replacement.unwrap().id, "task");
            }
            other => panic!("expected Found, got {other:?}"),
        }

        // Deprecated with out-of-scope replacement
        // If searched topics was only pm, old-task is not in topic
        let pm = vec!["pm".to_string()];
        match vocab.resolve("old-task", &pm) {
            Lookup::NotInTopic { hits } => {
                assert_eq!(
                    hits,
                    vec![Hit {
                        topic: "sdd".into(),
                        id: "old-task".into()
                    }]
                );
            }
            other => panic!("expected NotInTopic, got {other:?}"),
        }
    }

    #[test]
    fn resolve_ambiguous_across_declared_topics() {
        let vocab = fixture_vocab();
        let both = vec!["sdd".to_string(), "pm".to_string()];

        // "Task" is in both sdd (task) and pm (pm-task)
        match vocab.resolve("Task", &both) {
            Lookup::Ambiguous { hits } => {
                assert_eq!(hits.len(), 2);
                assert_eq!(
                    hits[0],
                    Hit {
                        topic: "pm".into(),
                        id: "pm-task".into()
                    }
                );
                assert_eq!(
                    hits[1],
                    Hit {
                        topic: "sdd".into(),
                        id: "task".into()
                    }
                );
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn resolve_not_in_topic_and_order_insensitivity() {
        let vocab = fixture_vocab();
        let sdd = vec!["sdd".to_string()];

        // "sprint" is in pm, not sdd
        match vocab.resolve("sprint", &sdd) {
            Lookup::NotInTopic { hits } => {
                assert_eq!(
                    hits,
                    vec![Hit {
                        topic: "pm".into(),
                        id: "sprint".into()
                    }]
                );
            }
            other => panic!("expected NotInTopic, got {other:?}"),
        }

        // Topic order insensitivity
        let res1 = vocab.resolve("Task", &["sdd".into(), "pm".into()]);
        let res2 = vocab.resolve("Task", &["pm".into(), "sdd".into()]);
        assert_eq!(res1, res2);
    }

    #[test]
    fn resolve_unknown() {
        let vocab = fixture_vocab();
        let sdd = vec!["sdd".to_string()];
        assert_eq!(vocab.resolve("nonexistent-term", &sdd), Lookup::Unknown);
    }

    #[test]
    fn preferred_label_matches_only_id() {
        let vocab = fixture_vocab();
        let sdd = vec!["sdd".to_string()];

        // Matches by id
        match vocab.preferred_label("task", &sdd) {
            Lookup::Found { term, via, .. } => {
                assert_eq!(term.id, "task");
                assert_eq!(via, MatchVia::Id);
            }
            other => panic!("expected Found, got {other:?}"),
        }

        // Does NOT match by label or alt label
        assert_eq!(vocab.preferred_label("Task", &sdd), Lookup::Unknown);
        assert_eq!(vocab.preferred_label("AC", &sdd), Lookup::Unknown);
    }

    #[test]
    fn visible_terms_and_topics() {
        let vocab = fixture_vocab();
        let pm = vec!["pm".to_string()];
        let visible = vocab.visible(&pm);
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[0].id, "milestone");
        assert_eq!(visible[1].id, "pm-task");
        assert_eq!(visible[2].id, "sprint");

        let all_topics = vocab.topics();
        assert_eq!(all_topics.len(), 2);
        assert_eq!(all_topics[0].id, "pm");
        assert_eq!(all_topics[1].id, "sdd");
    }

    #[test]
    fn spec_topics_derivation() {
        assert_eq!(spec_topics(None), Vec::<String>::new());

        let meta_no_topics = SpecMeta::default();
        assert_eq!(spec_topics(Some(&meta_no_topics)), Vec::<String>::new());

        let meta = SpecMeta {
            topics: vec![" sdd ".into(), "".into(), "pm".into(), "sdd".into()],
            ..Default::default()
        };
        assert_eq!(spec_topics(Some(&meta)), vec!["sdd", "pm"]);
    }

    #[test]
    fn scope_findings_cases() {
        let loaded = Loaded {
            status: Status::Loaded,
            vocabulary: fixture_vocab(),
            findings: Vec::new(),
        };

        // Undeclared topics
        let f = scope_findings("demo", ".kiro/specs/demo/spec.json", &[], &loaded);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, codes::TOPICS_UNDECLARED);
        assert_eq!(f[0].file, ".kiro/specs/demo/spec.json");
        assert_eq!(f[0].feature.as_deref(), Some("demo"));

        // Valid topics
        let f = scope_findings(
            "demo",
            ".kiro/specs/demo/spec.json",
            &["sdd".into()],
            &loaded,
        );
        assert!(f.is_empty());

        // Unknown topic with Loaded status
        let f = scope_findings(
            "demo",
            ".kiro/specs/demo/spec.json",
            &["sdd".into(), "ghost".into()],
            &loaded,
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, codes::TOPIC_UNKNOWN);
        assert!(f[0].message.contains("ghost"));

        // Unknown topic with Missing status: suppressed
        let missing = Loaded {
            status: Status::Missing,
            vocabulary: Vocabulary::default(),
            findings: Vec::new(),
        };
        let f = scope_findings(
            "demo",
            ".kiro/specs/demo/spec.json",
            &["ghost".into()],
            &missing,
        );
        assert!(f.is_empty());
    }
}
