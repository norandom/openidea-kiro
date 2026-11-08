//! Integration tests for the constraints engine (tasks 4.2 and 4.3).
//!
//! Validates:
//! - Fixture-driven checks: good/clean, bad/broken, bad/wide, bad/none
//! - Missing and corrupted vocabulary trees
//! - EARS regressions corpus (v1 false positive fixes and true positives)
//! - Single-check configuration statuses
//! - Severity overrides on check finding codes
//! - Determinism across multiple runs
//! - Self-corpus requirements documents from this repository
//! - Generated-model cross-check (BDD vs brute-force Typed::eval)
//! - Garbage no-panic totality
//! - Release-only timing benchmarks (< 1s for 1000 criteria EARS+terms, < 1s for 200 contradiction criteria)

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::cloned_ref_to_slice_refs
)]

use std::collections::BTreeMap;
#[cfg(not(debug_assertions))]
use std::time::{Duration, Instant};

use kiro_core::checks::bdd::Encoding;
use kiro_core::checks::contradiction::search as contradiction_search;
use kiro_core::checks::ears::check as ears_check;
use kiro_core::checks::finitize::{Atom, Domain, Typed, Value, finitize};
use kiro_core::checks::guard::{parse_effect, parse_guard};
use kiro_core::checks::terms::resolve_scope;
use kiro_core::checks::{CheckConfig, CheckKind, CheckStatus, run_checks};
use kiro_core::finding::{Severity, codes};
use kiro_core::requirements::parse as parse_requirements;
use kiro_core::source::{MemorySource, Workspace};
use kiro_core::vocab::{VOCABULARY_PATH, parse as vocab_parse};

// Fixture constants
const GOOD_VOCAB: &str = include_str!("../fixtures/checks/good/.kiro/vocabulary.toml");
const GOOD_ROADMAP: &str = include_str!("../fixtures/checks/good/.kiro/steering/roadmap.md");
const GOOD_CLEAN_SPEC: &str = include_str!("../fixtures/checks/good/.kiro/specs/clean/spec.json");
const GOOD_CLEAN_REQ: &str =
    include_str!("../fixtures/checks/good/.kiro/specs/clean/requirements.md");

const BAD_VOCAB: &str = include_str!("../fixtures/checks/bad/.kiro/vocabulary.toml");
const BAD_ROADMAP: &str = include_str!("../fixtures/checks/bad/.kiro/steering/roadmap.md");
const BAD_BROKEN_SPEC: &str = include_str!("../fixtures/checks/bad/.kiro/specs/broken/spec.json");
const BAD_BROKEN_REQ: &str =
    include_str!("../fixtures/checks/bad/.kiro/specs/broken/requirements.md");
const BAD_WIDE_SPEC: &str = include_str!("../fixtures/checks/bad/.kiro/specs/wide/spec.json");
const BAD_WIDE_REQ: &str = include_str!("../fixtures/checks/bad/.kiro/specs/wide/requirements.md");
const BAD_NONE_SPEC: &str = include_str!("../fixtures/checks/bad/.kiro/specs/none/spec.json");
const BAD_NONE_REQ: &str = include_str!("../fixtures/checks/bad/.kiro/specs/none/requirements.md");

const EARS_REGRESSIONS: &str = include_str!("../fixtures/checks/ears/regressions.md");

// Self-corpus fixtures
const SEED_VOCAB: &str = include_str!("../../../.kiro/vocabulary.toml");
const SEED_ROADMAP: &str = include_str!("../../../.kiro/steering/roadmap.md");
const SELF_CLI_GATE_SPEC: &str = include_str!("../../../.kiro/specs/cli-gate/spec.json");
const SELF_CLI_GATE_REQ: &str = include_str!("../../../.kiro/specs/cli-gate/requirements.md");
const SELF_KIRO_CORE_SPEC: &str = include_str!("../../../.kiro/specs/kiro-core/spec.json");
const SELF_KIRO_CORE_REQ: &str = include_str!("../../../.kiro/specs/kiro-core/requirements.md");
const SELF_PLUGIN_CORE_SPEC: &str = include_str!("../../../.kiro/specs/plugin-core/spec.json");
const SELF_PLUGIN_CORE_REQ: &str = include_str!("../../../.kiro/specs/plugin-core/requirements.md");
const SELF_SPEC_CONSTRAINTS_SPEC: &str =
    include_str!("../../../.kiro/specs/spec-constraints/spec.json");
