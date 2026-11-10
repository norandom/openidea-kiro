//! Configuration of the code gate (code-constraints requirement 2).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use kiro_core::Severity;
use kiro_core::code::{
    CodeConfig, CodeFacts, Completeness, CoverageCell, EntityKey, EvaluateInput, LimitSource,
    MetricId, Observation, ProducerId, Scope, evaluate, parse_config,
};
use kiro_core::source::Feature;

fn limit_of(
    config: &CodeConfig,
    scope: Scope,
    metric: &str,
    path: &str,
) -> Option<(f64, Severity, String)> {
    config
        .limit_for(scope, metric, path)
        .map(|(limit, source)| (limit.maximum, limit.severity, source.label()))
}

#[test]
fn an_absent_file_gives_the_built_in_limits() {
    let config = parse_config(None).unwrap();
    assert_eq!(config, CodeConfig::default());
    assert_eq!(
        limit_of(&config, Scope::Routine, "cyclomatic", "src/a.rs"),
        Some((10.0, Severity::Warning, "default".to_string()))
    );
    assert_eq!(config.digest(), CodeConfig::default().digest());
    assert_eq!(config.digest().len(), 16);
    assert!(
        config
            .limit_for(Scope::Routine, "unknown", "src/a.rs")
            .is_none()
    );
}

const FULL: &str = r#"
[limits.routine.cyclomatic]
max = 8

[limits.routine.nesting]
severity = "error"

[limits.routine.parameters]
max = 7
reason = "builders take many arguments"

[limits.file.types]
severity = "off"
reason = "type-heavy model modules"

[[scopes]]
paths = ["src/gen/"]
reason = "generated code"
[scopes.limits.routine.cyclomatic]
max = 50

[[scopes]]
paths = ["src/gen/hand/", "src/one.rs"]
reason = "hand-written part"
[scopes.limits.routine.cyclomatic]
max = 12

[[ignores]]
paths = ["fixtures/"]
reason = "measured by tests, not judged"
"#;

#[test]
fn file_limits_override_defaults_and_scopes_override_the_file() {
    let config = parse_config(Some(FULL)).unwrap();
    // Tighter than the default: no reason needed.
    assert_eq!(
        limit_of(&config, Scope::Routine, "cyclomatic", "src/a.rs"),
        Some((8.0, Severity::Warning, "file".to_string()))
    );
    assert_eq!(
        limit_of(&config, Scope::Routine, "nesting", "src/a.rs"),
        Some((3.0, Severity::Error, "file".to_string()))
    );
    assert_eq!(
        limit_of(&config, Scope::Routine, "parameters", "src/a.rs")
            .unwrap()
            .0,
        7.0
    );
    // Off: the metric is not judged at all.
    assert!(config.limit_for(Scope::File, "types", "src/a.rs").is_none());
    // The most specific scope wins; a directory covers only whole segments.
    assert_eq!(
        limit_of(&config, Scope::Routine, "cyclomatic", "src/gen/x.rs"),
        Some((50.0, Severity::Warning, "scope src/gen/".to_string()))
    );
    let cyclomatic = |path: &str| {
        limit_of(&config, Scope::Routine, "cyclomatic", path)
            .unwrap()
            .0
    };
    assert_eq!(cyclomatic("src/gen/hand/x.rs"), 12.0);
    assert_eq!(cyclomatic("src/one.rs"), 12.0);
    assert_eq!(
        cyclomatic("src/generic.rs"),
        8.0,
        "src/gen/ does not cover src/generic.rs"
    );
    // A metric the scope does not mention falls through to the file.
    assert_eq!(
        limit_of(&config, Scope::Routine, "nesting", "src/gen/x.rs")
            .unwrap()
            .1,
        Severity::Error
    );
    assert!(config.ignored("fixtures/a/b.rs").is_some());
    assert!(config.ignored("src/fixtures.rs").is_none());
    assert_ne!(config.digest(), CodeConfig::default().digest());
    assert!(matches!(
        config
            .limit_for(Scope::Routine, "cyclomatic", "src/gen/x.rs")
            .unwrap()
            .1,
        LimitSource::Scope(_)
    ));
}

#[test]
fn the_effective_configuration_names_origin_and_reason() {
    let effective = parse_config(Some(FULL)).unwrap().effective_text();
    for line in [
        "routine cyclomatic: max 8, warning (file)\n",
        "routine parameters: max 7, warning (file) reason: builders take many arguments\n",
        "file types: max 8, off (file) reason: type-heavy model modules\n",
        "routine code_lines: max 60, warning (default)\n",
        "scope src/gen/: routine cyclomatic: max 50, warning reason: generated code\n",
        "ignore fixtures/ reason: measured by tests, not judged\n",
    ] {
        assert!(effective.contains(line), "missing {line:?} in\n{effective}");
    }
}

#[test]
fn on_equal_specificity_the_later_scope_wins() {
    let text = r#"
[[scopes]]
paths = ["src/a/"]
reason = "first"
[scopes.limits.routine.nesting]
max = 5

[[scopes]]
paths = ["src/a/"]
reason = "second"
[scopes.limits.routine.nesting]
max = 6
"#;
    let config = parse_config(Some(text)).unwrap();
    assert_eq!(
        limit_of(&config, Scope::Routine, "nesting", "src/a/x.rs")
            .unwrap()
            .0,
        6.0
    );
}

