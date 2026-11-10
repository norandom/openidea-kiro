//! Pure policy of the code gate (code-constraints 2 to 7, 11): facts,
//! limits, ownership and evaluation, without any native measurement.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use kiro_core::Severity;
use kiro_core::code::{
    CodeConfig, CodeFacts, Completeness, CoverageCell, EntityKey, EvaluateInput, MetricId,
    Observation, ProducerId, Scope, SelectionError, Tag, attribute, evaluate, highest, judge,
    matches_any, report, spec_paths, task_paths,
};
use kiro_core::source::Feature;

fn routine(path: &str, name: &str) -> EntityKey {
    EntityKey {
        scope: Scope::Routine,
        path: path.to_string(),
        qualified_name: name.to_string(),
        parameters: "()".to_string(),
        ordinal: 0,
    }
}

fn file(path: &str) -> EntityKey {
    EntityKey {
        scope: Scope::File,
        path: path.to_string(),
        qualified_name: path.to_string(),
        parameters: String::new(),
        ordinal: 0,
    }
}

fn observe(entity: &EntityKey, metric: &str, value: f64) -> Observation {
    Observation {
        entity: entity.clone(),
        metric: MetricId {
            name: metric.to_string(),
            definition_version: "1".to_string(),
        },
        value,
        producer: ProducerId {
            name: "test".to_string(),
            version: "0".to_string(),
            grammar: "none".to_string(),
        },
        line: 7,
    }
}

fn complete(path: &str) -> CoverageCell {
    CoverageCell {
        path: path.to_string(),
        metric: None,
        state: Completeness::Complete,
        reason: None,
        line: None,
    }
}

fn facts(observations: Vec<Observation>, coverage: Vec<CoverageCell>) -> CodeFacts {
    let mut facts = CodeFacts {
        observations,
        coverage,
        ..CodeFacts::default()
    };
    facts.sort();
    facts
}

fn features() -> Vec<Feature> {
    let mut workspace = common::minimal_workspace();
    vec![
        workspace.load("demo").clone(),
        workspace.load("old").clone(),
    ]
}

#[test]
fn a_value_equal_to_its_limit_passes_and_one_above_it_breaches() {
    let config = CodeConfig::default();
    let at = routine("src/demo.rs", "at_limit");
    let over = routine("src/demo.rs", "over_limit");
    let facts = facts(
        vec![
            observe(&at, "cyclomatic", 10.0),
            observe(&at, "nesting", 3.0),
            observe(&at, "parameters", 5.0),
            observe(&over, "cyclomatic", 11.0),
            observe(&over, "parameters", 6.0),
        ],
        vec![complete("src/demo.rs")],
    );
    let judged = judge(&facts, &config, &|_| true);
    let names: Vec<(&str, &str)> = judged
        .iter()
        .map(|j| (j.entity.qualified_name.as_str(), j.metric.as_str()))
        .collect();
    assert_eq!(
        names,
        [("over_limit", "cyclomatic"), ("over_limit", "parameters")]
    );
    assert!(
        judged.iter().all(|j| j.severity == Severity::Warning),
        "uncalibrated"
    );
    assert_eq!(judged[0].limit, 10.0);
    assert_eq!(judged[0].line, 7);
}

#[test]
fn lines_per_statement_is_judged_only_at_five_statements_or_more() {
    let config = CodeConfig::default();
    let small = routine("src/demo.rs", "small");
    let large = routine("src/demo.rs", "large");
    let dense = routine("src/demo.rs", "dense");
    let no_statements = routine("src/demo.rs", "no_statements");
    let facts = facts(
        vec![
            // 4 statements on 40 lines: below the floor, not judged.
            observe(&small, "code_lines", 40.0),
            observe(&small, "statements", 4.0),
            // 5 statements on 21 lines: 4.2 > 4.
            observe(&large, "code_lines", 21.0),
            observe(&large, "statements", 5.0),
            // 5 statements on 20 lines: exactly 4, passes.
            observe(&dense, "code_lines", 20.0),
            observe(&dense, "statements", 5.0),
            // Missing statements: not judged, never divided by zero.
            observe(&no_statements, "code_lines", 50.0),
        ],
        vec![complete("src/demo.rs")],
    );
    let judged = judge(&facts, &config, &|_| true);
    assert_eq!(judged.len(), 1);
    assert_eq!(judged[0].entity.qualified_name, "large");
    assert_eq!(judged[0].metric, "lines_per_statement");
    assert!((judged[0].value - 4.2).abs() < 1e-9);
}

#[test]
fn only_selected_files_are_judged() {
    let config = CodeConfig::default();
    let a = routine("src/a.rs", "f");
    let b = routine("src/b.rs", "f");
    let facts = facts(
        vec![observe(&a, "nesting", 9.0), observe(&b, "nesting", 9.0)],
        vec![complete("src/a.rs"), complete("src/b.rs")],
    );
    let judged = judge(&facts, &config, &|path| path == "src/b.rs");
    assert_eq!(judged.len(), 1);
    assert_eq!(judged[0].entity.path, "src/b.rs");
}