const SELF_SPEC_CONSTRAINTS_REQ: &str =
    include_str!("../../../.kiro/specs/spec-constraints/requirements.md");
const SELF_SPEC_NAVIGATION_SPEC: &str =
    include_str!("../../../.kiro/specs/spec-navigation/spec.json");
const SELF_SPEC_NAVIGATION_REQ: &str =
    include_str!("../../../.kiro/specs/spec-navigation/requirements.md");
const SELF_SPEC_VISUALIZATION_SPEC: &str =
    include_str!("../../../.kiro/specs/spec-visualization/spec.json");
const SELF_SPEC_VISUALIZATION_REQ: &str =
    include_str!("../../../.kiro/specs/spec-visualization/requirements.md");
const SELF_VOCABULARY_GROUNDING_SPEC: &str =
    include_str!("../../../.kiro/specs/vocabulary-grounding/spec.json");
const SELF_VOCABULARY_GROUNDING_REQ: &str =
    include_str!("../../../.kiro/specs/vocabulary-grounding/requirements.md");

fn build_good_workspace() -> (Workspace<MemorySource>, String) {
    let src = MemorySource::new()
        .with(".kiro/vocabulary.toml", GOOD_VOCAB)
        .with(".kiro/steering/roadmap.md", GOOD_ROADMAP)
        .with(".kiro/specs/clean/spec.json", GOOD_CLEAN_SPEC)
        .with(".kiro/specs/clean/requirements.md", GOOD_CLEAN_REQ);
    (
        Workspace::new(src, vec!["clean".to_string()]),
        "clean".to_string(),
    )
}

fn build_bad_workspace(feature: &str) -> (Workspace<MemorySource>, String) {
    let src = MemorySource::new()
        .with(".kiro/vocabulary.toml", BAD_VOCAB)
        .with(".kiro/steering/roadmap.md", BAD_ROADMAP)
        .with(".kiro/specs/broken/spec.json", BAD_BROKEN_SPEC)
        .with(".kiro/specs/broken/requirements.md", BAD_BROKEN_REQ)
        .with(".kiro/specs/wide/spec.json", BAD_WIDE_SPEC)
        .with(".kiro/specs/wide/requirements.md", BAD_WIDE_REQ)
        .with(".kiro/specs/none/spec.json", BAD_NONE_SPEC)
        .with(".kiro/specs/none/requirements.md", BAD_NONE_REQ);
    (
        Workspace::new(
            src,
            vec!["broken".to_string(), "wide".to_string(), "none".to_string()],
        ),
        feature.to_string(),
    )
}

