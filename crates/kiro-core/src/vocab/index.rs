//! Semantic validation and indexing for vocabulary files (design component `VocabIndex`).
//!
//! Validates term definitions, kinds, duplicate labels, broader relations,
//! replacement chains, cycle detection, and builds the nearest-first broader closure.

use std::collections::{BTreeMap, HashSet, VecDeque};

use crate::finding::{Finding, codes, sort_findings};
use crate::vocab::model::{Kind, Term, Topic, Vocabulary, normalize_label};
use crate::vocab::parse::{LineIndex, RawFile, RawTerm};

/// Builds an indexed, validated `Vocabulary` and its sorted semantic findings from `RawFile`.
/// Always succeeds.
#[allow(dead_code)]
#[must_use]
pub(crate) fn build(raw: RawFile, text: &str, file: &str) -> (Vocabulary, Vec<Finding>) {
    let clean_text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines = LineIndex::new(clean_text);
    let mut findings = Vec::new();

    // Step 1: schema_version check
    if let Some(sv) = &raw.schema_version
        && sv.get_ref() != "2"
    {
        let line = lines.line_of(sv.span().start);
        findings.push(
            Finding::new(
                codes::VOCAB_SCHEMA_UNSUPPORTED,
                file,
                format!(
                    "schema_version `{}` is not supported; expected `2`",
                    sv.get_ref()
                ),
            )
            .at(line),
        );
    }

    // Step 2: Terms validation and construction
    let mut terms: BTreeMap<String, Term> = BTreeMap::new();
    for (id, spanned_term) in &raw.terms {
        let span = spanned_term.span();
        let line = lines.line_of(span.start);
        let raw_term: &RawTerm = spanned_term.get_ref();

        // Check definition
        if raw_term.definition.trim().is_empty() {
            findings.push(
                Finding::new(
                    codes::TERM_NO_DEFINITION,
                    file,
                    format!("term `{id}` has no definition"),
                )
                .at(line),
            );
        }

        // Check kind
        let kind = match raw_term.kind.as_deref() {
            Some("entity") | None => Kind::Entity,
            Some("bool") => Kind::Bool,
            Some("enum") => {
                if raw_term.values.is_empty() {
                    findings.push(
                        Finding::new(
                            codes::TERM_KIND_INVALID,
                            file,
                            format!("term `{id}` has kind `enum` with empty values list"),
                        )
                        .at(line),
                    );
                    Kind::Entity
                } else {
                    Kind::Enum {
                        values: raw_term.values.clone(),
                    }
                }
            }
            Some("int") => {
                if let (Some(min), Some(max)) = (raw_term.min, raw_term.max) {
                    if min > max {
                        findings.push(
                            Finding::new(
                                codes::TERM_KIND_INVALID,
                                file,
                                format!(
                                    "term `{id}` has kind `int` with min ({min}) > max ({max})"
                                ),
                            )
                            .at(line),
                        );
                        Kind::Entity
                    } else {
                        Kind::Int {
                            min: Some(min),
                            max: Some(max),
                        }
                    }
                } else {
                    Kind::Int {
                        min: raw_term.min,
                        max: raw_term.max,
                    }
                }
            }
            Some(unknown) => {
                findings.push(
                    Finding::new(
                        codes::TERM_KIND_INVALID,
                        file,
                        format!("term `{id}` has invalid kind `{unknown}`"),
                    )
                    .at(line),
                );
                Kind::Entity
            }
        };

        terms.insert(
            id.clone(),
            Term {
                id: id.clone(),
                topic: raw_term.topic.clone(),
                label: raw_term.label.clone(),
                definition: raw_term.definition.clone(),
                kind,
                broader: raw_term.broader.clone(),
                alt_labels: raw_term.alt_labels.clone(),
                deprecated: raw_term.deprecated,
                replaced_by: raw_term.replaced_by.clone(),
                line,
            },
        );
    }

    // Step 3: Topics validation and construction
    let mut topics: BTreeMap<String, Topic> = BTreeMap::new();
    for (id, spanned_topic) in &raw.topics {
        let span = spanned_topic.span();
        let line = lines.line_of(span.start);
        let raw_topic = spanned_topic.get_ref();
        topics.insert(
            id.clone(),
            Topic {
                id: id.clone(),
                label: raw_topic.label.clone(),
                description: raw_topic.description.clone(),
                line: Some(line),
                term_count: 0,
            },
        );
    }

    // Add implicit topics from terms
    for term in terms.values() {
        topics.entry(term.topic.clone()).or_insert_with(|| Topic {
            id: term.topic.clone(),
            label: None,
            description: None,
            line: None,
            term_count: 0,
        });
    }

    // Count terms per topic and report TOPIC_EMPTY for declared topics with count == 0
    for term in terms.values() {
        if let Some(topic) = topics.get_mut(&term.topic) {
            topic.term_count += 1;
        }
    }
    for topic in topics.values() {
        if topic.line.is_some() && topic.term_count == 0 {
            findings.push(
                Finding::new(
                    codes::TOPIC_EMPTY,
                    file,
                    format!("topic `{}` has no terms", topic.id),
                )
                .at(topic.line.unwrap_or(1)),
            );
        }
    }

    // Step 4: Labels indexing and duplicate / cross-topic detection
    // Visit terms in ascending file line order
    let mut term_list: Vec<&Term> = terms.values().collect();
    term_list.sort_by_key(|t| (t.line, &t.id));

    let mut labels: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    // Global tracking: norm_label -> Vec<(topic, id, line, as_written)>
    let mut global_labels: BTreeMap<String, Vec<(String, String, usize, String)>> = BTreeMap::new();

    for term in term_list {
        let norm_main = normalize_label(&term.label);
        let mut labels_to_register = vec![(norm_main.clone(), term.label.clone())];

        let mut seen_alts = HashSet::new();
        seen_alts.insert(norm_main.clone());

        for alt in &term.alt_labels {
            let norm_alt = normalize_label(alt);
            if seen_alts.insert(norm_alt.clone()) {
                labels_to_register.push((norm_alt, alt.clone()));
            }
        }

        let topic_labels = labels.entry(term.topic.clone()).or_default();

        for (norm, as_written) in labels_to_register {
            // Check same-topic duplicate label
            if let Some(existing_ids) = topic_labels.get_mut(&norm) {
                for prev_id in existing_ids.iter() {
                    findings.push(
                        Finding::new(
                            codes::TERM_DUPLICATE_LABEL,
                            file,
                            format!(
                                "term `{}` shares label \"{}\" with `{}` in topic `{}`",
                                term.id, as_written, prev_id, term.topic
                            ),
                        )
                        .at(term.line),
                    );
                }
                existing_ids.push(term.id.clone());
            } else {
                topic_labels.insert(norm.clone(), vec![term.id.clone()]);
            }

            // Check cross-topic collision lint (VOCAB_LABEL_CROSS_TOPIC, info)
            let prev_globals = global_labels.entry(norm.clone()).or_default();
            for (prev_topic, prev_id, _, _) in prev_globals.iter() {
                if prev_topic != &term.topic {
                    findings.push(
                        Finding::new(
                            codes::VOCAB_LABEL_CROSS_TOPIC,
                            file,
                            format!(
                                "label \"{}\" exists in topic `{}` as `{}` and topic `{}` as `{}`",
                                as_written, prev_topic, prev_id, term.topic, term.id
                            ),
                        )
                        .at(term.line),
                    );
                }
            }
            prev_globals.push((term.topic.clone(), term.id.clone(), term.line, as_written));
        }
    }

    // Step 5: Broader validation
    let term_meta: Vec<(String, String, usize, Vec<String>)> = terms
        .values()
        .map(|t| (t.id.clone(), t.topic.clone(), t.line, t.broader.clone()))
        .collect();

    for (id, topic, line, broader) in term_meta {
        let mut kept = Vec::new();
        for broader_id in broader {
            if broader_id == id {
                kept.push(broader_id);
            } else if let Some(target) = terms.get(&broader_id) {
                if target.topic != topic {
                    findings.push(
                        Finding::new(
                            codes::TERM_BROADER_CROSS_TOPIC,
                            file,
                            format!(
                                "term `{id}` references cross-topic broader term `{broader_id}` in topic `{}`",
                                target.topic
                            ),
                        )
                        .at(line),
                    );
                } else {
                    kept.push(broader_id);
                }
            } else {
                findings.push(
                    Finding::new(
                        codes::TERM_BROADER_UNKNOWN,
                        file,
                        format!("term `{id}` references unknown broader term `{broader_id}`"),
                    )
                    .at(line),
                );
            }
        }
        if let Some(t) = terms.get_mut(&id) {
            t.broader = kept;
        }
    }

    // Step 6: Closure and cycles in one walk
    let mut ancestors_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (start_id, start_term) in &terms {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut ancestors = Vec::new();
        let mut cycle = false;

        for b in &start_term.broader {
            if b == start_id {
                cycle = true;
            } else if visited.insert(b.clone()) {
                ancestors.push(b.clone());
                queue.push_back(b.clone());
            }
        }

        while let Some(current_id) = queue.pop_front() {
            if let Some(current_term) = terms.get(&current_id) {
                for b in &current_term.broader {
                    if b == start_id {
                        cycle = true;
                    } else if visited.insert(b.clone()) {
                        ancestors.push(b.clone());
                        queue.push_back(b.clone());
                    }
                }
            }
        }

        if cycle {
            findings.push(
                Finding::new(
                    codes::TERM_BROADER_CYCLE,
                    file,
                    format!("term `{start_id}` is transitively broader than itself"),
                )
                .at(start_term.line),
            );
        }

        ancestors_map.insert(start_id.clone(), ancestors);
    }

    // Step 7: Replacement validation
    for term in terms.values() {
        if term.deprecated && term.replaced_by.is_none() {
            findings.push(
                Finding::new(
                    codes::TERM_DEPRECATED_NO_REPLACEMENT,
                    file,
                    format!("term `{}` is deprecated without replacement", term.id),
                )
                .at(term.line),
            );
        }

        if let Some(rep_id) = &term.replaced_by {
            if let Some(target) = terms.get(rep_id) {
                if target.topic != term.topic {
                    findings.push(
                        Finding::new(
                            codes::TERM_REPLACED_BY_CROSS_TOPIC,
                            file,
                            format!(
                                "term `{}` has cross-topic replacement `{}` in topic `{}`",
                                term.id, rep_id, target.topic
                            ),
                        )
                        .at(term.line),
                    );
                }
            } else {
                findings.push(
                    Finding::new(
                        codes::TERM_REPLACED_BY_UNKNOWN,
                        file,
                        format!("term `{}` has unknown replacement `{}`", term.id, rep_id),
                    )
                    .at(term.line),
                );
            }
        }
    }

    sort_findings(&mut findings);

    (
        Vocabulary::new(terms, topics, labels, ancestors_map),
        findings,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::vocab::parse::parse_raw;

    #[test]
    fn single_topic_valid_raw_file_builds_with_no_finding() {
        let content = "schema_version = \"2\"\n\n[topics.sdd]\nlabel = \"SDD\"\n\n[terms.task]\ntopic = \"sdd\"\nlabel = \"Task\"\ndefinition = \"A task.\"\nkind = \"entity\"\n";
        let raw = parse_raw(content, ".kiro/vocabulary.toml").unwrap();
        let (vocab, findings) = build(raw, content, ".kiro/vocabulary.toml");
        assert_eq!(findings, vec![]);
        assert_eq!(vocab.len(), 1);
    }

    #[test]
    fn semantic_findings_exercised_from_fixture() {
        let content = include_str!("../../fixtures/vocab/semantic.toml");
        let raw = parse_raw(content, ".kiro/vocabulary.toml").unwrap();
        let (_vocab, findings) = build(raw, content, ".kiro/vocabulary.toml");

        let codes_produced: Vec<&'static str> = findings.iter().map(|f| f.code).collect();
        assert_eq!(
            codes_produced,
            vec![
                codes::VOCAB_SCHEMA_UNSUPPORTED,
                codes::TOPIC_EMPTY,
                codes::TERM_NO_DEFINITION,
                codes::TERM_KIND_INVALID,
                codes::TERM_DUPLICATE_LABEL,
                codes::TERM_BROADER_UNKNOWN,
                codes::TERM_BROADER_CROSS_TOPIC,
                codes::TERM_DEPRECATED_NO_REPLACEMENT,
                codes::TERM_REPLACED_BY_UNKNOWN,
                codes::TERM_REPLACED_BY_CROSS_TOPIC,
            ]
        );
    }

    #[test]
    fn cross_topic_label_collision_lint_on_valid_fixture() {
        let content = include_str!("../../fixtures/vocab/valid.toml");
        let raw = parse_raw(content, ".kiro/vocabulary.toml").unwrap();
        let (_vocab, findings) = build(raw, content, ".kiro/vocabulary.toml");

        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.code, codes::VOCAB_LABEL_CROSS_TOPIC);
        assert_eq!(f.line, Some(65));
        assert!(f.message.contains(
            "label \"Task\" exists in topic `sdd` as `task` and topic `pm` as `pm-task`"
        ));
    }

    #[test]
    fn diamond_hierarchy_and_depth_three_closure_order() {
        // top -> left, right -> bottom
        let content = r#"
schema_version = "2"
[terms.top]
topic = "sdd"
label = "Top"
definition = "Top"

[terms.left]
topic = "sdd"
label = "Left"
definition = "Left"
broader = ["top"]

[terms.right]
topic = "sdd"
label = "Right"
definition = "Right"
broader = ["top"]

[terms.bottom]
topic = "sdd"
label = "Bottom"
definition = "Bottom"
broader = ["left", "right"]
"#;
        let raw = parse_raw(content, ".kiro/vocabulary.toml").unwrap();
        let (vocab, findings) = build(raw, content, ".kiro/vocabulary.toml");
        assert!(findings.is_empty());
        let ancestors = vocab.ancestors.get("bottom").unwrap();
        // Nearest first: left, right, top
        assert_eq!(ancestors, &vec!["left", "right", "top"]);
    }

    #[test]
    fn cycle_detection_and_pointer_into_cycle() {
        let content = include_str!("../../fixtures/vocab/cycle.toml");
        let raw = parse_raw(content, ".kiro/vocabulary.toml").unwrap();
        let (vocab, findings) = build(raw, content, ".kiro/vocabulary.toml");

        assert_eq!(findings.len(), 4);
        for f in &findings {
            assert_eq!(f.code, codes::TERM_BROADER_CYCLE);
        }
        let cycle_terms: Vec<&str> = findings
            .iter()
            .map(|f| {
                if f.message.contains("`a`") {
                    "a"
                } else if f.message.contains("`b`") {
                    "b"
                } else if f.message.contains("`c`") {
                    "c"
                } else {
                    "d"
                }
            })
            .collect();
        assert_eq!(cycle_terms, vec!["a", "b", "c", "d"]);

        // e points into cycle (e -> a -> b -> c) without cycle finding
        let e_ancestors = vocab.ancestors.get("e").unwrap();
        assert_eq!(e_ancestors, &vec!["a", "b", "c"]);
    }

    #[test]
    fn ten_thousand_terms_depth_ten_builds_cleanly() {
        let mut content = String::from("schema_version = \"2\"\n\n");
        // Chain of depth 10: c0 -> c1 -> ... -> c9
        for i in 0..10 {
            content.push_str(&format!(
                "[terms.c{i}]\ntopic = \"sdd\"\nlabel = \"Chain {i}\"\ndefinition = \"def\"\n"
            ));
            if i > 0 {
                content.push_str(&format!("broader = [\"c{}\"]\n", i - 1));
            }
        }
        // 9,990 additional terms pointing to c9
        for i in 10..10_000 {
            content.push_str(&format!(
                "[terms.t{i}]\ntopic = \"sdd\"\nlabel = \"Term {i}\"\ndefinition = \"def\"\nbroader = [\"c9\"]\n"
            ));
        }

        let raw = parse_raw(&content, ".kiro/vocabulary.toml").unwrap();
        let (vocab, findings) = build(raw, &content, ".kiro/vocabulary.toml");
        assert!(findings.is_empty());
        assert_eq!(vocab.len(), 10_000);
        let t9999_anc = vocab.ancestors.get("t9999").unwrap();
        assert_eq!(t9999_anc.len(), 10);
    }
}