#[test]
fn every_problem_is_reported_with_its_key() {
    let text = r#"
[limits.routine.cyclomatic]
max = 20

[limits.routine.nesting]
severity = "info"

[limits.routine.speed]
max = 1

[limits.type.methods]
severity = "fatal"

[limits.file.code_lines]
max = -1

[[scopes]]
paths = ["src/x/"]
[scopes.limits.routine.statements]
max = 99

[[scopes]]
paths = []
reason = "no paths"

[[ignores]]
paths = ["../outside", "C:/abs"]
reason = "   "
"#;
    let errors = parse_config(Some(text)).unwrap_err();
    let listed: Vec<String> = errors.iter().map(ToString::to_string).collect();
    let all = listed.join("\n");
    for needle in [
        "limits.routine.cyclomatic: is looser than the built-in limit and needs a reason",
        "limits.routine.nesting: is looser than the built-in limit and needs a reason",
        "limits.routine.speed: unknown routine metric",
        "limits.type.methods.severity: unknown severity",
        "limits.file.code_lines.max: must be a finite number of at least 0",
        "scopes[0]: every scope needs a reason",
        "scopes[1].paths: needs at least one path",
        "ignores[0].paths: `../outside` is not a project-relative path",
        "ignores[0].paths: `C:/abs` is not a project-relative path",
        "ignores[0]: every ignore needs a reason",
    ] {
        assert!(all.contains(needle), "missing `{needle}` in:\n{all}");
    }
    assert_eq!(errors.len(), 10, "{all}");
}

#[test]
fn unknown_keys_and_malformed_toml_are_rejected() {
    let unknown = parse_config(Some("[limits.routine.cyclomatic]\nmaximum = 3\n")).unwrap_err();
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].key, ".kiro/code-rules.toml");
    assert!(
        unknown[0].message.contains("maximum"),
        "{}",
        unknown[0].message
    );
    assert!(!unknown[0].message.contains('\n'));

    let top = parse_config(Some("[rules]\nx = 1\n")).unwrap_err();
    assert!(top[0].message.contains("rules"), "{}", top[0].message);

    let not_a_number =
        parse_config(Some("[limits.routine.cyclomatic]\nmax = \"ten\"\n")).unwrap_err();
    assert_eq!(not_a_number.len(), 1);

    let broken = parse_config(Some("[limits\n")).unwrap_err();
    assert_eq!(broken.len(), 1);
}

fn nesting(path: &str) -> Observation {
    Observation {
        entity: EntityKey {
            scope: Scope::Routine,
            path: path.to_string(),
            qualified_name: "deep".to_string(),
            parameters: "()".to_string(),
            ordinal: 0,
        },
        metric: MetricId {
            name: "nesting".to_string(),
            definition_version: "1".to_string(),
        },
        value: 4.0,
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

#[test]
fn configured_severity_scope_and_ignore_change_the_judgement() {
    let mut workspace = common::minimal_workspace();
    let features = [workspace.load("demo").clone()];
    let specs: Vec<&Feature> = features.iter().collect();
    let facts = CodeFacts {
        observations: vec![nesting("src/demo.rs"), nesting("src/ui.rs")],
        coverage: vec![complete("src/demo.rs"), complete("src/ui.rs")],
        ..CodeFacts::default()
    };
    let selected: Vec<String> = ["src/demo.rs", "src/ui.rs"].map(String::from).to_vec();
    let run = |text: &str| {
        let config = parse_config(Some(text)).unwrap();
        evaluate(&EvaluateInput {
            facts: &facts,
            config: &config,
            selected: &selected,
            specs: &specs,
            before: None,
            before_paths: &[],
            extensions: &[],
            extension_notes: &[],
        })
    };
    let limit_files = |report: &kiro_core::code::CodeReport| -> Vec<String> {
        report
            .findings
            .iter()
            .filter(|entry| entry.finding.code == "CODE_ROUTINE_LIMIT")
            .map(|entry| entry.finding.file.clone())
            .collect()
    };

    let strict = run("[limits.routine.nesting]\nseverity = \"error\"\n");
    assert_eq!(strict.counts().0, 2, "both breaches are errors now");
    assert_eq!(strict.findings[0].limit_source.as_deref(), Some("file"));

    let scoped = run(
        "[[scopes]]\npaths = [\"src/ui.rs\"]\nreason = \"view code\"\n[scopes.limits.routine.nesting]\nmax = 4\n",
    );
    assert_eq!(
        limit_files(&scoped),
        ["src/demo.rs"],
        "the scope relaxes only its own file"
    );

    let ignoring = run("[[ignores]]\npaths = [\"src/demo.rs\"]\nreason = \"legacy\"\n");
    assert_eq!(limit_files(&ignoring), ["src/ui.rs"]);
    assert_eq!(ignoring.coverage.ignored, ["src/demo.rs"]);
    let coverage = ignoring.findings.last().unwrap();
    assert!(
        coverage
            .finding
            .message
            .contains("1 ignored by configuration")
    );
}