#[test]
fn fixture_good_clean_default_and_patterns() {
    let (mut ws, feat_name) = build_good_workspace();

    // Default configuration (patterns off)
    let rep = run_checks(&mut ws, &feat_name, &CheckConfig::default());
    let (errs, warns, infos) = rep.counts();
    assert_eq!(errs, 0, "good fixture must produce 0 errors");
    assert_eq!(warns, 0, "good fixture must produce 0 warnings");
    assert_eq!(
        infos, 1,
        "good fixture must produce exactly 1 info finding (CHECK_COVERAGE)"
    );
    assert_eq!(rep.findings[0].code, codes::CHECK_COVERAGE);

    assert_eq!(
        rep.status_of(CheckKind::Ears),
        &CheckStatus::Ran { findings: 0 }
    );
    assert_eq!(
        rep.status_of(CheckKind::Terms),
        &CheckStatus::Ran { findings: 0 }
    );
    assert_eq!(
        rep.status_of(CheckKind::Contradictions),
        &CheckStatus::Ran { findings: 0 }
    );

    let ears_sum = rep.ears.as_ref().unwrap();
    assert_eq!(ears_sum.criteria, 6);
    assert_eq!(ears_sum.unclassified, 0);

    let terms_sum = rep.terms.as_ref().unwrap();
    assert_eq!(terms_sum.declared, 10);
    assert_eq!(terms_sum.resolved, 10);
    assert_eq!(terms_sum.undeclared, 0);

    let cont_sum = rep.contradictions.as_ref().unwrap();
    assert_eq!(cont_sum.guarded, 4);
    assert_eq!(cont_sum.contradictions, 0);
    assert_eq!(cont_sum.refused_pairs, 0);

    // With pattern reporting enabled: 6 criteria each produce 1 EARS_PATTERN finding
    let cfg_patterns = CheckConfig {
        report_patterns: true,
        ..Default::default()
    };
    let rep_pat = run_checks(&mut ws, &feat_name, &cfg_patterns);
    assert_eq!(rep_pat.findings.len(), 7); // 6 EARS_PATTERN + 1 CHECK_COVERAGE

    let pattern_findings: Vec<_> = rep_pat
        .findings
        .iter()
        .filter(|f| f.code == codes::EARS_PATTERN)
        .collect();
    assert_eq!(pattern_findings.len(), 6);
    assert!(pattern_findings[0].message.contains("pattern: ubiquitous"));
    assert!(
        pattern_findings[1]
            .message
            .contains("pattern: event-driven")
    );
    assert!(
        pattern_findings[2]
            .message
            .contains("pattern: state-driven")
    );
    assert!(
        pattern_findings[3]
            .message
            .contains("pattern: unwanted-behaviour")
    );
    assert!(
        pattern_findings[4]
            .message
            .contains("pattern: optional-feature")
    );
    assert!(pattern_findings[5].message.contains("pattern: complex"));
}

#[test]
fn fixture_bad_broken_exact_multiset_and_witnesses() {
    let (mut ws, feat_name) = build_bad_workspace("broken");
    let rep = run_checks(&mut ws, &feat_name, &CheckConfig::default());

    let mut code_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for f in &rep.findings {
        *code_counts.entry(f.code).or_default() += 1;
    }

    // Exact expected finding code multiset
    assert_eq!(code_counts.get(codes::CHECK_COVERAGE), Some(&1));
    assert_eq!(code_counts.get(codes::TERM_TOPIC_MISMATCH), Some(&1));
    assert_eq!(code_counts.get(codes::TERM_UNDECLARED), Some(&1));
    assert_eq!(code_counts.get(codes::TERM_AMBIGUOUS), Some(&1));
    assert_eq!(code_counts.get(codes::TERM_DEPRECATED), Some(&1));
    assert_eq!(code_counts.get(codes::TERM_SUGGESTED), Some(&1));
    assert_eq!(code_counts.get(codes::EARS_NO_SHALL), Some(&2));
    assert_eq!(code_counts.get(codes::EARS_MULTI_SHALL), Some(&1));
    assert_eq!(code_counts.get(codes::EARS_MODAL), Some(&1));
    assert_eq!(code_counts.get(codes::EARS_NO_TRIGGER), Some(&1));
    assert_eq!(code_counts.get(codes::EARS_UNPARSED), Some(&1));
    assert_eq!(code_counts.get(codes::GUARD_UNPARSED), Some(&4));
    assert_eq!(code_counts.get(codes::GUARD_TERM_UNDECLARED), Some(&1));
    assert_eq!(code_counts.get(codes::GUARD_KIND_MISMATCH), Some(&1));
    assert_eq!(code_counts.get(codes::GUARD_UNSATISFIABLE), Some(&1));
    assert_eq!(code_counts.get(codes::CONFLICT_REF_UNKNOWN), Some(&1));
    assert_eq!(code_counts.get(codes::CONTRADICTION), Some(&2));

    // Verify the two witness strings
    let contradictions: Vec<_> = rep
        .findings
        .iter()
        .filter(|f| f.code == codes::CONTRADICTION)
        .collect();
    assert_eq!(contradictions.len(), 2);

    let effect_clash = contradictions
        .iter()
        .find(|f| f.message.contains("effect armed"))
        .expect("effect clash contradiction must be reported");
    assert!(
        effect_clash
            .message
            .contains("witness: depth >= 1001, mode = safe")
            || effect_clash
                .message
                .contains("witness: mode = safe, depth >= 1001"),
        "witness was: {}",
        effect_clash.message
    );

    let declared_conflict = contradictions
        .iter()
        .find(|f| f.message.contains("declared conflict"))
        .expect("declared conflict contradiction must be reported");
    assert!(
        declared_conflict
            .message
            .contains("witness: in every state"),
        "witness was: {}",
        declared_conflict.message
    );
}

