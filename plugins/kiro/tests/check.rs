//! Plugin end-to-end integration tests for check commands (task 6.3).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]

use openidea_plugin_kiro::commands::{activate, run_command};
use openidea_plugin_kiro::host::{FakeHost, HostEvent, Level};

fn setup_good_host() -> FakeHost {
    let host = FakeHost::new().with_root("C:\\proj");

    host.with_file(
        ".kiro/vocabulary.toml",
        include_str!("../../../crates/kiro-core/fixtures/checks/good/.kiro/vocabulary.toml"),
    )
    .with_file(
        ".kiro/steering/roadmap.md",
        include_str!("../../../crates/kiro-core/fixtures/checks/good/.kiro/steering/roadmap.md"),
    )
    .with_file(
        ".kiro/specs/clean/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/checks/good/.kiro/specs/clean/spec.json"),
    )
    .with_file(
        ".kiro/specs/clean/requirements.md",
        include_str!(
            "../../../crates/kiro-core/fixtures/checks/good/.kiro/specs/clean/requirements.md"
        ),
    )
}

fn setup_bad_host() -> FakeHost {
    let host = FakeHost::new().with_root("C:\\proj");

    host.with_file(
        ".kiro/vocabulary.toml",
        include_str!("../../../crates/kiro-core/fixtures/checks/bad/.kiro/vocabulary.toml"),
    )
    .with_file(
        ".kiro/steering/roadmap.md",
        include_str!("../../../crates/kiro-core/fixtures/checks/bad/.kiro/steering/roadmap.md"),
    )
    .with_file(
        ".kiro/specs/broken/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/checks/bad/.kiro/specs/broken/spec.json"),
    )
    .with_file(
        ".kiro/specs/broken/requirements.md",
        include_str!(
            "../../../crates/kiro-core/fixtures/checks/bad/.kiro/specs/broken/requirements.md"
        ),
    )
    .with_file(
        ".kiro/specs/none/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/checks/bad/.kiro/specs/none/spec.json"),
    )
    .with_file(
        ".kiro/specs/none/requirements.md",
        include_str!(
            "../../../crates/kiro-core/fixtures/checks/bad/.kiro/specs/none/requirements.md"
        ),
    )
    .with_file(
        ".kiro/specs/wide/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/checks/bad/.kiro/specs/wide/spec.json"),
    )
    .with_file(
        ".kiro/specs/wide/requirements.md",
        include_str!(
            "../../../crates/kiro-core/fixtures/checks/bad/.kiro/specs/wide/requirements.md"
        ),
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

fn last_notification(host: &FakeHost) -> (Level, String, String) {
    host.events()
        .into_iter()
        .rev()
        .find_map(|e| match e {
            HostEvent::Notify { level, title, body } => Some((level, title, body)),
            _ => None,
        })
        .expect("expected a notify event")
}

#[test]
fn spec_command_on_clean_spec() {
    let host = setup_good_host();
    activate_host(&host);

    let res = run_command(&host, "check.spec.clean", None)
        .expect("command must succeed")
        .expect("must return output text");

    let (level, title, body) = last_notification(&host);
    assert_eq!(level, Level::Info);
    assert_eq!(title, "Kiro: clean — 0 errors, 0 warnings, 1 info");

    // Scope line
    assert!(body.contains("Spec: clean (topics: ops) — vocabulary: loaded"));

    // Three status lines
    assert!(body.contains("EARS: 6 criteria checked, 0 findings"));
    assert!(body.contains(
        "Terms: 10 declared — 10 resolved, 0 deprecated, 0 mismatched, 0 undeclared, 0 ambiguous; 0 suggested"
    ));
    assert!(body.contains(
        "Contradictions: 4 guarded criteria, 3 pairs considered, 0 found, 0 refused (caps: 512 variables, 100000 nodes)"
    ));

    // CHECK_COVERAGE finding line
    assert!(body.contains("CHECK_COVERAGE"));
    assert!(
        body.contains("6 criteria, 7 annotated, 3 pairs considered, 0 contradictions, 0 refused")
    );

    assert_eq!(res, body);
}

#[test]
fn spec_command_on_broken_spec() {
    let host = setup_bad_host();
    activate_host(&host);

    let res = run_command(&host, "check.spec.broken", None)
        .expect("command must succeed")
        .expect("must return output text");

    let (level, title, body) = last_notification(&host);
    println!("CONTRADICTIONS BODY:\n{body}");
    assert_eq!(level, Level::Error);
    assert!(title.starts_with("Kiro: broken — "));
    assert!(title.contains("errors"));

    // Status lines present
    assert!(body.contains("EARS:"));
    assert!(body.contains("Terms:"));
    assert!(body.contains("Contradictions:"));

    // Contradiction lines present
    assert!(body.contains("CONTRADICTION"));
    assert!(body.contains("witness:"));
    assert!(body.contains("CHECK_COVERAGE"));

    assert_eq!(res, body);
}

#[test]
fn ears_command_patterns_and_pattern_reports() {
    let host = setup_bad_host();
    activate_host(&host);

    let res = run_command(&host, "check.ears.broken", None)
        .expect("command must succeed")
        .expect("must return output text");

    let (level, title, body) = last_notification(&host);
    println!("CONTRADICTIONS BODY:\n{body}");
    assert_eq!(level, Level::Error);
    assert!(title.starts_with("Kiro: broken — "));

    // EARS status line followed by Patterns line
    assert!(body.contains("EARS:"));
    assert!(body.contains("criteria checked"));
    assert!(body.contains("Patterns: ubiquitous "));
    assert!(body.contains("event-driven "));
    assert!(body.contains("state-driven "));
    assert!(body.contains("unwanted-behaviour "));
    assert!(body.contains("optional-feature "));
    assert!(body.contains("complex "));
    assert!(body.contains("unclassified "));

    // Neither Terms nor Contradictions lines appear
    assert!(!body.contains("Terms:"));
    assert!(!body.contains("Contradictions:"));

    // Pattern reports (EARS_PATTERN) present
    assert!(body.contains("EARS_PATTERN"));

    assert_eq!(res, body);
}

#[test]
fn terms_command_counts_and_mismatch() {
    let host = setup_bad_host();
    activate_host(&host);

    let res = run_command(&host, "check.terms.broken", None)
        .expect("command must succeed")
        .expect("must return output text");

    let (level, title, body) = last_notification(&host);
    println!("CONTRADICTIONS BODY:\n{body}");
    assert_eq!(level, Level::Error);
    assert!(title.starts_with("Kiro: broken — "));

    // Terms status line
    assert!(body.contains("Terms: "));
    assert!(body.contains("declared — "));
    assert!(body.contains("resolved, "));
    assert!(body.contains("deprecated, "));
    assert!(body.contains("mismatched, "));
    assert!(body.contains("undeclared, "));
    assert!(body.contains("ambiguous; "));
    assert!(body.contains("suggested"));

    // Term findings
    assert!(body.contains("TERM_TOPIC_MISMATCH"));
    assert!(body.contains("TERM_UNDECLARED"));
    assert!(body.contains("TERM_DEPRECATED"));
    assert!(body.contains("TERM_AMBIGUOUS"));

    // EARS and Contradictions lines do not appear
    assert!(!body.contains("EARS:"));
    assert!(!body.contains("Contradictions:"));

    assert_eq!(res, body);
}

#[test]
fn contradictions_command_pairs_and_witness() {
    let host = setup_bad_host();
    activate_host(&host);

    let res = run_command(&host, "check.contradictions.broken", None)
        .expect("command must succeed")
        .expect("must return output text");

    let (level, title, body) = last_notification(&host);
    assert_eq!(level, Level::Error);
    assert!(title.starts_with("Kiro: broken — "));

    // Contradictions status line with caps
    assert!(body.contains(
        "Contradictions: 3 guarded criteria, 2 pairs considered, 2 found, 0 refused (caps: 512 variables, 100000 nodes)"
    ));

    // Both witnesses present
    assert!(body.contains("CONTRADICTION"));
    assert!(body.contains("witness: depth >= 1001, mode = safe"));
    assert!(body.contains("witness: in every state"));

    // EARS and Terms lines do not appear
    assert!(!body.contains("EARS:"));
    assert!(!body.contains("Terms:"));

    assert_eq!(res, body);
}

#[test]
fn unscoped_spec_skipped_lines_unscoped_finding_and_pointer() {
    let host = setup_bad_host();
    activate_host(&host);

    let res = run_command(&host, "check.spec.none", None)
        .expect("command must succeed")
        .expect("must return output text");

    let (level, title, body) = last_notification(&host);
    assert_eq!(level, Level::Warning);
    assert_eq!(title, "Kiro: none — 0 errors, 1 warning, 1 info");

    // Scope line indicates no topics
    assert!(body.contains("Spec: none (topics: none) — vocabulary: loaded"));

    // EARS ran; Terms and Contradictions skipped
    assert!(body.contains("EARS: 1 criteria checked, 0 findings"));
    assert!(body.contains("Terms: skipped (spec declares no topics)"));
    assert!(body.contains("Contradictions: skipped (spec declares no topics)"));

    // Findings
    assert!(body.contains("TOPICS_UNDECLARED"));
    assert!(body.contains("CHECK_UNSCOPED"));

    // Vocabulary documentation pointer
    assert!(body.contains("See docs/vocabulary.md or run Kiro: Vocabulary Topics."));

    assert_eq!(res, body);
}

#[test]
fn missing_and_invalid_vocabulary_across_commands() {
    // 1. Missing vocabulary
    let missing_host = FakeHost::new()
        .with_root("C:\\proj")
        .with_file(
            ".kiro/steering/roadmap.md",
            include_str!(
                "../../../crates/kiro-core/fixtures/checks/good/.kiro/steering/roadmap.md"
            ),
        )
        .with_file(
            ".kiro/specs/clean/spec.json",
            include_str!(
                "../../../crates/kiro-core/fixtures/checks/good/.kiro/specs/clean/spec.json"
            ),
        )
        .with_file(
            ".kiro/specs/clean/requirements.md",
            include_str!(
                "../../../crates/kiro-core/fixtures/checks/good/.kiro/specs/clean/requirements.md"
            ),
        );
    activate_host(&missing_host);

    for cmd in &[
        "check.spec.clean",
        "check.ears.clean",
        "check.terms.clean",
        "check.contradictions.clean",
    ] {
        let res = run_command(&missing_host, cmd, None);
        assert!(res.is_ok(), "{cmd} must succeed when vocabulary is missing");
    }

    let res = run_command(&missing_host, "check.spec.clean", None)
        .unwrap()
        .unwrap();
    assert!(res.contains("VOCAB_MISSING"));
    assert!(res.contains("CHECK_UNSCOPED"));
    assert!(res.contains("Terms: skipped (vocabulary missing)"));
    assert!(res.contains("Contradictions: skipped (vocabulary missing)"));
    assert!(res.contains("See docs/vocabulary.md or run Kiro: Vocabulary Topics."));

    // 2. Invalid vocabulary
    let invalid_host = FakeHost::new()
        .with_root("C:\\proj")
        .with_file(".kiro/vocabulary.toml", "invalid [toml syntax")
        .with_file(
            ".kiro/steering/roadmap.md",
            include_str!(
                "../../../crates/kiro-core/fixtures/checks/good/.kiro/steering/roadmap.md"
            ),
        )
        .with_file(
            ".kiro/specs/clean/spec.json",
            include_str!(
                "../../../crates/kiro-core/fixtures/checks/good/.kiro/specs/clean/spec.json"
            ),
        )
        .with_file(
            ".kiro/specs/clean/requirements.md",
            include_str!(
                "../../../crates/kiro-core/fixtures/checks/good/.kiro/specs/clean/requirements.md"
            ),
        );
    activate_host(&invalid_host);

    for cmd in &[
        "check.spec.clean",
        "check.ears.clean",
        "check.terms.clean",
        "check.contradictions.clean",
    ] {
        let res = run_command(&invalid_host, cmd, None);
        assert!(res.is_ok(), "{cmd} must succeed when vocabulary is invalid");
    }

    let res = run_command(&invalid_host, "check.spec.clean", None)
        .unwrap()
        .unwrap();
    assert!(res.contains("VOCAB_INVALID"));
    assert!(res.contains("CHECK_UNSCOPED"));
    assert!(res.contains("Terms: skipped (vocabulary invalid)"));
    assert!(res.contains("Contradictions: skipped (vocabulary invalid)"));
    assert!(res.contains("See docs/vocabulary.md or run Kiro: Vocabulary Topics."));
}

#[test]
fn check_spec_insert_produces_one_insert_event_with_full_text() {
    let host = setup_good_host();
    activate_host(&host);

    let res = run_command(&host, "check.spec.insert", Some("clean"))
        .expect("command must succeed")
        .expect("must return text");

    let inserts: Vec<_> = host
        .events()
        .into_iter()
        .filter_map(|e| match e {
            HostEvent::Insert(text) => Some(text),
            _ => None,
        })
        .collect();

    assert_eq!(inserts.len(), 1);
    assert_eq!(inserts[0], res);
}

#[test]
fn unresolved_spec_error_identical_to_plugin_core() {
    let mut host = setup_bad_host();
    activate_host(&host);

    // 1. Explicit unknown spec
    let err = run_command(&host, "check.spec.ghost", None).unwrap_err();
    assert_eq!(
        err,
        "spec `ghost` was not found in this project (no readable .kiro/specs/ghost/spec.json)"
    );

    // 2. Ambiguous resolution: multiple known specs, no active file
    host.active = None;
    let err2 = run_command(&host, "check.spec", None).unwrap_err();
    assert_eq!(
        err2,
        "no current spec: open a file under .kiro/specs/<spec>/ or run the per-spec command; candidates: broken, none, wide"
    );
}

#[test]
fn vocabulary_path_read_at_most_once_per_command() {
    let host = setup_good_host();
    activate_host(&host);

    // Initial activation reads
    let initial_reads = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Read(path) if path.contains("vocabulary.toml")))
        .count();
    assert_eq!(initial_reads, 0);

    // Run check.spec
    run_command(&host, "check.spec.clean", None).unwrap();
    let spec_reads = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Read(path) if path.contains("vocabulary.toml")))
        .count();
    assert_eq!(
        spec_reads, 1,
        "vocabulary file must be read exactly once during check.spec"
    );

    // Run check.terms
    run_command(&host, "check.terms.clean", None).unwrap();
    let terms_reads = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Read(path) if path.contains("vocabulary.toml")))
        .count();
    // After two commands, at most 2 reads total (exactly 1 read per command)
    assert_eq!(
        terms_reads - spec_reads,
        1,
        "vocabulary file must be read at most once per command"
    );
}
