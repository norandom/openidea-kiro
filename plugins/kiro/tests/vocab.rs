//! Plugin end-to-end integration tests for vocabulary commands (task 4.4).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]

use openidea_plugin_kiro::commands::{activate, run_command};
use openidea_plugin_kiro::host::{FakeHost, HostEvent};

fn setup_tree_host() -> FakeHost {
    let host = FakeHost::new().with_root("C:\\proj");

    host.with_file(
        ".kiro/vocabulary.toml",
        include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/vocabulary.toml"),
    )
    .with_file(
        ".kiro/steering/roadmap.md",
        include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/steering/roadmap.md"),
    )
    .with_file(
        ".kiro/specs/demo/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/specs/demo/spec.json"),
    )
    .with_file(
        ".kiro/specs/both/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/specs/both/spec.json"),
    )
    .with_file(
        ".kiro/specs/none/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/specs/none/spec.json"),
    )
    .with_file(
        ".kiro/specs/stale/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/specs/stale/spec.json"),
    )
}

fn activate_host(host: &FakeHost) {
    activate(
        host,
        &[
            "project-read".to_owned(),
            "editor-read".to_owned(),
            "editor-write".to_owned(),
        ],
    )
    .expect("activation must succeed");
}

#[test]
fn term_command_outcomes_and_findings_line() {
    let host = setup_tree_host();
    activate_host(&host);

    // 1. Found via Label ("Specification") on demo
    let out = run_command(&host, "vocab.term.demo", Some("Specification"))
        .unwrap()
        .unwrap();
    assert!(out.contains("Term: spec"));
    assert!(out.contains("Label: Specification"));
    assert!(out.contains("Topic: sdd"));
    assert!(out.contains("Kind: entity"));
    assert!(out.contains("Matched via: label"));
    assert!(out.contains("Scope: demo (topics: sdd)"));
    assert!(out.contains("Vocabulary findings: 1 (run Kiro: Vocabulary Topics)"));

    // 2. Found via AltLabel ("criterion") with broader closure
    let out = run_command(&host, "vocab.term.demo", Some("criterion"))
        .unwrap()
        .unwrap();
    assert!(out.contains("Term: acceptance-criterion"));
    assert!(out.contains("Matched via: alt label \"criterion\""));
    assert!(out.contains("Broader: requirement > spec"));
    assert!(out.contains("Alt labels: criterion, AC"));

    // 3. Deprecated with replacement ("old-task")
    let out = run_command(&host, "vocab.term.demo", Some("old-task"))
        .unwrap()
        .unwrap();
    assert!(out.contains("Deprecated: yes, replaced by task (Task)"));

    // 4. Not in topic ("sprint" is in pm, demo declares sdd)
    let out = run_command(&host, "vocab.term.demo", Some("sprint"))
        .unwrap()
        .unwrap();
    assert!(out.contains("`sprint` is a term of another topic: pm/sprint."));
    assert!(out.contains("Spec demo declares: sdd."));
    assert!(out.contains("Cross-topic matches are never resolved"));

    // 5. Ambiguous across declared topics (query "Task" on "both" spec)
    let out = run_command(&host, "vocab.term.both", Some("Task"))
        .unwrap()
        .unwrap();
    assert!(
        out.contains("`Task` matches several terms in the declared topics: pm/pm-task, sdd/task.")
    );
    assert!(out.contains("Use the term id."));

    // 6. Unknown
    let out = run_command(&host, "vocab.term.demo", Some("nonexistent"))
        .unwrap()
        .unwrap();
    assert!(out.contains("`nonexistent` is not a term in topics sdd of spec demo."));

    // 7. Empty query: usage line + visible table
    let out = run_command(&host, "vocab.term.demo", None)
        .unwrap()
        .unwrap();
    assert!(out.contains("vocab.term expects a term label or id as its argument"));
    assert!(out.contains("Spec: demo (topics: sdd)"));
    assert!(out.contains("| Id"));
    assert!(out.contains("acceptance-criterion"));

    // 8. No topics declared on spec ("none")
    let out = run_command(&host, "vocab.term.none", Some("task"))
        .unwrap()
        .unwrap();
    assert!(out.contains(
        "Spec none declares no topics: add \"topics\": [\"sdd\"] to .kiro/specs/none/spec.json."
    ));
    assert!(out.contains("Available topics:"));
}

#[test]
fn list_command_rows_topic_filter_and_insertion() {
    let host = setup_tree_host();
    activate_host(&host);

    // 1. List visible terms for demo
    let out = run_command(&host, "vocab.list.demo", None)
        .unwrap()
        .unwrap();
    assert!(out.contains("Spec: demo (topics: sdd)"));
    assert!(out.contains("acceptance-criterion"));
    assert!(out.contains("Acceptance criterion"));
    assert!(out.contains("task"));
    assert!(out.contains("Task"));
    assert!(out.contains("Vocabulary findings: 1 (run Kiro: Vocabulary Topics)"));

    // 2. Topic filter for declared topic
    let out = run_command(&host, "vocab.list.demo", Some("sdd"))
        .unwrap()
        .unwrap();
    assert!(out.contains("Topic: sdd"));

    // 3. Topic filter for undeclared topic -> warning
    let out = run_command(&host, "vocab.list.demo", Some("pm"))
        .unwrap()
        .unwrap();
    assert!(out.contains("Topic pm is not declared by spec demo; declared: sdd."));

    // 4. Insertion via vocab.list.demo with "insert" argument
    let insert_out = run_command(&host, "vocab.list.demo", Some("insert"))
        .unwrap()
        .unwrap();
    assert!(insert_out.contains("Spec: demo (topics: sdd)"));

    let events = host.events();
    let insert_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, HostEvent::Insert(_)))
        .collect();
    assert_eq!(
        insert_events.len(),
        1,
        "vocab.list with insert must trigger exactly one HostEvent::Insert"
    );
    if let HostEvent::Insert(text) = insert_events[0] {
        assert!(text.contains("Spec: demo (topics: sdd)"));
        assert!(text.contains("acceptance-criterion"));
    }
}