#[test]
fn fixture_bad_wide_variable_cap_refusal() {
    let (mut ws, feat_name) = build_bad_workspace("wide");
    let rep = run_checks(&mut ws, &feat_name, &CheckConfig::default());

    let refused = rep
        .findings
        .iter()
        .find(|f| f.code == codes::SEARCH_REFUSED)
        .expect("SEARCH_REFUSED finding must be present");

    assert!(
        refused
            .message
            .contains("variable cap 512 exceeded: 520 values")
    );
    assert!(refused.message.contains("1 pairs not checked: 1.1/1.2"));
    assert!(
        refused
            .message
            .contains("largest terms: w01 (8 values), w02 (8 values)")
    );

    // Exactly 0 contradictions because search was refused
    assert!(!rep.findings.iter().any(|f| f.code == codes::CONTRADICTION));
    assert!(rep.findings.iter().any(|f| f.code == codes::CHECK_COVERAGE));
}

#[test]
fn fixture_bad_none_unscoped_spec() {
    let (mut ws, feat_name) = build_bad_workspace("none");
    let rep = run_checks(&mut ws, &feat_name, &CheckConfig::default());

    assert!(
        rep.findings
            .iter()
            .any(|f| f.code == codes::TOPICS_UNDECLARED)
    );
    assert!(rep.findings.iter().any(|f| f.code == codes::CHECK_UNSCOPED));

    // Neither terms nor contradictions ran: CHECK_COVERAGE must be absent
    assert!(!rep.findings.iter().any(|f| f.code == codes::CHECK_COVERAGE));

    assert_eq!(
        rep.status_of(CheckKind::Ears),
        &CheckStatus::Ran { findings: 0 }
    );
    assert_eq!(
        rep.status_of(CheckKind::Terms),
        &CheckStatus::Skipped {
            reason: "spec declares no topics".to_string()
        }
    );
    assert_eq!(
        rep.status_of(CheckKind::Contradictions),
        &CheckStatus::Skipped {
            reason: "spec declares no topics".to_string()
        }
    );
}

#[test]
fn missing_and_corrupted_vocabulary_trees() {
    // Missing vocabulary
    let src_missing = MemorySource::new()
        .with(".kiro/steering/roadmap.md", GOOD_ROADMAP)
        .with(".kiro/specs/clean/spec.json", GOOD_CLEAN_SPEC)
        .with(".kiro/specs/clean/requirements.md", GOOD_CLEAN_REQ);
    let mut ws_missing = Workspace::new(src_missing, vec!["clean".to_string()]);
    let rep_missing = run_checks(&mut ws_missing, "clean", &CheckConfig::default());

    assert_eq!(
        rep_missing.status_of(CheckKind::Terms),
        &CheckStatus::Skipped {
            reason: "vocabulary missing".to_string()
        }
    );
    assert_eq!(
        rep_missing.status_of(CheckKind::Contradictions),
        &CheckStatus::Skipped {
            reason: "vocabulary missing".to_string()
        }
    );
    assert!(
        rep_missing
            .findings
            .iter()
            .any(|f| f.code == codes::VOCAB_MISSING)
    );
    assert!(
        rep_missing
            .findings
            .iter()
            .any(|f| f.code == codes::CHECK_UNSCOPED)
    );

    // Corrupted vocabulary
    let src_corrupt = MemorySource::new()
        .with(".kiro/vocabulary.toml", "invalid [toml syntax")
        .with(".kiro/steering/roadmap.md", GOOD_ROADMAP)
        .with(".kiro/specs/clean/spec.json", GOOD_CLEAN_SPEC)
        .with(".kiro/specs/clean/requirements.md", GOOD_CLEAN_REQ);
    let mut ws_corrupt = Workspace::new(src_corrupt, vec!["clean".to_string()]);
    let rep_corrupt = run_checks(&mut ws_corrupt, "clean", &CheckConfig::default());

    assert_eq!(
        rep_corrupt.status_of(CheckKind::Terms),
        &CheckStatus::Skipped {
            reason: "vocabulary invalid".to_string()
        }
    );
    assert_eq!(
        rep_corrupt.status_of(CheckKind::Contradictions),
        &CheckStatus::Skipped {
            reason: "vocabulary invalid".to_string()
        }
    );
    assert!(
        rep_corrupt
            .findings
            .iter()
            .any(|f| f.code == codes::VOCAB_INVALID)
    );
    assert!(
        rep_corrupt
            .findings
            .iter()
            .any(|f| f.code == codes::CHECK_UNSCOPED)
    );
}