#[test]
fn container_limits_are_judged_for_files_and_types() {
    let config = CodeConfig::default();
    let big = file("src/demo.rs");
    let holder = EntityKey {
        scope: Scope::Type,
        qualified_name: "Holder".to_string(),
        ..file("src/demo.rs")
    };
    let facts = facts(
        vec![
            observe(&big, "code_lines", 501.0),
            observe(&big, "functions", 25.0),
            observe(&big, "types", 9.0),
            observe(&holder, "methods", 21.0),
            observe(&holder, "fields", 10.0),
        ],
        vec![complete("src/demo.rs")],
    );
    let metrics: Vec<String> = judge(&facts, &config, &|_| true)
        .into_iter()
        .map(|j| format!("{} {}", j.entity.scope.label(), j.metric))
        .collect();
    assert_eq!(metrics, ["type methods", "file code_lines", "file types"]);
}

#[test]
fn highest_returns_one_observation_per_scope_and_metric() {
    let a = routine("src/a.rs", "f");
    let b = routine("src/b.rs", "g");
    let facts = facts(
        vec![
            observe(&a, "nesting", 2.0),
            observe(&b, "nesting", 5.0),
            observe(&a, "cyclomatic", 4.0),
        ],
        vec![],
    );
    let top = highest(&facts, &|_| true);
    assert_eq!(top.len(), 2);
    let nesting = top.iter().find(|o| o.metric.name == "nesting").unwrap();
    assert_eq!(nesting.value, 5.0);
    assert_eq!(nesting.entity.path, "src/b.rs");
}

#[test]
fn invalid_facts_are_named() {
    let entity = routine("src/a.rs", "f");
    let mut bad_path = routine("../outside.rs", "g");
    bad_path.ordinal = 1;
    let facts = facts(
        vec![
            observe(&entity, "nesting", f64::NAN),
            observe(&entity, "cyclomatic", 1.0),
            observe(&entity, "cyclomatic", 2.0),
            observe(&bad_path, "nesting", 1.0),
        ],
        vec![],
    );
    let problems = facts.problems().join(" | ");
    assert!(problems.contains("value is not finite"), "{problems}");
    assert!(problems.contains("two different values"), "{problems}");
    assert!(
        problems.contains("invalid path `../outside.rs`"),
        "{problems}"
    );
    for path in ["/abs.rs", "a\\b.rs", "C:/x.rs", "a//b.rs", "./a.rs", ""] {
        assert!(!kiro_core::code::facts::is_valid_path(path), "{path}");
    }
    assert!(kiro_core::code::facts::is_valid_path("crates/a/src/lib.rs"));
}

#[test]
fn ownership_comes_from_the_file_structure_plan_and_task_boundaries() {
    let features = features();
    let specs: Vec<&Feature> = features.iter().collect();
    let owned = attribute("src/demo.rs", &specs);
    assert_eq!(owned.feature.as_deref(), Some("demo"));
    assert_eq!(owned.components, ["Core"]);
    assert_eq!(owned.tasks, ["1.1", "2"]);
    assert!(owned.others.is_empty());

    let unowned = attribute("src/elsewhere.rs", &specs);
    assert_eq!(unowned.feature, None);

    // A second spec mapping the same file is listed, not primary.
    let mut twin = features[0].clone();
    twin.name = "twin".to_string();
    let both = [&features[0], &twin];
    let shared = attribute("src/demo.rs", &both);
    assert_eq!(shared.feature.as_deref(), Some("demo"));
    assert_eq!(shared.others, ["twin"]);
    let reversed = [&twin, &features[0]];
    assert_eq!(
        attribute("src/demo.rs", &reversed).feature.as_deref(),
        Some("twin")
    );
}

#[test]
fn spec_and_task_selection_resolve_or_name_the_choices() {
    let features = features();
    let specs: Vec<&Feature> = features.iter().collect();
    assert_eq!(
        spec_paths("demo", &specs).unwrap(),
        ["src/demo.rs", "src/ui.rs"]
    );
    assert_eq!(task_paths("demo", "1.2", &specs).unwrap(), ["src/ui.rs"]);
    assert_eq!(task_paths("demo", "1.1", &specs).unwrap(), ["src/demo.rs"]);
    assert!(matches_any(&["src/demo.rs".to_string()], "src/demo.rs"));
    assert!(!matches_any(&["src/demo.rs".to_string()], "src/other.rs"));

    match spec_paths("nope", &specs).unwrap_err() {
        SelectionError::UnknownSpec { available, .. } => assert_eq!(available, ["demo", "old"]),
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        spec_paths("old", &specs).unwrap_err(),
        SelectionError::NoDesign { .. }
    ));
    match task_paths("demo", "9.9", &specs).unwrap_err() {
        SelectionError::UnknownTask { available, .. } => {
            assert_eq!(available, ["1", "1.1", "1.2", "2"]);
        }
        other => panic!("{other:?}"),
    }
    // Task 1 is a major without a boundary of its own.
    let error = task_paths("demo", "1", &specs).unwrap_err();
    assert!(
        matches!(error, SelectionError::NoBoundary { .. }),
        "{error}"
    );
    assert!(!error.to_string().contains('\n'));
}

