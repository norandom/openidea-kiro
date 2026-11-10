//! Ratchet decisions (code-constraints 5 and 11): only regression and new
//! breaches block. Every expected outcome below was decided by hand from
//! the rules in `ratchet.rs` and `evaluate.rs` before the first run.
//!
//! Built-in limits used: cyclomatic 10, nesting 3, code_lines 60,
//! statements 40, file code_lines 500.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_core::Severity;
use kiro_core::code::{
    Before, CodeConfig, CodeFacts, CodeReport, Completeness, CoverageCell, EntityKey,
    EvaluateInput, Fingerprint, MetricId, NetDelta, Observation, ProducerId, Scope, Trust,
    evaluate, pair, report,
};

/// A routine with its metric values and body hash.
struct Routine<'a> {
    path: &'a str,
    name: &'a str,
    parameters: &'a str,
    hash: &'a str,
    values: &'a [(&'a str, f64)],
}

const fn routine<'a>(path: &'a str, name: &'a str, values: &'a [(&'a str, f64)]) -> Routine<'a> {
    Routine {
        path,
        name,
        parameters: "(a)",
        hash: "h",
        values,
    }
}

fn producer() -> ProducerId {
    ProducerId {
        name: "test".to_string(),
        version: "0".to_string(),
        grammar: "none".to_string(),
    }
}

fn metric(name: &str) -> MetricId {
    MetricId {
        name: name.to_string(),
        definition_version: "1".to_string(),
    }
}

fn build(routines: &[Routine<'_>], files: &[(&str, f64)], failed: &[&str]) -> CodeFacts {
    let mut facts = CodeFacts::default();
    let mut paths: Vec<&str> = Vec::new();
    for entry in routines {
        let entity = EntityKey {
            scope: Scope::Routine,
            path: entry.path.to_string(),
            qualified_name: entry.name.to_string(),
            parameters: entry.parameters.to_string(),
            ordinal: 0,
        };
        for (name, value) in entry.values {
            facts.observations.push(Observation {
                entity: entity.clone(),
                metric: metric(name),
                value: *value,
                producer: producer(),
                line: 1,
            });
        }
        facts.fingerprints.push(Fingerprint {
            entity,
            hash: entry.hash.to_string(),
        });
        paths.push(entry.path);
    }
    for (path, lines) in files {
        facts.observations.push(Observation {
            entity: EntityKey {
                scope: Scope::File,
                path: (*path).to_string(),
                qualified_name: (*path).to_string(),
                parameters: String::new(),
                ordinal: 0,
            },
            metric: metric("code_lines"),
            value: *lines,
            producer: producer(),
            line: 1,
        });
        paths.push(path);
    }
    paths.extend(failed.iter().copied());
    paths.sort_unstable();
    paths.dedup();
    for path in paths {
        let is_failed = failed.contains(&path);
        facts.coverage.push(CoverageCell {
            path: path.to_string(),
            metric: None,
            state: if is_failed {
                Completeness::Failed
            } else {
                Completeness::Complete
            },
            reason: is_failed.then(|| "syntax error at line 1".to_string()),
            line: is_failed.then_some(1),
        });
    }
    facts.sort();
    facts
}

fn run(before: &CodeFacts, after: &CodeFacts, renames: &[(String, String)]) -> CodeReport {
    let mut selected: Vec<String> = after
        .coverage
        .iter()
        .map(|cell| cell.path.clone())
        .collect();
    selected.sort();
    let before_paths: Vec<String> = before
        .coverage
        .iter()
        .map(|cell| cell.path.clone())
        .collect();
    let config = CodeConfig::default();
    evaluate(&EvaluateInput {
        facts: after,
        config: &config,
        selected: &selected,
        specs: &[],
        before: Some(Before {
            facts: before,
            identity: "revision test",
            renames,
        }),
        before_paths: &before_paths,
    })
}

/// (code, severity, pre-existing) of the findings about `name`, sorted.
fn outcomes(report: &CodeReport, name: &str) -> Vec<(String, Severity, bool)> {
    let mut out: Vec<(String, Severity, bool)> = report
        .findings
        .iter()
        .filter(|entry| {
            entry
                .entity
                .as_ref()
                .is_some_and(|entity| entity.qualified_name == name)
        })
        .map(|entry| {
            (
                format!("{} {}", entry.finding.code, entry.metric.clone().unwrap()),
                entry.finding.severity,
                entry.pre_existing,
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The first finding about the entity `name`.
fn about<'r>(report: &'r CodeReport, name: &str) -> &'r kiro_core::code::CodeFinding {
    report
        .findings
        .iter()
        .find(|entry| {
            entry
                .entity
                .as_ref()
                .is_some_and(|entity| entity.qualified_name == name)
        })
        .unwrap()
}

fn blocking(report: &CodeReport) -> usize {
    report
        .blocking_findings()
        .filter(|finding| finding.severity != Severity::Info)
        .count()
}

#[test]
fn debt_that_did_not_worsen_is_pre_existing_and_does_not_block() {
    let before = build(&[routine("a.rs", "old", &[("cyclomatic", 15.0)])], &[], &[]);
    // Unchanged.
    let same = run(&before, &before, &[]);
    assert_eq!(
        outcomes(&same, "old"),
        [(
            "CODE_ROUTINE_LIMIT cyclomatic".to_string(),
            Severity::Warning,
            true
        )]
    );
    assert_eq!(blocking(&same), 0);
    assert!(
        about(&same, "old")
            .finding
            .message
            .contains("pre-existing, was 15 before")
    );
    assert_eq!(about(&same, "old").before, Some(15.0));
    assert_eq!(about(&same, "old").trust, Some("exact"));
    assert!(same.changes.is_empty());

    // Improved but still above the limit: still pre-existing, and recorded
    // as an improvement.
    let better = build(&[routine("a.rs", "old", &[("cyclomatic", 12.0)])], &[], &[]);
    let improved = run(&before, &better, &[]);
    assert_eq!(outcomes(&improved, "old")[0].2, true);
    assert_eq!(improved.changes.len(), 1);
    assert!(improved.changes[0].improved());
}

#[test]
fn a_value_that_rises_above_or_further_above_the_limit_is_an_error() {
    let before = build(
        &[
            routine("a.rs", "worse", &[("cyclomatic", 15.0)]),
            routine("a.rs", "crossing", &[("cyclomatic", 9.0)]),
        ],
        &[],
        &[],
    );
    let after = build(
        &[
            routine("a.rs", "worse", &[("cyclomatic", 18.0)]),
            routine("a.rs", "crossing", &[("cyclomatic", 11.0)]),
        ],
        &[],
        &[],
    );
    let result = run(&before, &after, &[]);
    for name in ["worse", "crossing"] {
        assert_eq!(
            outcomes(&result, name),
            [(
                "CODE_RATCHET cyclomatic".to_string(),
                Severity::Error,
                false
            )],
            "{name}"
        );
    }
    assert_eq!(blocking(&result), 2);
    let message = &about(&result, "worse").finding.message;
    assert!(message.contains("rose from"), "{message}");
}

#[test]
fn growth_within_the_limit_is_a_warning() {
    let before = build(&[routine("a.rs", "grow", &[("cyclomatic", 5.0)])], &[], &[]);
    let after = build(&[routine("a.rs", "grow", &[("cyclomatic", 7.0)])], &[], &[]);
    let result = run(&before, &after, &[]);
    assert_eq!(
        outcomes(&result, "grow"),
        [(
            "CODE_RATCHET cyclomatic".to_string(),
            Severity::Warning,
            false
        )]
    );
    assert!(
        about(&result, "grow")
            .finding
            .message
            .contains("rose from 5 to 7, within the limit 10")
    );
}

#[test]
fn a_new_routine_above_the_limit_is_judged_on_the_limit_alone() {
    let before = build(
        &[routine("a.rs", "other", &[("cyclomatic", 1.0)])],
        &[],
        &[],
    );
    let after = build(
        &[
            routine("a.rs", "other", &[("cyclomatic", 1.0)]),
            Routine {
                hash: "new-body",
                ..routine("a.rs", "fresh", &[("cyclomatic", 14.0)])
            },
        ],
        &[],
        &[],
    );
    let result = run(&before, &after, &[]);
    assert_eq!(
        outcomes(&result, "fresh"),
        [(
            "CODE_ROUTINE_LIMIT cyclomatic".to_string(),
            Severity::Warning,
            false
        )]
    );
    assert_eq!(about(&result, "fresh").before, None);
}

#[test]
fn a_changed_signature_can_show_regression_but_never_excuses_debt() {
    let before = build(
        &[
            routine("a.rs", "same_value", &[("cyclomatic", 15.0)]),
            routine("b.rs", "got_worse", &[("cyclomatic", 15.0)]),
        ],
        &[],
        &[],
    );
    let changed =
        |path: &'static str, name: &'static str, value: &'static [(&'static str, f64)]| Routine {
            parameters: "(a, b)",
            hash: "other",
            ..routine(path, name, value)
        };
    let after = build(
        &[
            changed("a.rs", "same_value", &[("cyclomatic", 15.0)]),
            changed("b.rs", "got_worse", &[("cyclomatic", 18.0)]),
        ],
        &[],
        &[],
    );
    let result = run(&before, &after, &[]);
    // Paired by signature heuristic: the before value is known, but an
    // unchanged breach is not pre-existing.
    assert_eq!(
        outcomes(&result, "same_value"),
        [(
            "CODE_ROUTINE_LIMIT cyclomatic".to_string(),
            Severity::Warning,
            false
        )]
    );
    assert_eq!(
        outcomes(&result, "got_worse"),
        [(
            "CODE_RATCHET cyclomatic".to_string(),
            Severity::Error,
            false
        )]
    );
    let finding = result
        .findings
        .iter()
        .find(|entry| entry.finding.file == "a.rs" && entry.entity.is_some())
        .unwrap();
    assert_eq!(finding.trust, Some("signature"));
    assert_eq!(finding.before, Some(15.0));
}

#[test]
fn ambiguous_signature_changes_stay_unpaired() {
    // Two overloads disappeared and two appeared under one name.
    let old = |parameters: &'static str| Routine {
        parameters,
        hash: "x",
        ..routine("a.rs", "f", &[("cyclomatic", 15.0)])
    };
    let before = build(&[old("(a)"), old("(a, b)")], &[], &[]);
    let after = build(&[old("(c)"), old("(c, d)")], &[], &[]);
    let pairing = pair(
        &Before {
            facts: &before,
            identity: "t",
            renames: &[],
        },
        &after,
    );
    assert!(pairing.is_empty());
    let result = run(&before, &after, &[]);
    assert!(
        outcomes(&result, "f")
            .iter()
            .all(|(code, _, pre_existing)| code.starts_with("CODE_ROUTINE_LIMIT") && !pre_existing)
    );
}

#[test]
fn a_routine_moved_to_another_file_keeps_its_debt_status() {
    // Splitting a god file: `big` moves from a.rs to b.rs unchanged.
    let before = build(
        &[Routine {
            hash: "body-1",
            ..routine("a.rs", "big", &[("cyclomatic", 15.0)])
        }],
        &[],
        &[],
    );
    let moved = build(
        &[Routine {
            hash: "body-1",
            ..routine("b.rs", "big", &[("cyclomatic", 15.0)])
        }],
        &[("a.rs", 10.0)],
        &[],
    );
    let result = run(&before, &moved, &[]);
    assert_eq!(
        outcomes(&result, "big"),
        [(
            "CODE_ROUTINE_LIMIT cyclomatic".to_string(),
            Severity::Warning,
            true
        )]
    );
    assert_eq!(
        result
            .findings
            .iter()
            .find(|e| e.pre_existing)
            .unwrap()
            .trust,
        Some("moved")
    );
    assert_eq!(blocking(&result), 0);

    // The same move with a changed body is a new routine.
    let rewritten = build(
        &[Routine {
            hash: "body-2",
            ..routine("b.rs", "big", &[("cyclomatic", 15.0)])
        }],
        &[("a.rs", 10.0)],
        &[],
    );
    let result = run(&before, &rewritten, &[]);
    assert_eq!(outcomes(&result, "big")[0].2, false);

    // Two identical copies appear: the move is ambiguous, nothing is excused.
    let copied = build(
        &[
            Routine {
                hash: "body-1",
                ..routine("b.rs", "big", &[("cyclomatic", 15.0)])
            },
            Routine {
                hash: "body-1",
                ..routine("c.rs", "big", &[("cyclomatic", 15.0)])
            },
        ],
        &[("a.rs", 10.0)],
        &[],
    );
    let result = run(&before, &copied, &[]);
    assert!(outcomes(&result, "big").iter().all(|(_, _, pre)| !pre));
}

#[test]
fn a_renamed_file_is_compared_through_the_rename() {
    let before = build(
        &[routine("old/name.rs", "f", &[("cyclomatic", 15.0)])],
        &[("old/name.rs", 600.0)],
        &[],
    );
    let after = build(
        &[Routine {
            hash: "edited",
            ..routine("new/name.rs", "f", &[("cyclomatic", 15.0)])
        }],
        &[("new/name.rs", 600.0)],
        &[],
    );
    let renames = [("old/name.rs".to_string(), "new/name.rs".to_string())];
    let result = run(&before, &after, &renames);
    assert_eq!(
        outcomes(&result, "f")[0].2,
        true,
        "exact through the rename"
    );
    // The file's own breach is pre-existing too.
    let file = result
        .findings
        .iter()
        .find(|entry| entry.finding.code == "CODE_FILE_LIMIT")
        .unwrap();
    assert!(file.pre_existing);
    assert_eq!(file.trust, Some("exact"));

    // Without the rename information the routine is recognized as moved
    // only when its body is unchanged; here it was edited.
    let result = run(&before, &after, &[]);
    assert_eq!(outcomes(&result, "f")[0].2, false);
}

#[test]
fn size_growth_is_excused_only_when_control_flow_got_simpler() {
    let before = build(
        &[
            routine(
                "a.rs",
                "guarded",
                &[
                    ("cyclomatic", 6.0),
                    ("nesting", 3.0),
                    ("code_lines", 20.0),
                    ("statements", 10.0),
                ],
            ),
            routine(
                "a.rs",
                "bloated",
                &[
                    ("cyclomatic", 6.0),
                    ("nesting", 3.0),
                    ("code_lines", 20.0),
                    ("statements", 10.0),
                ],
            ),
            routine(
                "a.rs",
                "mixed",
                &[
                    ("cyclomatic", 6.0),
                    ("nesting", 3.0),
                    ("code_lines", 20.0),
                    ("statements", 10.0),
                ],
            ),
        ],
        &[],
        &[],
    );
    let after = build(
        &[
            // Guard clauses: more lines and statements, less nesting.
            routine(
                "a.rs",
                "guarded",
                &[
                    ("cyclomatic", 6.0),
                    ("nesting", 2.0),
                    ("code_lines", 24.0),
                    ("statements", 12.0),
                ],
            ),
            // Just longer.
            routine(
                "a.rs",
                "bloated",
                &[
                    ("cyclomatic", 6.0),
                    ("nesting", 3.0),
                    ("code_lines", 24.0),
                    ("statements", 12.0),
                ],
            ),
            // Less nesting but more branches: nothing is excused.
            routine(
                "a.rs",
                "mixed",
                &[
                    ("cyclomatic", 7.0),
                    ("nesting", 2.0),
                    ("code_lines", 24.0),
                    ("statements", 12.0),
                ],
            ),
        ],
        &[],
        &[],
    );
    let result = run(&before, &after, &[]);
    assert!(
        outcomes(&result, "guarded").is_empty(),
        "{:?}",
        outcomes(&result, "guarded")
    );
    let codes =
        |name: &str| -> Vec<String> { outcomes(&result, name).into_iter().map(|o| o.0).collect() };
    assert_eq!(
        codes("bloated"),
        ["CODE_RATCHET code_lines", "CODE_RATCHET statements"]
    );
    assert_eq!(
        codes("mixed"),
        [
            "CODE_RATCHET code_lines",
            "CODE_RATCHET cyclomatic",
            "CODE_RATCHET statements"
        ]
    );
    // Every change is still recorded for the summary.
    assert_eq!(result.changes.len(), 3 + 2 + 4);
}

#[test]
fn a_file_that_failed_to_parse_before_gives_no_comparison() {
    let before = build(&[], &[], &["a.rs"]);
    let after = build(&[routine("a.rs", "f", &[("cyclomatic", 15.0)])], &[], &[]);
    let result = run(&before, &after, &[]);
    assert_eq!(
        outcomes(&result, "f"),
        [(
            "CODE_ROUTINE_LIMIT cyclomatic".to_string(),
            Severity::Warning,
            false
        )]
    );
    assert!(matches!(result.net, NetDelta::Unavailable(_)));
}

#[test]
fn container_counts_are_never_ratcheted() {
    let before = build(&[], &[("grow.rs", 400.0), ("big.rs", 600.0)], &[]);
    let after = build(&[], &[("grow.rs", 450.0), ("big.rs", 600.0)], &[]);
    let result = run(&before, &after, &[]);
    // Growth within the file limit: no finding at all.
    assert!(
        !result
            .findings
            .iter()
            .any(|entry| entry.finding.file == "grow.rs")
    );
    // An unchanged breach of a file is pre-existing.
    let big = result
        .findings
        .iter()
        .find(|entry| entry.finding.file == "big.rs" && entry.entity.is_some())
        .unwrap();
    assert_eq!(big.finding.code, "CODE_FILE_LIMIT");
    assert!(big.pre_existing);
    assert!(
        result.changes.is_empty(),
        "container changes are not listed"
    );
}

#[test]
fn net_growth_counts_additions_and_deletions_over_complete_sides() {
    let before = build(
        &[
            routine(
                "a.rs",
                "kept",
                &[("statements", 10.0), ("code_lines", 20.0)],
            ),
            routine(
                "gone.rs",
                "deleted",
                &[("statements", 7.0), ("code_lines", 9.0)],
            ),
        ],
        &[],
        &[],
    );
    let after = build(
        &[
            routine(
                "a.rs",
                "kept",
                &[("statements", 12.0), ("code_lines", 25.0)],
            ),
            Routine {
                hash: "n",
                ..routine("a.rs", "added", &[("statements", 3.0), ("code_lines", 4.0)])
            },
        ],
        &[],
        &[],
    );
    let result = run(&before, &after, &[]);
    // statements: (12 + 3) - (10 + 7) = -2; lines: (25 + 4) - (20 + 9) = 0.
    assert_eq!(
        result.net,
        NetDelta::Known {
            statements: -2.0,
            code_lines: 0.0
        }
    );
    assert_eq!(result.net.label(), "-2 statements, 0 lines");
    let coverage = result.findings.last().unwrap();
    assert!(
        coverage
            .finding
            .message
            .contains("before side: revision test")
    );
    assert!(
        coverage
            .finding
            .message
            .contains("net: -2 statements, 0 lines")
    );
}

#[test]
fn trust_levels_say_what_they_may_excuse() {
    assert!(Trust::Exact.excuses_debt());
    assert!(Trust::Moved.excuses_debt());
    assert!(!Trust::Signature.excuses_debt());
}

#[test]
fn the_summary_is_short_ascii_in_a_fixed_order() {
    let many_before: Vec<Routine<'_>> = Vec::new();
    let _ = many_before;
    let names: Vec<String> = (0..30).map(|index| format!("r{index:02}")).collect();
    let values_before = [("cyclomatic", 5.0)];
    let values_worse = [("cyclomatic", 7.0)];
    let values_better = [("cyclomatic", 3.0)];
    let before_routines: Vec<Routine<'_>> = names
        .iter()
        .map(|name| routine("a.rs", name, &values_before))
        .collect();
    let after_routines: Vec<Routine<'_>> = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            routine(
                "a.rs",
                name,
                if index % 2 == 0 {
                    &values_worse
                } else {
                    &values_better
                },
            )
        })
        .collect();
    let before = build(&before_routines, &[], &[]);
    let after = build(&after_routines, &[], &[]);
    let result = run(&before, &after, &[]);
    let text = report::summary(&result, "FAIL", "warning", 15);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.len() <= report::SUMMARY_MAX_LINES,
        "{} lines",
        lines.len()
    );
    assert_eq!(lines[0], "kiro code check: FAIL (fail-on warning)");
    assert_eq!(lines[1], "before side: revision test");
    assert_eq!(
        lines[2],
        "findings: 15 blocking, 15 warnings, 0 pre-existing"
    );
    assert_eq!(lines[3], "worsened: 15");
    assert_eq!(lines[4], "  a.rs:1 r00 cyclomatic 5 -> 7 (limit 10)");
    assert_eq!(lines[12], "  ... and 7 more");
    assert_eq!(lines[13], "improved: 15");
    assert_eq!(lines[14], "  a.rs:1 r01 cyclomatic 5 -> 3 (limit 10)");
    assert_eq!(
        *lines.last().unwrap(),
        "net: unavailable (no statement values)"
            .replace("unavailable (no statement values)", &result.net.label())
    );
    assert!(text.is_ascii());
    assert!(
        text.chars()
            .all(|c| c == '\n' || c.is_ascii_graphic() || c == ' ')
    );
    assert!(text.ends_with('\n'));

    // A name outside ASCII cannot break the summary.
    let odd = build(
        &[routine("a.rs", "größe", &[("cyclomatic", 5.0)])],
        &[],
        &[],
    );
    let odd_after = build(
        &[routine("a.rs", "größe", &[("cyclomatic", 6.0)])],
        &[],
        &[],
    );
    let text = report::summary(&run(&odd, &odd_after, &[]), "PASS", "error", 0);
    assert!(text.is_ascii(), "{text}");
    assert!(text.contains("gr??e cyclomatic 5 -> 6"));
}