#[test]
fn ears_regressions_document_corpus() {
    let doc = parse_requirements(EARS_REGRESSIONS, "regressions.md");
    let (findings, summary) = ears_check(&doc, "regressions", false);

    assert_eq!(summary.criteria, 12);
    assert_eq!(summary.unclassified, 3);
    assert_eq!(findings.len(), 7);

    let mut codes_present = Vec::new();
    for f in &findings {
        codes_present.push(f.code);
    }

    assert_eq!(
        codes_present,
        vec![
            codes::EARS_NO_SHALL,
            codes::EARS_MULTI_SHALL,
            codes::EARS_NO_SHALL,
            codes::EARS_MODAL,
            codes::EARS_NO_TRIGGER,
            codes::EARS_UNPARSED,
            codes::EARS_UNPARSED,
        ]
    );
}

#[test]
fn single_check_statuses_and_severity_overrides() {
    let (mut ws, feat_name) = build_good_workspace();

    // Single check: EARS only
    let rep_ears = run_checks(&mut ws, &feat_name, &CheckConfig::only(CheckKind::Ears));
    assert_eq!(
        rep_ears.status_of(CheckKind::Ears),
        &CheckStatus::Ran { findings: 0 }
    );
    assert_eq!(
        rep_ears.status_of(CheckKind::Terms),
        &CheckStatus::Skipped {
            reason: "disabled by configuration".to_string()
        }
    );
    assert_eq!(
        rep_ears.status_of(CheckKind::Contradictions),
        &CheckStatus::Skipped {
            reason: "disabled by configuration".to_string()
        }
    );

    // Single check: Terms only
    let rep_terms = run_checks(&mut ws, &feat_name, &CheckConfig::only(CheckKind::Terms));
    assert_eq!(
        rep_terms.status_of(CheckKind::Ears),
        &CheckStatus::Skipped {
            reason: "disabled by configuration".to_string()
        }
    );
    assert_eq!(
        rep_terms.status_of(CheckKind::Terms),
        &CheckStatus::Ran { findings: 0 }
    );
    assert_eq!(
        rep_terms.status_of(CheckKind::Contradictions),
        &CheckStatus::Skipped {
            reason: "disabled by configuration".to_string()
        }
    );

    // Severity override on broken
    let (mut ws_broken, feat_broken) = build_bad_workspace("broken");
    let mut cfg_override = CheckConfig::default();
    cfg_override
        .severity_overrides
        .insert(codes::CONTRADICTION.to_string(), Severity::Warning);

    let rep_overridden = run_checks(&mut ws_broken, &feat_broken, &cfg_override);
    let conts: Vec<_> = rep_overridden
        .findings
        .iter()
        .filter(|f| f.code == codes::CONTRADICTION)
        .collect();
    assert_eq!(conts.len(), 2);
    for c in conts {
        assert_eq!(c.severity, Severity::Warning);
    }
}

#[test]
fn determinism_across_two_runs_of_all_fixtures() {
    let fixtures = ["clean", "broken", "wide", "none"];
    for name in fixtures {
        let (mut ws1, f1) = if name == "clean" {
            build_good_workspace()
        } else {
            build_bad_workspace(name)
        };
        let (mut ws2, f2) = if name == "clean" {
            build_good_workspace()
        } else {
            build_bad_workspace(name)
        };

        let rep1 = run_checks(&mut ws1, &f1, &CheckConfig::default());
        let rep2 = run_checks(&mut ws2, &f2, &CheckConfig::default());
        assert_eq!(rep1, rep2, "runs for fixture {name} must be deterministic");
    }
}