#[test]
fn topics_command_lines_and_findings() {
    let host = setup_tree_host();
    activate_host(&host);

    // Topics on demo
    let out = run_command(&host, "vocab.topics.demo", None)
        .unwrap()
        .unwrap();
    assert!(out.contains("Spec: demo"));
    assert!(out.contains("Declared: sdd"));
    assert!(out.contains("Available:"));
    assert!(out.contains("Not declared: pm"));
    assert!(out.contains("Vocabulary: loaded"));
    assert!(out.contains(".kiro/vocabulary.toml:65: INFO VOCAB_LABEL_CROSS_TOPIC"));

    // Topics on none
    let out = run_command(&host, "vocab.topics.none", None)
        .unwrap()
        .unwrap();
    assert!(out.contains("Spec: none"));
    assert!(out.contains("Declared: none"));
    assert!(out.contains(".kiro/specs/none/spec.json: INFO TOPICS_UNDECLARED"));

    // Topics on stale
    let out = run_command(&host, "vocab.topics.stale", None)
        .unwrap()
        .unwrap();
    assert!(out.contains("Spec: stale"));
    assert!(out.contains("Declared: sdd, legacy"));
    assert!(out.contains("Unknown to the vocabulary: legacy"));
    assert!(out.contains(".kiro/specs/stale/spec.json: WARNING TOPIC_UNKNOWN"));
}

#[test]
fn missing_and_invalid_vocabulary_across_all_commands() {
    // Missing vocabulary
    let missing_host = FakeHost::new()
        .with_root("C:\\proj")
        .with_file(
            ".kiro/steering/roadmap.md",
            include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/steering/roadmap.md"),
        )
        .with_file(
            ".kiro/specs/demo/spec.json",
            include_str!(
                "../../../crates/kiro-core/fixtures/vocab/tree/.kiro/specs/demo/spec.json"
            ),
        );
    activate_host(&missing_host);

    for cmd in &["vocab.term.demo", "vocab.list.demo", "vocab.topics.demo"] {
        let res = run_command(&missing_host, cmd, None);
        assert!(res.is_ok(), "{cmd} must succeed on missing vocabulary");
        let text = res.unwrap().unwrap();
        assert!(
            text.contains("No vocabulary file: .kiro/vocabulary.toml not found"),
            "{cmd} must report missing vocabulary guidance"
        );
    }

    // Invalid vocabulary
    let invalid_host = FakeHost::new()
        .with_root("C:\\proj")
        .with_file(
            ".kiro/vocabulary.toml",
            include_str!("../../../crates/kiro-core/fixtures/vocab/invalid-syntax.toml"),
        )
        .with_file(
            ".kiro/steering/roadmap.md",
            include_str!("../../../crates/kiro-core/fixtures/vocab/tree/.kiro/steering/roadmap.md"),
        )
        .with_file(
            ".kiro/specs/demo/spec.json",
            include_str!(
                "../../../crates/kiro-core/fixtures/vocab/tree/.kiro/specs/demo/spec.json"
            ),
        );
    activate_host(&invalid_host);

    for cmd in &["vocab.term.demo", "vocab.list.demo", "vocab.topics.demo"] {
        let res = run_command(&invalid_host, cmd, None);
        assert!(res.is_ok(), "{cmd} must succeed on invalid vocabulary");
        let text = res.unwrap().unwrap();
        assert!(
            text.contains("VOCAB_INVALID"),
            "{cmd} must report VOCAB_INVALID finding"
        );
    }
}

#[test]
fn unresolved_spec_error_identical_to_plugin_core() {
    let host = setup_tree_host();
    activate_host(&host);

    let err = run_command(&host, "vocab.term.ghost", Some("task")).unwrap_err();
    assert_eq!(
        err,
        "spec `ghost` was not found in this project (no readable .kiro/specs/ghost/spec.json)"
    );
}

#[test]
fn vocabulary_path_read_at_most_once_per_command() {
    let host = setup_tree_host();
    activate_host(&host);

    // Initial activation
    let initial_reads = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Read(path) if path.contains("vocabulary.toml")))
        .count();
    assert_eq!(initial_reads, 0);

    // Run vocab.term
    run_command(&host, "vocab.term.demo", Some("task")).unwrap();
    let term_reads = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Read(path) if path.contains("vocabulary.toml")))
        .count();
    assert_eq!(
        term_reads, 1,
        "vocabulary file must be read exactly once during vocab.term"
    );
}

#[test]
fn per_spec_variants_registered_for_all_known_specs() {
    let host = setup_tree_host();
    activate_host(&host);

    // Run any vocabulary command to trigger variant synchronization
    run_command(&host, "vocab.term.demo", Some("task")).unwrap();

    let registered = host.registered();
    for spec in &["demo", "both", "none", "stale"] {
        assert!(
            registered.contains_key(&format!("vocab.term.{spec}")),
            "missing vocab.term.{spec}"
        );
        assert!(
            registered.contains_key(&format!("vocab.list.{spec}")),
            "missing vocab.list.{spec}"
        );
        assert!(
            registered.contains_key(&format!("vocab.topics.{spec}")),
            "missing vocab.topics.{spec}"
        );
    }
}
