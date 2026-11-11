//! Metric snapshot and catalogue (quality-telemetry requirement 1).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_core::telemetry::{
    ATTRIBUTE_KEYS, Attr, METRICS, MetricPoint, MetricProvider, RESERVED_PREFIXES, Unit,
    catalogue_markdown, field_name, lookup, snapshot, stream_name,
};

struct Fixed(&'static str, Vec<MetricPoint>);

impl MetricProvider for Fixed {
    fn source(&self) -> &'static str {
        self.0
    }
    fn points(&self) -> Vec<MetricPoint> {
        self.1.clone()
    }
}

fn findings(spec: &str, code: &str, value: f64) -> MetricPoint {
    MetricPoint::new(
        "kiro.spec.findings",
        Unit::Count,
        value,
        vec![
            Attr::new("kiro.spec", spec),
            Attr::new("kiro.code", code),
            Attr::new("kiro.severity", "warning"),
        ],
    )
}

fn tasks_total(spec: &str, value: f64) -> MetricPoint {
    MetricPoint::new(
        "kiro.spec.tasks.total",
        Unit::Count,
        value,
        vec![Attr::new("kiro.spec", spec)],
    )
}

#[test]
fn equal_input_gives_equal_snapshots_in_any_order() {
    let a = Fixed(
        "check",
        vec![
            findings("b", "EARS_MODAL", 2.0),
            findings("a", "EARS_MODAL", 1.0),
        ],
    );
    let b = Fixed(
        "check",
        vec![
            findings("a", "EARS_MODAL", 1.0),
            findings("b", "EARS_MODAL", 2.0),
        ],
    );
    let first = snapshot(&[&a], &["check"]);
    let second = snapshot(&[&b], &["check"]);
    assert_eq!(first, second);
    assert_eq!(first.points.len(), 2);
    assert_eq!(first.points[0].attrs[2].value, "a", "sorted by attributes");
}

#[test]
fn attribute_order_within_a_point_does_not_matter() {
    let mut reordered = findings("a", "EARS_MODAL", 1.0);
    reordered.attrs.reverse();
    let first = snapshot(
        &[&Fixed("check", vec![findings("a", "EARS_MODAL", 1.0)])],
        &[],
    );
    let second = snapshot(&[&Fixed("check", vec![reordered])], &[]);
    assert_eq!(first, second);
}

#[test]
fn an_expected_source_without_a_provider_is_absent_and_not_zero() {
    let check = Fixed("check", vec![findings("a", "EARS_MODAL", 1.0)]);
    let result = snapshot(&[&check], &["check", "code"]);
    let code = result
        .coverage
        .iter()
        .find(|entry| entry.source == "code")
        .unwrap();
    assert!(!code.present);
    assert!(code.reason.is_some());
    assert!(
        result.points.iter().all(|point| point.value != 0.0),
        "an absent source adds no zero point"
    );
    assert_eq!(result.points.len(), 1);
}

#[test]
fn invalid_points_are_dropped_and_named_in_coverage() {
    let provider = Fixed(
        "check",
        vec![
            findings("a", "EARS_MODAL", 1.0),
            MetricPoint::new("kiro.unknown", Unit::Count, 1.0, vec![]),
            MetricPoint::new(
                "kiro.spec.tasks.total",
                Unit::Count,
                1.0,
                vec![Attr::new("kiro.code", "X")],
            ),
            tasks_total("a", f64::NAN),
            tasks_total("a", f64::INFINITY),
            MetricPoint::new(
                "kiro.spec.tasks.total",
                Unit::Lines,
                1.0,
                vec![Attr::new("kiro.spec", "a")],
            ),
            findings("a", "EARS_MODAL", 9.0),
        ],
    );
    let result = snapshot(&[&provider], &[]);
    assert_eq!(result.points.len(), 1, "only the first valid point is kept");
    assert_eq!(result.points[0].value, 1.0);
    let reason = result.coverage[0].reason.as_ref().unwrap();
    assert!(reason.starts_with("dropped 6 point(s)"), "{reason}");
    for needle in [
        "not in the catalogue",
        "is not listed for this metric",
        "value is not finite",
        "unit lines differs",
        "duplicate point",
    ] {
        assert!(reason.contains(needle), "{needle} missing in {reason}");
    }
}

#[test]
fn a_second_provider_adds_points_without_any_other_change() {
    let check = Fixed("check", vec![findings("a", "EARS_MODAL", 1.0)]);
    let status = Fixed("status", vec![tasks_total("a", 7.0)]);
    let result = snapshot(&[&check, &status], &["check", "status"]);
    assert_eq!(result.points.len(), 2);
    assert_eq!(result.coverage.len(), 2);
    assert!(result.coverage.iter().all(|entry| entry.present));
}

#[test]
fn catalogue_is_sorted_unique_and_uses_only_listed_attributes() {
    for pair in METRICS.windows(2) {
        assert!(pair[0].name < pair[1].name, "{} out of order", pair[1].name);
    }
    for info in METRICS {
        assert!(info.name.starts_with("kiro."), "{}", info.name);
        assert!(!info.meaning.is_empty());
        for key in info.attributes {
            assert!(ATTRIBUTE_KEYS.contains(key), "{} uses {key}", info.name);
        }
        // A metric under a reserved prefix belongs to the spec that owns the
        // prefix: its source is the word between `kiro.` and the next dot.
        for (prefix, owner) in RESERVED_PREFIXES {
            if info.name.starts_with(prefix) {
                let source = prefix.trim_start_matches("kiro.").trim_end_matches('.');
                assert_eq!(
                    info.source, source,
                    "{} lies under the prefix of {owner}",
                    info.name
                );
            }
        }
        assert_eq!(lookup(info.name), Some(info));
    }
    assert!(lookup("kiro.nope").is_none());
}