#[test]
fn self_corpus_requirements_produce_no_term_or_contradiction_errors_or_warnings() {
    let src = MemorySource::new()
        .with(".kiro/vocabulary.toml", SEED_VOCAB)
        .with(".kiro/steering/roadmap.md", SEED_ROADMAP)
        .with(".kiro/specs/cli-gate/spec.json", SELF_CLI_GATE_SPEC)
        .with(".kiro/specs/cli-gate/requirements.md", SELF_CLI_GATE_REQ)
        .with(".kiro/specs/kiro-core/spec.json", SELF_KIRO_CORE_SPEC)
        .with(".kiro/specs/kiro-core/requirements.md", SELF_KIRO_CORE_REQ)
        .with(".kiro/specs/plugin-core/spec.json", SELF_PLUGIN_CORE_SPEC)
        .with(
            ".kiro/specs/plugin-core/requirements.md",
            SELF_PLUGIN_CORE_REQ,
        )
        .with(
            ".kiro/specs/spec-constraints/spec.json",
            SELF_SPEC_CONSTRAINTS_SPEC,
        )
        .with(
            ".kiro/specs/spec-constraints/requirements.md",
            SELF_SPEC_CONSTRAINTS_REQ,
        )
        .with(
            ".kiro/specs/spec-navigation/spec.json",
            SELF_SPEC_NAVIGATION_SPEC,
        )
        .with(
            ".kiro/specs/spec-navigation/requirements.md",
            SELF_SPEC_NAVIGATION_REQ,
        )
        .with(
            ".kiro/specs/spec-visualization/spec.json",
            SELF_SPEC_VISUALIZATION_SPEC,
        )
        .with(
            ".kiro/specs/spec-visualization/requirements.md",
            SELF_SPEC_VISUALIZATION_REQ,
        )
        .with(
            ".kiro/specs/vocabulary-grounding/spec.json",
            SELF_VOCABULARY_GROUNDING_SPEC,
        )
        .with(
            ".kiro/specs/vocabulary-grounding/requirements.md",
            SELF_VOCABULARY_GROUNDING_REQ,
        );

    let specs = [
        "cli-gate",
        "kiro-core",
        "plugin-core",
        "spec-constraints",
        "spec-navigation",
        "spec-visualization",
        "vocabulary-grounding",
    ];

    let known_features: Vec<String> = specs.iter().map(|s| (*s).to_string()).collect();
    let mut ws = Workspace::new(src, known_features);

    let term_or_cont_finding_codes = [
        codes::TERM_UNDECLARED,
        codes::TERM_TOPIC_MISMATCH,
        codes::TERM_AMBIGUOUS,
        codes::TERM_DEPRECATED,
        codes::CHECK_UNSCOPED,
        codes::GUARD_UNPARSED,
        codes::GUARD_TERM_UNDECLARED,
        codes::GUARD_KIND_MISMATCH,
        codes::GUARD_UNSATISFIABLE,
        codes::CONFLICT_REF_UNKNOWN,
        codes::CONTRADICTION,
        codes::SEARCH_REFUSED,
    ];

    for s in specs {
        let rep = run_checks(&mut ws, s, &CheckConfig::default());

        for finding in &rep.findings {
            if finding.file.ends_with("requirements.md")
                && term_or_cont_finding_codes.contains(&finding.code)
            {
                assert!(
                    finding.severity != Severity::Error && finding.severity != Severity::Warning,
                    "Spec {s} produced unexpected {finding:?} on requirements.md"
                );
            }
        }
    }
}

// PRNG for cross-check and garbage testing
struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }

    fn next_bool(&mut self) -> bool {
        (self.next_u64() >> 32) & 1 == 1
    }
}