#[test]
fn evaluation_orders_findings_and_always_reports_coverage() {
    let features = features();
    let specs: Vec<&Feature> = features.iter().collect();
    let owned = routine("src/demo.rs", "deep");
    let stray = routine("tools/x.rs", "deep");
    let facts = facts(
        vec![
            observe(&owned, "nesting", 4.0),
            observe(&stray, "nesting", 5.0),
        ],
        vec![
            complete("src/demo.rs"),
            complete("tools/x.rs"),
            CoverageCell {
                path: "src/ui.rs".to_string(),
                metric: None,
                state: Completeness::Failed,
                reason: Some("syntax error at line 3".to_string()),
                line: Some(3),
            },
            CoverageCell {
                path: "notes.txt".to_string(),
                metric: None,
                state: Completeness::Unavailable,
                reason: None,
                line: None,
            },
        ],
    );
    let selected: Vec<String> = ["notes.txt", "src/demo.rs", "src/ui.rs", "tools/x.rs"]
        .map(String::from)
        .to_vec();
    let config = CodeConfig::default();
    let result = evaluate(&EvaluateInput {
        facts: &facts,
        config: &config,
        selected: &selected,
        specs: &specs,
    });

    let codes: Vec<(&str, &str)> = result
        .findings
        .iter()
        .map(|entry| (entry.finding.file.as_str(), entry.finding.code))
        .collect();
    assert_eq!(
        codes,
        [
            ("src/demo.rs", "CODE_ROUTINE_LIMIT"),
            ("src/ui.rs", "CODE_PARSE_FAILED"),
            ("tools/x.rs", "CODE_FILE_UNOWNED"),
            ("tools/x.rs", "CODE_ROUTINE_LIMIT"),
            (".kiro/code-rules.toml", "CODE_COVERAGE"),
        ]
    );
    let limit = &result.findings[0];
    assert_eq!(limit.tag, Tag::Simplify);
    assert_eq!(limit.finding.feature.as_deref(), Some("demo"));
    assert_eq!(limit.finding.line, Some(7));
    assert_eq!(limit.value, Some(4.0));
    let parse = &result.findings[1];
    assert_eq!(parse.finding.severity, Severity::Error);
    assert_eq!(parse.tag, Tag::Fix);
    assert_eq!(parse.finding.line, Some(3));
    assert_eq!(parse.finding.feature.as_deref(), Some("demo"));

    assert_eq!(result.coverage.selected, 4);
    assert_eq!(result.coverage.measured, 2);
    assert_eq!(result.coverage.failed, ["src/ui.rs"]);
    assert_eq!(result.coverage.unmeasured, ["notes.txt"]);
    assert_eq!(result.coverage.unowned, 2);
    let summary: Vec<(Option<&str>, usize)> = result
        .specs
        .iter()
        .map(|s| (s.spec.as_deref(), s.files))
        .collect();
    assert_eq!(summary, [(Some("demo"), 2), (None, 2)]);
    assert_eq!(result.counts(), (1, 2, 2));

    // Two renderings of the same report are byte-identical.
    assert_eq!(report::text(&result), report::text(&result));
    assert_eq!(report::json(&result), report::json(&result));
    let json: serde_json::Value = serde_json::from_str(&report::json(&result)).unwrap();
    assert_eq!(json["findings"].as_array().unwrap().len(), 5);
}

#[test]
fn an_empty_selection_still_reports_what_was_not_measured() {
    let config = CodeConfig::default();
    let result = evaluate(&EvaluateInput {
        facts: &CodeFacts::default(),
        config: &config,
        selected: &[],
        specs: &[],
    });
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].finding.code, "CODE_COVERAGE");
    assert!(
        result.findings[0]
            .finding
            .message
            .starts_with("0 files selected")
    );
}

#[test]
fn every_hint_opens_with_exactly_one_known_tag() {
    for tag in [
        Tag::Simplify,
        Tag::Split,
        Tag::Delete,
        Tag::Shrink,
        Tag::Collect,
        Tag::Name,
        Tag::Fix,
    ] {
        assert!(!tag.label().is_empty());
        assert!(tag.label().chars().all(|c| c.is_ascii_lowercase()));
    }
}
