//! Engine integration tests over the vocabulary fixtures and the seed file (task 2.8).
//!
//! Validates fixture statuses, line numbers, scoped query outcomes,
//! hierarchy closure, cycle detection, seed file completeness, determinism,
//! release performance, and parser totality over pseudo-random garbage.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::cloned_ref_to_slice_refs
)]

use kiro_core::finding::codes;
use kiro_core::source::{MemorySource, Workspace};
use kiro_core::vocab::{
    Hit, Loaded, Lookup, MatchVia, Status, VOCABULARY_PATH, load, parse, scope_findings,
    spec_topics,
};

const VALID_TOML: &str = include_str!("../fixtures/vocab/valid.toml");
const INVALID_SYNTAX_TOML: &str = include_str!("../fixtures/vocab/invalid-syntax.toml");
const INVALID_STRUCTURE_TOML: &str = include_str!("../fixtures/vocab/invalid-structure.toml");
const SEMANTIC_TOML: &str = include_str!("../fixtures/vocab/semantic.toml");
const CYCLE_TOML: &str = include_str!("../fixtures/vocab/cycle.toml");
const SEED_TOML: &str = include_str!("../../../.kiro/vocabulary.toml");

#[test]
fn fixture_valid_toml_status_and_findings() {
    let loaded = parse(VALID_TOML, VOCABULARY_PATH);
    assert_eq!(loaded.status, Status::Loaded);

    // Exactly one info finding (VOCAB_LABEL_CROSS_TOPIC), zero errors, zero warnings
    assert_eq!(loaded.findings.len(), 1);
    let f = &loaded.findings[0];
    assert_eq!(f.code, codes::VOCAB_LABEL_CROSS_TOPIC);
    assert_eq!(f.line, Some(65));
    assert!(
        f.message
            .contains("label \"Task\" exists in topic `sdd` as `task` and topic `pm` as `pm-task`")
    );
}

#[test]
fn fixture_invalid_syntax_toml_line_and_status() {
    let loaded = parse(INVALID_SYNTAX_TOML, VOCABULARY_PATH);
    assert_eq!(loaded.status, Status::Invalid);
    assert!(loaded.vocabulary.is_empty());
    assert_eq!(loaded.findings.len(), 1);
    let f = &loaded.findings[0];
    assert_eq!(f.code, codes::VOCAB_INVALID);
    assert_eq!(f.line, Some(4));
}

#[test]
fn fixture_invalid_structure_toml_line_and_status() {
    let loaded = parse(INVALID_STRUCTURE_TOML, VOCABULARY_PATH);
    assert_eq!(loaded.status, Status::Invalid);
    assert!(loaded.vocabulary.is_empty());
    assert_eq!(loaded.findings.len(), 1);
    let f = &loaded.findings[0];
    assert_eq!(f.code, codes::VOCAB_INVALID);
    assert_eq!(f.line, Some(11));
}