#[test]
fn generated_model_cross_check_bdd_vs_brute_force_typed_eval() {
    let mut rng = SimpleRng::new(0xCAFE_BABE_DEAD_BEEF);

    for model_idx in 0..64 {
        // Generate a small model with 2 terms, total <= 12 values
        let d1_vals = vec![
            Value::Enum("v1".to_string()),
            Value::Enum("v2".to_string()),
            Value::Enum("v3".to_string()),
        ];
        let d2_vals = vec![Value::Bool(true), Value::Bool(false)];
        let d3_vals = vec![Value::Enum("u1".to_string()), Value::Enum("u2".to_string())];

        let domains = vec![
            Domain {
                id: "t1".to_string(),
                topic: "top".to_string(),
                values: d1_vals.clone(),
            },
            Domain {
                id: "t2".to_string(),
                topic: "top".to_string(),
                values: d2_vals.clone(),
            },
            Domain {
                id: "t3".to_string(),
                topic: "top".to_string(),
                values: d3_vals.clone(),
            },
        ];

        // Total domain values = 3 + 2 + 2 = 7 <= 12
        let total_values: usize = domains.iter().map(|d| d.values.len()).sum();
        assert!(total_values <= 12);

        // Random expression
        let atom1 = Typed::Atom(Atom::Enum {
            id: "t1".to_string(),
            equal: rng.next_bool(),
            value: if rng.next_bool() {
                "v1".to_string()
            } else {
                "v2".to_string()
            },
        });
        let atom2 = Typed::Atom(Atom::Bool {
            id: "t2".to_string(),
            value: rng.next_bool(),
        });
        let atom3 = Typed::Atom(Atom::Enum {
            id: "t3".to_string(),
            equal: rng.next_bool(),
            value: "u1".to_string(),
        });

        let expr = match rng.next_usize(4) {
            0 => Typed::And(vec![atom1, atom2]),
            1 => Typed::Or(vec![atom1, atom3]),
            2 => Typed::And(vec![Typed::Not(Box::new(atom1)), atom2]),
            _ => Typed::Or(vec![Typed::And(vec![atom1, atom2]), atom3]),
        };

        // 1. Evaluate with BDD
        let enc = Encoding::new(&domains, 512, 10_000).unwrap();
        let bdd = enc.encode(&expr, &domains, 10_000, "cross_check").unwrap();
        let bdd_sat = enc.satisfiable(&bdd, 10_000, "cross_check").unwrap();

        // 2. Evaluate with brute-force Typed::eval over all combinations
        let mut brute_sat = false;
        for v1 in &d1_vals {
            for v2 in &d2_vals {
                for v3 in &d3_vals {
                    let mut state = BTreeMap::new();
                    state.insert("t1".to_string(), v1.clone());
                    state.insert("t2".to_string(), v2.clone());
                    state.insert("t3".to_string(), v3.clone());

                    if expr.eval(&state) {
                        brute_sat = true;
                        break;
                    }
                }
                if brute_sat {
                    break;
                }
            }
            if brute_sat {
                break;
            }
        }

        assert_eq!(
            bdd_sat, brute_sat,
            "Model {model_idx} mismatch between BDD ({bdd_sat}) and brute-force ({brute_sat})"
        );
    }
}

#[test]
fn garbage_no_panic_totality_tests() {
    let mut rng = SimpleRng::new(0x1234_5678_9ABC_DEF0);

    for _ in 0..256 {
        let len = rng.next_usize(128);
        let mut garbage = String::with_capacity(len);
        for _ in 0..len {
            let byte = (rng.next_u64() & 0xFF) as u8;
            // Mixed printable and control ASCII
            if byte.is_ascii() {
                garbage.push(byte as char);
            } else {
                garbage.push('?');
            }
        }

        // 1. Guard parser totality
        let _ = parse_guard(&garbage);
        let _ = parse_effect(&garbage);

        // 2. Document runner totality
        let doc = parse_requirements(&garbage, "garbage.md");
        let (ears_f, _) = ears_check(&doc, "garbage", false);
        let _ = ears_f;

        let loaded = vocab_parse(GOOD_VOCAB, VOCABULARY_PATH);
        let annotated = kiro_core::checks::annotations::collect(&doc);
        let topics: Vec<String> = Vec::new();
        let scope = resolve_scope(&annotated, &loaded.vocabulary, &topics);
        let fin = finitize(&doc, &annotated, &scope, "garbage", "garbage.md");
        let _ = contradiction_search(&fin, "garbage", "garbage.md", 512, 10_000);
    }
}