#[test]
fn no_attribute_key_can_carry_a_routine_path_or_message() {
    for key in ATTRIBUTE_KEYS {
        for forbidden in ["routine", "file", "path", "message", "name", "line"] {
            assert!(!key.contains(forbidden), "{key}");
        }
    }
}

#[test]
fn backend_names_replace_dots_with_underscores() {
    assert_eq!(stream_name("kiro.spec.findings"), "kiro_spec_findings");
    assert_eq!(field_name("kiro.spec"), "kiro_spec");
}

#[test]
fn documentation_contains_the_catalogue_table_verbatim() {
    let docs = include_str!("../../../docs/telemetry.md").replace("\r\n", "\n");
    let table = catalogue_markdown();
    assert!(
        docs.contains(&table),
        "docs/telemetry.md is out of date; its catalogue table must equal:\n{table}"
    );
    let documented = docs
        .lines()
        .filter(|line| line.starts_with("| `kiro."))
        .count();
    assert_eq!(
        documented,
        METRICS.len(),
        "docs/telemetry.md lists a metric the catalogue does not have"
    );
}

mod common;

use kiro_core::checks::{CheckConfig, run_checks};
use kiro_core::telemetry::{SpecInput, SpecMetrics, snapshot_json, snapshot_text};

/// Snapshot of the minimal fixture tree: `demo` is checked, `old` is only loaded.
fn minimal_snapshot() -> kiro_core::telemetry::MetricSnapshot {
    let mut workspace = common::minimal_workspace();
    let report = run_checks(&mut workspace, "demo", &CheckConfig::default());
    let demo = workspace.load("demo").clone();
    let old = workspace.load("old").clone();
    let mut findings = demo.findings.clone();
    findings.extend(report.findings.iter().cloned());
    let metrics = SpecMetrics {
        specs: vec![
            SpecInput {
                feature: &demo,
                report: Some(&report),
                findings: Some(&findings),
            },
            SpecInput {
                feature: &old,
                report: None,
                findings: None,
            },
        ],
    };
    snapshot(&[&metrics], &["spec", "code"])
}

#[test]
fn minimal_tree_snapshot_matches_the_expected_fixture() {
    let actual = snapshot_json(&minimal_snapshot());
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/telemetry/expected.json"
    );
    if std::env::var_os("KIRO_UPDATE_TELEMETRY").is_some() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
    assert_eq!(actual, expected);
}

#[test]
fn spec_metrics_follow_the_fixture_documents() {
    let result = minimal_snapshot();
    let value = |name: &str, spec: &str| {
        result
            .points
            .iter()
            .find(|point| {
                point.name == name
                    && point
                        .attrs
                        .iter()
                        .any(|attr| attr.key == "kiro.spec" && attr.value == spec)
            })
            .map(|point| point.value)
    };
    // demo has tasks, requirements and a design.
    let total = value("kiro.spec.tasks.total", "demo").unwrap();
    let done = value("kiro.spec.tasks.done", "demo").unwrap();
    assert!(total >= 1.0 && done <= total);
    assert!(value("kiro.spec.criteria.total", "demo").unwrap() >= 1.0);
    assert!(value("kiro.trace.components.unmapped", "demo").is_some());
    assert!(value("kiro.trace.criteria.untasked", "demo").is_some());
    // old has only spec.json: status is known, the rest is absent, not zero.
    assert_eq!(value("kiro.spec.phase", "old"), Some(1.0));
    assert!(value("kiro.spec.tasks.total", "old").is_none());
    assert!(value("kiro.spec.criteria.total", "old").is_none());
    assert!(value("kiro.trace.files.multi_owned", "old").is_none());
    assert!(value("kiro.spec.findings", "old").is_none());
    // The analyzer source did not run.
    let code = result
        .coverage
        .iter()
        .find(|entry| entry.source == "code")
        .unwrap();
    assert!(!code.present);
    // Nothing was dropped: every emitted point is in the catalogue.
    let spec = result
        .coverage
        .iter()
        .find(|entry| entry.source == "spec")
        .unwrap();
    assert_eq!(spec.reason, None);
}

#[test]
fn text_rendering_lists_points_and_coverage() {
    let text = snapshot_text(&minimal_snapshot());
    assert!(text.contains("kiro.spec.tasks.total{kiro.spec=demo} "));
    assert!(text.contains("source code: absent"));
    assert!(text.contains("source spec: present\n"));
}

#[test]
fn every_criterion_of_the_fixture_is_referenced_by_a_task() {
    // tasks.md references 1.1, 1.2, 2.2 (sub-tasks) and 2.1 (a major).
    let result = minimal_snapshot();
    let untasked = result
        .points
        .iter()
        .find(|point| point.name == "kiro.trace.criteria.untasked")
        .unwrap();
    assert_eq!(untasked.value, 0.0);
}