#[test]
fn fixture_semantic_toml_exercises_every_code_at_expected_lines() {
    let loaded = parse(SEMANTIC_TOML, VOCABULARY_PATH);
    assert_eq!(loaded.status, Status::Loaded);

    let summary: Vec<(&'static str, Option<usize>)> =
        loaded.findings.iter().map(|f| (f.code, f.line)).collect();

    assert_eq!(
        summary,
        vec![
            (codes::VOCAB_SCHEMA_UNSUPPORTED, Some(13)),
            (codes::TOPIC_EMPTY, Some(15)),
            (codes::TERM_NO_DEFINITION, Some(24)),
            (codes::TERM_KIND_INVALID, Some(29)),
            (codes::TERM_DUPLICATE_LABEL, Some(40)),
            (codes::TERM_BROADER_UNKNOWN, Some(45)),
            (codes::TERM_BROADER_CROSS_TOPIC, Some(51)),
            (codes::TERM_DEPRECATED_NO_REPLACEMENT, Some(57)),
            (codes::TERM_REPLACED_BY_UNKNOWN, Some(63)),
            (codes::TERM_REPLACED_BY_CROSS_TOPIC, Some(70)),
        ]
    );
}

#[test]
fn fixture_cycle_toml_reports_cycle_and_closure() {
    let loaded = parse(CYCLE_TOML, VOCABULARY_PATH);
    assert_eq!(loaded.status, Status::Loaded);

    let cycle_lines: Vec<Option<usize>> = loaded
        .findings
        .iter()
        .filter(|f| f.code == codes::TERM_BROADER_CYCLE)
        .map(|f| f.line)
        .collect();

    // a at line 9, b at line 15, c at line 21, d at line 27
    assert_eq!(cycle_lines, vec![Some(9), Some(15), Some(21), Some(27)]);

    // e points into cycle without itself being in a cycle
    match loaded.vocabulary.resolve("e", &["sdd".into()]) {
        Lookup::Found { term, broader, .. } => {
            assert_eq!(term.id, "e");
            let broader_ids: Vec<&str> = broader.iter().map(|t| t.id.as_str()).collect();
            assert_eq!(broader_ids, vec!["a", "b", "c"]);
        }
        other => panic!("expected Found for e, got {other:?}"),
    }
}

fn fixture_tree_workspace() -> Workspace<MemorySource> {
    let source = MemorySource::new()
        .with(
            ".kiro/vocabulary.toml",
            include_str!("../fixtures/vocab/tree/.kiro/vocabulary.toml"),
        )
        .with(
            ".kiro/steering/roadmap.md",
            include_str!("../fixtures/vocab/tree/.kiro/steering/roadmap.md"),
        )
        .with(
            ".kiro/specs/demo/spec.json",
            include_str!("../fixtures/vocab/tree/.kiro/specs/demo/spec.json"),
        )
        .with(
            ".kiro/specs/both/spec.json",
            include_str!("../fixtures/vocab/tree/.kiro/specs/both/spec.json"),
        )
        .with(
            ".kiro/specs/none/spec.json",
            include_str!("../fixtures/vocab/tree/.kiro/specs/none/spec.json"),
        )
        .with(
            ".kiro/specs/stale/spec.json",
            include_str!("../fixtures/vocab/tree/.kiro/specs/stale/spec.json"),
        );

    Workspace::new(
        source,
        vec![
            "demo".to_string(),
            "both".to_string(),
            "none".to_string(),
            "stale".to_string(),
        ],
    )
}

#[test]
fn fixture_tree_lookup_outcomes_and_order_insensitivity() {
    let mut ws = fixture_tree_workspace();
    let loaded = load(&mut ws);
    assert_eq!(loaded.status, Status::Loaded);

    let demo_feature = ws.load("demo");
    let demo_topics = spec_topics(demo_feature.meta.as_ref());
    assert_eq!(demo_topics, vec!["sdd"]);

    // demo: Found via Id ("task")
    match loaded.vocabulary.resolve("task", &demo_topics) {
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

    // demo: Found via Label ("Specification" matches label of "spec")
    match loaded.vocabulary.resolve("Specification", &demo_topics) {
        Lookup::Found {
            term, via, broader, ..
        } => {
            assert_eq!(term.id, "spec");
            assert_eq!(via, MatchVia::Label);
            assert!(broader.is_empty());
        }
        other => panic!("expected Found, got {other:?}"),
    }

    // demo: Found via AltLabel
    match loaded.vocabulary.resolve("criterion", &demo_topics) {
        Lookup::Found {
            term, via, broader, ..
        } => {
            assert_eq!(term.id, "acceptance-criterion");
            assert_eq!(via, MatchVia::AltLabel("criterion".into()));
            assert_eq!(broader.len(), 2);
            assert_eq!(broader[0].id, "requirement");
            assert_eq!(broader[1].id, "spec");
        }
        other => panic!("expected Found, got {other:?}"),
    }

    // demo: NotInTopic (sprint is in pm)
    match loaded.vocabulary.resolve("sprint", &demo_topics) {
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

    // both: Ambiguous ("Task" in both sdd and pm)
    let both_feature = ws.load("both");
    let both_topics = spec_topics(both_feature.meta.as_ref());
    assert_eq!(both_topics, vec!["sdd", "pm"]);

    match loaded.vocabulary.resolve("Task", &both_topics) {
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

    // Topic order insensitivity
    let res1 = loaded.vocabulary.resolve("Task", &both_topics);
    let mut reversed = both_topics.clone();
    reversed.reverse();
    let res2 = loaded.vocabulary.resolve("Task", &reversed);
    assert_eq!(res1, res2);

    // none: Unscoped
    let none_feature = ws.load("none");
    let none_topics = spec_topics(none_feature.meta.as_ref());
    assert_eq!(none_topics, Vec::<String>::new());
    assert_eq!(
        loaded.vocabulary.resolve("task", &none_topics),
        Lookup::Unscoped
    );

    // Unknown query
    assert_eq!(
        loaded.vocabulary.resolve("nonexistent", &demo_topics),
        Lookup::Unknown
    );
}

#[test]
fn fixture_tree_scope_findings() {
    let mut ws = fixture_tree_workspace();
    let loaded = load(&mut ws);

    // demo: clean scope findings
    let demo = ws.load("demo");
    let demo_topics = spec_topics(demo.meta.as_ref());
    let demo_findings = scope_findings("demo", ".kiro/specs/demo/spec.json", &demo_topics, &loaded);
    assert!(demo_findings.is_empty());

    // none: TOPICS_UNDECLARED
    let none = ws.load("none");
    let none_topics = spec_topics(none.meta.as_ref());
    let none_findings = scope_findings("none", ".kiro/specs/none/spec.json", &none_topics, &loaded);
    assert_eq!(none_findings.len(), 1);
    assert_eq!(none_findings[0].code, codes::TOPICS_UNDECLARED);

    // stale: TOPIC_UNKNOWN ("legacy")
    let stale = ws.load("stale");
    let stale_topics = spec_topics(stale.meta.as_ref());
    let stale_findings = scope_findings(
        "stale",
        ".kiro/specs/stale/spec.json",
        &stale_topics,
        &loaded,
    );
    assert_eq!(stale_findings.len(), 1);
    assert_eq!(stale_findings[0].code, codes::TOPIC_UNKNOWN);
    assert!(stale_findings[0].message.contains("legacy"));

    // Missing vocabulary suppresses TOPIC_UNKNOWN
    let missing_loaded = Loaded {
        status: Status::Missing,
        vocabulary: Default::default(),
        findings: vec![],
    };
    let missing_findings = scope_findings(
        "stale",
        ".kiro/specs/stale/spec.json",
        &stale_topics,
        &missing_loaded,
    );
    assert!(missing_findings.is_empty());
}

#[test]
fn seed_file_loads_cleanly_with_zero_findings_and_all_required_ids() {
    let loaded1 = parse(SEED_TOML, VOCABULARY_PATH);
    assert_eq!(loaded1.status, Status::Loaded);
    assert!(
        loaded1.findings.is_empty(),
        "seed vocabulary must load with zero findings, got: {:?}",
        loaded1.findings
    );

    let required_ids = [
        "spec",
        "brief",
        "requirement",
        "acceptance-criterion",
        "ears-pattern",
        "design",
        "task",
        "boundary",
        "component",
        "phase",
        "approval",
        "steering",
        "roadmap",
        "finding",
        "severity",
        "topic",
        "term",
        "vocabulary",
    ];

    let sdd = vec!["sdd".to_string()];
    for id in required_ids {
        match loaded1.vocabulary.preferred_label(id, &sdd) {
            Lookup::Found { term, .. } => assert_eq!(term.id, id),
            other => panic!("expected seed term `{id}` to be Found, got {other:?}"),
        }
    }

    // Two runs produce identical findings (determinism)
    let loaded2 = parse(SEED_TOML, VOCABULARY_PATH);
    assert_eq!(loaded1.findings, loaded2.findings);
}

#[test]
fn fixed_seed_garbage_never_panics() {
    // 256 pseudo-random garbage documents generated deterministically
    let mut state: u32 = 0x12345678;
    for _ in 0..256 {
        let mut doc = String::new();
        for _ in 0..64 {
            // Simple LCG
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let b = (state >> 24) as u8;
            if (32..=126).contains(&b) || b == b'\n' || b == b'\t' {
                doc.push(b as char);
            }
        }
        let _ = parse(&doc, VOCABULARY_PATH);
    }
}

#[test]
#[cfg(not(debug_assertions))]
fn ten_thousand_terms_depth_ten_builds_under_one_second() {
    let start = std::time::Instant::now();
    let mut content = String::from("schema_version = \"2\"\n\n");
    for i in 0..10 {
        content.push_str(&format!(
            "[terms.c{i}]\ntopic = \"sdd\"\nlabel = \"Chain {i}\"\ndefinition = \"def\"\n"
        ));
        if i > 0 {
            content.push_str(&format!("broader = [\"c{}\"]\n", i - 1));
        }
    }
    for i in 10..10_000 {
        content.push_str(&format!(
            "[terms.t{i}]\ntopic = \"sdd\"\nlabel = \"Term {i}\"\ndefinition = \"def\"\nbroader = [\"c9\"]\n"
        ));
    }

    let loaded = parse(&content, VOCABULARY_PATH);
    assert_eq!(loaded.status, Status::Loaded);
    assert_eq!(loaded.vocabulary.len(), 10_000);
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
}