#[test]
#[cfg(not(debug_assertions))]
fn release_only_timing_criteria_and_search() {
    // 1. 1000 criteria EARS and terms check under 1 second
    let mut big_req = String::from(
        "# Requirements Document\n\n## Requirements\n\n### Requirement 1: Massive\n\n_Terms: mode_\n\n#### Acceptance Criteria\n\n",
    );
    for i in 1..=1000 {
        use std::fmt::Write;
        let _ = writeln!(
            big_req,
            "{i}. The controller shall process incoming telemetry event number {i}.\n   _Guard: mode = normal\n"
        );
    }

    let t0 = Instant::now();
    let doc = parse_requirements(&big_req, "massive.md");
    let (ears_findings, ears_sum) = ears_check(&doc, "massive", false);
    assert_eq!(ears_sum.criteria, 1000);
    assert_eq!(ears_findings.len(), 0);

    let loaded = vocab_parse(GOOD_VOCAB, VOCABULARY_PATH);
    let annotated = kiro_core::checks::annotations::collect(&doc);
    let topics = vec!["ops".to_string()];
    let scope = resolve_scope(&annotated, &loaded.vocabulary, &topics);
    let (terms_findings, terms_sum) = kiro_core::checks::terms::check(
        &annotated,
        &scope,
        &loaded.vocabulary,
        &topics,
        "massive",
        "massive.md",
        false,
    );
    assert_eq!(terms_sum.declared, 1);
    assert_eq!(terms_findings.len(), 0);

    let elapsed_ears_terms = t0.elapsed();
    assert!(
        elapsed_ears_terms < Duration::from_secs(1),
        "1000 criteria EARS and terms took {:?}",
        elapsed_ears_terms
    );

    // 2. 200 criteria contradiction search on 32 terms under 1 second
    // Generate 32 terms (16 enum terms with 4 values, 16 int terms)
    let mut domains = Vec::new();
    for i in 0..16 {
        domains.push(Domain {
            id: format!("term_enum_{i}"),
            topic: "top".to_string(),
            values: vec![
                Value::Enum("v1".to_string()),
                Value::Enum("v2".to_string()),
                Value::Enum("v3".to_string()),
                Value::Enum("v4".to_string()),
            ],
        });
    }
    for i in 0..16 {
        domains.push(Domain {
            id: format!("term_int_{i}"),
            topic: "top".to_string(),
            values: vec![
                Value::Region {
                    low: None,
                    high: Some(10),
                },
                Value::Region {
                    low: Some(11),
                    high: Some(20),
                },
                Value::Region {
                    low: Some(21),
                    high: None,
                },
            ],
        });
    }

    let mut criteria = Vec::new();
    for i in 0..200 {
        let term_id = format!("term_enum_{}", i % 16);
        criteria.push(kiro_core::checks::finitize::Prepared {
            id: format!("1.{i}"),
            line: i + 1,
            guard_line: Some(i + 1),
            guard: Typed::Atom(Atom::Enum {
                id: term_id,
                equal: true,
                value: "v1".to_string(),
            }),
            effects: vec![],
            conflicts: if i % 2 == 1 {
                vec![format!("1.{}", i - 1)]
            } else {
                vec![]
            },
        });
    }

    let fin = kiro_core::checks::finitize::Finitized {
        criteria,
        domains,
        findings: vec![],
        excluded: 0,
    };

    let t1 = Instant::now();
    let (findings, summary) = contradiction_search(&fin, "bench", "bench.md", 512, 10_000);
    let elapsed_cont = t1.elapsed();

    assert!(
        elapsed_cont < Duration::from_secs(1),
        "200 criteria contradiction search took {:?}",
        elapsed_cont
    );
    assert_eq!(summary.guarded, 200);
    assert_eq!(findings.len(), 100); // 100 pairs of conflicting criteria
}
