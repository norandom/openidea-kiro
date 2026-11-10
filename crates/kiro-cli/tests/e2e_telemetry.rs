//! Telemetry journey (quality-telemetry).
//!
//! Task 1.2: the OTLP JSON lines that OpenObserve ingested in the T0
//! prototype decode with the official OpenTelemetry protocol definitions.
//! These recorded lines are the contract the later mapping must meet; the
//! real-binary export journey is added by task 4.1.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use opentelemetry_proto::tonic::collector::logs::v1::ExportLogsServiceRequest;
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use opentelemetry_proto::tonic::metrics::v1::metric::Data;

const RESOURCE: &str = r#""resource":{"attributes":[{"key":"service.name","value":{"stringValue":"kiro-cli"}},{"key":"kiro.run.kind","value":{"stringValue":"manual"}}]}"#;

fn metrics_line() -> String {
    format!(
        r#"{{"resourceMetrics":[{{{RESOURCE},"scopeMetrics":[{{"scope":{{"name":"kiro"}},"metrics":[{{"name":"kiro.spec.findings","unit":"1","gauge":{{"dataPoints":[{{"asDouble":4.0,"timeUnixNano":"1794337635000000000","attributes":[{{"key":"kiro.spec","value":{{"stringValue":"json-probe"}}}},{{"key":"kiro.code","value":{{"stringValue":"EARS_MODAL"}}}}]}}]}}}}]}}]}}]}}"#
    )
}

fn logs_line() -> String {
    format!(
        r#"{{"resourceLogs":[{{{RESOURCE},"scopeLogs":[{{"scope":{{"name":"kiro"}},"logRecords":[{{"timeUnixNano":"1794337635000000000","severityNumber":13,"severityText":"WARN","body":{{"stringValue":"json probe finding"}},"attributes":[{{"key":"kiro.spec","value":{{"stringValue":"json-probe"}}}},{{"key":"kiro.line","value":{{"intValue":"18"}}}}]}}]}}]}}]}}"#
    )
}

fn traces_line() -> String {
    format!(
        r#"{{"resourceSpans":[{{{RESOURCE},"scopeSpans":[{{"scope":{{"name":"kiro"}},"spans":[{{"traceId":"5b8efff798038103d269b633813fc60c","spanId":"eee19b7ec3c1b174","name":"kiro.gate","kind":1,"startTimeUnixNano":"1794337635000000000","endTimeUnixNano":"1794337635020000000","attributes":[{{"key":"kiro.verdict","value":{{"stringValue":"pass"}}}}]}}]}}]}}]}}"#
    )
}

#[test]
fn recorded_metrics_line_decodes_as_a_gauge() {
    let request: ExportMetricsServiceRequest = serde_json::from_str(&metrics_line()).unwrap();
    let resource = &request.resource_metrics[0];
    assert_eq!(resource.resource.as_ref().unwrap().attributes.len(), 2);
    let metric = &resource.scope_metrics[0].metrics[0];
    assert_eq!(metric.name, "kiro.spec.findings");
    let Some(Data::Gauge(gauge)) = &metric.data else {
        panic!("expected a gauge, got {:?}", metric.data);
    };
    assert_eq!(gauge.data_points.len(), 1);
    assert_eq!(
        gauge.data_points[0].time_unix_nano,
        1_794_337_635_000_000_000
    );
    assert_eq!(gauge.data_points[0].attributes.len(), 2);
}

#[test]
fn recorded_logs_line_decodes_as_one_record() {
    let request: ExportLogsServiceRequest = serde_json::from_str(&logs_line()).unwrap();
    let record = &request.resource_logs[0].scope_logs[0].log_records[0];
    assert_eq!(record.severity_number, 13);
    assert_eq!(record.severity_text, "WARN");
    assert!(record.body.is_some());
    assert_eq!(record.attributes.len(), 2);
}

#[test]
fn recorded_traces_line_decodes_as_one_span() {
    let request: ExportTraceServiceRequest = serde_json::from_str(&traces_line()).unwrap();
    let span = &request.resource_spans[0].scope_spans[0].spans[0];
    assert_eq!(span.name, "kiro.gate");
    assert_eq!(span.trace_id.len(), 16);
    assert_eq!(span.span_id.len(), 8);
    assert_eq!(
        span.end_time_unix_nano - span.start_time_unix_nano,
        20_000_000
    );
}

#[test]
fn malformed_lines_are_rejected() {
    let wrong_shape = r#"{"resourceMetrics":[{"scopeMetrics":"x"}]}"#;
    assert!(serde_json::from_str::<ExportMetricsServiceRequest>(wrong_shape).is_err());

    let bad_trace_id = traces_line().replace("5b8efff798038103d269b633813fc60c", "not-hex");
    assert!(serde_json::from_str::<ExportTraceServiceRequest>(&bad_trace_id).is_err());

    let truncated = &logs_line()[..40];
    assert!(serde_json::from_str::<ExportLogsServiceRequest>(truncated).is_err());
}

mod common;

use common::{core_fixture_path, run_cli, setup_tree};

fn files_under(root: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path.strip_prefix(root).unwrap().display().to_string());
            }
        }
    }
    out.sort();
    out
}

#[test]
fn dry_run_prints_the_snapshot_and_writes_nothing() {
    let (_temp, path) = setup_tree(&core_fixture_path("minimal"));
    let project = path.to_str().unwrap();
    let before = files_under(&path);

    let text = run_cli(&[
        "--project",
        project,
        "telemetry",
        "export",
        "--all",
        "--dry-run",
    ]);
    assert_eq!(text.code, 0, "stderr: {}", text.stderr);
    assert_eq!(text.stderr, "");
    for line in [
        "kiro.spec.tasks.total{kiro.spec=demo} 4 count\n",
        "kiro.spec.tasks.done{kiro.spec=demo} 1 count\n",
        "kiro.spec.criteria.total{kiro.spec=demo} 4 count\n",
        "kiro.trace.criteria.untasked{kiro.spec=demo} 0 count\n",
        "kiro.spec.phase{kiro.phase=tasks-generated,kiro.spec=demo} 1 value\n",
        "source spec: present\n",
    ] {
        assert!(
            text.stdout.contains(line),
            "missing {line:?} in\n{}",
            text.stdout
        );
    }

    let json = run_cli(&[
        "--project",
        project,
        "telemetry",
        "export",
        "--feature",
        "demo",
        "--dry-run",
        "--format",
        "json",
    ]);
    assert_eq!(json.code, 0);
    let document: serde_json::Value = serde_json::from_str(&json.stdout).unwrap();
    assert_eq!(document["schema_version"], 1);
    let points = document["points"].as_array().unwrap();
    assert!(!points.is_empty());
    assert!(
        points
            .iter()
            .all(|point| point["attributes"]["kiro.spec"] == "demo"),
        "--feature limits the snapshot to that spec"
    );

    assert_eq!(files_under(&path), before, "a dry run writes nothing");
}

#[test]
fn export_without_a_directory_or_dry_run_is_a_usage_failure() {
    let (_temp, path) = setup_tree(&core_fixture_path("minimal"));
    let out = run_cli(&[
        "--project",
        path.to_str().unwrap(),
        "telemetry",
        "export",
        "--all",
    ]);
    assert_eq!(out.code, 2);
    assert_eq!(out.stdout, "");
    assert!(out.stderr.contains("--telemetry-dir"));
    assert!(out.stderr.contains("KIRO_TELEMETRY_DIR"));
    assert!(out.stderr.contains("--dry-run"));
}

// ------------------------------------------------------------ written files

use common::run_cli_env;
use opentelemetry_proto::tonic::common::v1::{KeyValue, any_value};
use tempfile::TempDir;

struct Written {
    metrics: Vec<ExportMetricsServiceRequest>,
    logs: Vec<ExportLogsServiceRequest>,
    traces: Vec<ExportTraceServiceRequest>,
    raw: String,
}

/// Reads the three files and decodes every line with the official
/// protocol definitions. A line that does not decode fails the test.
fn read_written(dir: &std::path::Path) -> Written {
    let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap();
    let metrics = read("metrics.jsonl");
    let logs = read("logs.jsonl");
    let traces = read("traces.jsonl");
    for text in [&metrics, &logs, &traces] {
        assert!(text.ends_with('\n'), "every line is terminated");
        assert!(!text.contains('\r'));
    }
    Written {
        metrics: metrics
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect(),
        logs: logs
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect(),
        traces: traces
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect(),
        raw: format!("{metrics}{logs}{traces}"),
    }
}

fn text_of(attributes: &[KeyValue], key: &str) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.key == key)
        .and_then(|attribute| attribute.value.as_ref())
        .and_then(|value| match &value.value {
            Some(any_value::Value::StringValue(text)) => Some(text.clone()),
            _ => None,
        })
}

/// The value of the gauge point `name` whose attributes include `spec`.
fn gauge(written: &Written, name: &str, spec: &str) -> Option<f64> {
    use opentelemetry_proto::tonic::metrics::v1::number_data_point::Value as Number;
    written.metrics.iter().find_map(|request| {
        request.resource_metrics[0].scope_metrics[0]
            .metrics
            .iter()
            .filter(|metric| metric.name == name)
            .find_map(|metric| {
                let Some(Data::Gauge(gauge)) = &metric.data else {
                    panic!("{name} is not a gauge");
                };
                gauge
                    .data_points
                    .iter()
                    .find(|point| text_of(&point.attributes, "kiro.spec").as_deref() == Some(spec))
                    .and_then(|point| match point.value {
                        Some(Number::AsDouble(value)) => Some(value),
                        _ => None,
                    })
            })
    })
}

fn minimal_project() -> (TempDir, std::path::PathBuf, TempDir) {
    let (temp, path) = setup_tree(&core_fixture_path("minimal"));
    let telemetry = TempDir::new().unwrap();
    (temp, path, telemetry)
}

#[test]
fn export_writes_three_decodable_files_with_the_snapshot_values() {
    let (_temp, path, telemetry) = minimal_project();
    let project = path.to_str().unwrap();
    let dir = telemetry.path().join("out");
    let before = files_under(&path);

    let out = run_cli(&[
        "--project",
        project,
        "telemetry",
        "export",
        "--all",
        "--telemetry-dir",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "", "the export prints no report");
    assert_eq!(
        files_under(&path),
        before,
        "nothing is written into the project"
    );

    let written = read_written(&dir);
    assert_eq!(written.metrics.len(), 1);
    assert_eq!(written.logs.len(), 1);
    assert_eq!(written.traces.len(), 1);

    // Gauges carry the values the dry run prints.
    assert_eq!(gauge(&written, "kiro.spec.tasks.total", "demo"), Some(4.0));
    assert_eq!(gauge(&written, "kiro.spec.tasks.done", "demo"), Some(1.0));
    assert_eq!(
        gauge(&written, "kiro.spec.criteria.total", "demo"),
        Some(4.0)
    );
    assert_eq!(
        gauge(&written, "kiro.trace.criteria.untasked", "demo"),
        Some(0.0)
    );
    assert_eq!(
        gauge(&written, "kiro.spec.tasks.total", "old"),
        None,
        "absent, not zero"
    );

    // One record per finding of the run.
    let records = &written.logs[0].resource_logs[0].scope_logs[0].log_records;
    let json = run_cli(&["--project", project, "check", "--all", "--format", "json"]);
    let report: serde_json::Value = serde_json::from_str(&json.stdout).unwrap();
    let counts = &report["summary"];
    let total = counts["errors"].as_u64().unwrap()
        + counts["warnings"].as_u64().unwrap()
        + counts["info"].as_u64().unwrap();
    assert_eq!(records.len() as u64, total);
    assert!(records.iter().all(|record| record.body.is_some()));
    assert!(
        records
            .iter()
            .all(|record| text_of(&record.attributes, "kiro.code").is_some())
    );

    // One trace: a root span and one child per phase.
    let spans = &written.traces[0].resource_spans[0].scope_spans[0].spans;
    let names: Vec<&str> = spans.iter().map(|span| span.name.as_str()).collect();
    assert_eq!(names, ["kiro.gate", "discover", "check"]);
    let root = &spans[0];
    assert!(root.parent_span_id.is_empty());
    assert_eq!(
        text_of(&root.attributes, "kiro.verdict").as_deref(),
        Some("pass")
    );
    for child in &spans[1..] {
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.parent_span_id, root.span_id);
        assert_ne!(child.span_id, root.span_id);
        assert!(child.start_time_unix_nano >= root.start_time_unix_nano);
        assert!(child.end_time_unix_nano <= root.end_time_unix_nano);
    }

    // Resource identity on every signal; no machine detail anywhere.
    let resource = written.metrics[0].resource_metrics[0]
        .resource
        .as_ref()
        .unwrap();
    assert_eq!(
        text_of(&resource.attributes, "service.name").as_deref(),
        Some("kiro-cli")
    );
    assert_eq!(
        text_of(&resource.attributes, "kiro.run.kind").as_deref(),
        Some("manual")
    );
    assert!(text_of(&resource.attributes, "service.version").is_some());
    assert!(!written.raw.contains(project), "no absolute project path");
    assert!(!written.raw.contains(&project.replace('\\', "/")));
    assert!(!written.raw.contains(telemetry.path().to_str().unwrap()));
    for variable in ["USERNAME", "USER"] {
        if let Ok(user) = std::env::var(variable)
            && user.len() > 3
        {
            assert!(!written.raw.contains(&user), "no user name");
        }
    }
}

#[test]
fn check_writes_telemetry_without_changing_report_or_exit_code() {
    let (_temp, path, telemetry) = minimal_project();
    let project = path.to_str().unwrap();
    let dir = telemetry.path().to_str().unwrap();

    let plain = run_cli(&["--project", project, "check", "--all"]);
    let with_flag = run_cli(&[
        "--project",
        project,
        "check",
        "--all",
        "--telemetry-dir",
        dir,
    ]);
    assert_eq!(with_flag.code, plain.code);
    assert_eq!(
        with_flag.stdout, plain.stdout,
        "the report is byte-identical"
    );
    assert_eq!(with_flag.stderr, plain.stderr);

    // The environment variable enables it too, and names the run kind.
    let with_env = run_cli_env(
        &path,
        &[
            ("KIRO_TELEMETRY_DIR", dir),
            ("KIRO_RUN_KIND", "ci"),
            ("OTEL_SERVICE_NAME", "gate-under-test"),
            ("OTEL_RESOURCE_ATTRIBUTES", "team=core,service.name=spoof"),
        ],
        &["check", "--all", "--format", "json"],
    );
    assert_eq!(with_env.code, plain.code);

    let written = read_written(telemetry.path());
    assert_eq!(written.traces.len(), 2, "one line per run");
    let spans = &written.traces[0].resource_spans[0].scope_spans[0].spans;
    let names: Vec<&str> = spans.iter().map(|span| span.name.as_str()).collect();
    assert_eq!(names, ["kiro.gate", "discover", "check", "render"]);

    let second = written.metrics[1].resource_metrics[0]
        .resource
        .as_ref()
        .unwrap();
    assert_eq!(
        text_of(&second.attributes, "kiro.run.kind").as_deref(),
        Some("ci")
    );
    assert_eq!(
        text_of(&second.attributes, "service.name").as_deref(),
        Some("gate-under-test"),
        "the operator's resource attributes cannot overwrite identity"
    );
    assert_eq!(text_of(&second.attributes, "team").as_deref(), Some("core"));
    let first_trace = &written.traces[0].resource_spans[0].scope_spans[0].spans[0].trace_id;
    let second_trace = &written.traces[1].resource_spans[0].scope_spans[0].spans[0].trace_id;
    assert_ne!(first_trace, second_trace, "each run has its own trace");
}

#[test]
fn a_failing_check_keeps_exit_1_and_records_the_verdict() {
    let temp = TempDir::new().unwrap();
    let spec = temp.path().join(".kiro").join("specs").join("demo");
    std::fs::create_dir_all(&spec).unwrap();
    std::fs::write(
        spec.join("spec.json"),
        r#"{"feature_name": "demo", "phase": "requirements-generated"}"#,
    )
    .unwrap();
    std::fs::write(
        spec.join("requirements.md"),
        "# Requirements\n\n### Requirement 1: Alpha\n\n#### Acceptance Criteria\n\n1. The system works quickly.\n",
    )
    .unwrap();
    let telemetry = TempDir::new().unwrap();

    let out = run_cli_env(
        temp.path(),
        &[("KIRO_TELEMETRY_DIR", telemetry.path().to_str().unwrap())],
        &["check", "--all"],
    );
    assert_eq!(out.code, 1);

    let written = read_written(telemetry.path());
    let root = &written.traces[0].resource_spans[0].scope_spans[0].spans[0];
    assert_eq!(
        text_of(&root.attributes, "kiro.verdict").as_deref(),
        Some("fail")
    );
    let records = &written.logs[0].resource_logs[0].scope_logs[0].log_records;
    assert_eq!(records[0].severity_text, "ERROR", "errors come first");
    assert_eq!(
        text_of(&records[0].attributes, "kiro.code").as_deref(),
        Some("EARS_NO_SHALL")
    );
    assert_eq!(
        text_of(&records[0].attributes, "kiro.file").as_deref(),
        Some(".kiro/specs/demo/requirements.md")
    );
}

#[test]
fn an_unwritable_directory_keeps_the_gate_result() {
    let (_temp, path, telemetry) = minimal_project();
    let project = path.to_str().unwrap();
    // A file where a directory is needed: the append must fail.
    let blocker = telemetry.path().join("blocker");
    std::fs::write(&blocker, "x").unwrap();
    let dir = blocker.join("sub");

    let plain = run_cli(&["--project", project, "check", "--all"]);
    let out = run_cli(&[
        "--project",
        project,
        "check",
        "--all",
        "--telemetry-dir",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(out.code, plain.code, "the gate's exit code is unchanged");
    assert_eq!(out.stdout, plain.stdout);
    assert_eq!(out.stderr.lines().count(), 1);
    assert!(out.stderr.starts_with("failed to "), "{}", out.stderr);

    // For the explicit export the write is the purpose: failure, exit 2.
    let export = run_cli(&[
        "--project",
        project,
        "telemetry",
        "export",
        "--all",
        "--telemetry-dir",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(export.code, 2);
}

#[test]
fn a_directory_inside_the_project_is_refused_before_the_gate_runs() {
    let (_temp, path, _telemetry) = minimal_project();
    let project = path.to_str().unwrap();
    let before = files_under(&path);
    for inside in [
        path.join("telemetry"),
        path.join(".kiro").join("telemetry"),
        path.join("a").join("..").join("b"),
    ] {
        let out = run_cli(&[
            "--project",
            project,
            "check",
            "--all",
            "--telemetry-dir",
            inside.to_str().unwrap(),
        ]);
        assert_eq!(out.code, 2);
        assert_eq!(out.stdout, "", "the gate did not run");
        assert!(out.stderr.contains("inside the project"), "{}", out.stderr);
    }
    // A relative path is taken from the current directory.
    let relative = run_cli_env(&path, &[], &["check", "--all", "--telemetry-dir", "out"]);
    assert_eq!(relative.code, 2);
    assert_eq!(files_under(&path), before, "nothing was created");
}

#[test]
fn an_invalid_run_kind_is_a_usage_failure_only_when_telemetry_is_on() {
    let (_temp, path, telemetry) = minimal_project();
    let off = run_cli_env(&path, &[("KIRO_RUN_KIND", "nightly")], &["check", "--all"]);
    assert_eq!(off.code, 0, "telemetry is off, the variable is not read");

    let on = run_cli_env(
        &path,
        &[
            ("KIRO_RUN_KIND", "nightly"),
            ("KIRO_TELEMETRY_DIR", telemetry.path().to_str().unwrap()),
        ],
        &["check", "--all"],
    );
    assert_eq!(on.code, 2);
    assert_eq!(on.stdout, "");
    assert!(on.stderr.contains("KIRO_RUN_KIND"));
    assert!(files_under(telemetry.path()).is_empty());
}

#[test]
fn a_dry_run_ignores_the_directory_from_the_environment() {
    let (_temp, path, telemetry) = minimal_project();
    let out = run_cli_env(
        &path,
        &[("KIRO_TELEMETRY_DIR", telemetry.path().to_str().unwrap())],
        &["telemetry", "export", "--all", "--dry-run"],
    );
    assert_eq!(out.code, 0);
    assert!(out.stdout.contains("kiro.spec.tasks.total{kiro.spec=demo}"));
    assert!(
        files_under(telemetry.path()).is_empty(),
        "a dry run writes nothing"
    );
}

#[test]
fn parallel_runs_leave_only_whole_lines() {
    let (_temp, path, telemetry) = minimal_project();
    let project = path.to_str().unwrap().to_string();
    let dir = telemetry.path().to_str().unwrap().to_string();
    let runs = 8;
    let handles: Vec<_> = (0..runs)
        .map(|_| {
            let project = project.clone();
            let dir = dir.clone();
            std::thread::spawn(move || {
                run_cli(&[
                    "--project",
                    &project,
                    "check",
                    "--all",
                    "--telemetry-dir",
                    &dir,
                ])
                .code
            })
        })
        .collect();
    for handle in handles {
        assert_eq!(handle.join().unwrap(), 0);
    }
    // read_written fails on any line that does not decode.
    let written = read_written(telemetry.path());
    assert_eq!(written.metrics.len(), runs);
    assert_eq!(written.logs.len(), runs);
    assert_eq!(written.traces.len(), runs);
}

#[test]
fn the_record_cap_drops_the_least_severe_and_reports_the_count() {
    use kiro_core::finding::{Finding, Severity, codes};
    use kiro_core::telemetry::{MetricSnapshot, Phase, ResourceIdentity, RunIdentity, otlp_lines};
    let finding = |severity: Severity, message: &str| {
        Finding::new(codes::CHECK_COVERAGE, "f.md", message).severity(severity)
    };
    let findings = vec![
        finding(Severity::Info, "i"),
        finding(Severity::Warning, "w"),
        finding(Severity::Error, "e"),
    ];
    let run = RunIdentity {
        trace_id: [7; 16],
        root_span_id: [9; 8],
        start_unix_nanos: 1_000,
        end_unix_nanos: 5_000,
        phases: vec![Phase {
            name: "check",
            start_unix_nanos: 1_000,
            end_unix_nanos: 4_000,
        }],
        verdict: "fail".to_string(),
    };
    let snapshot = MetricSnapshot {
        schema_version: 1,
        points: Vec::new(),
        coverage: Vec::new(),
    };
    let lines = otlp_lines(&snapshot, &findings, &ResourceIdentity::default(), &run, 2);
    assert_eq!(lines.dropped_records, 1);
    for line in [&lines.metrics, &lines.logs, &lines.traces] {
        assert!(!line.contains('\n'));
    }

    let logs: ExportLogsServiceRequest = serde_json::from_str(&lines.logs).unwrap();
    let records = &logs.resource_logs[0].scope_logs[0].log_records;
    let severities: Vec<&str> = records
        .iter()
        .map(|record| record.severity_text.as_str())
        .collect();
    assert_eq!(
        severities,
        ["ERROR", "WARN"],
        "the information record is dropped"
    );

    let metrics: ExportMetricsServiceRequest = serde_json::from_str(&lines.metrics).unwrap();
    let names: Vec<&str> = metrics.resource_metrics[0].scope_metrics[0]
        .metrics
        .iter()
        .map(|metric| metric.name.as_str())
        .collect();
    assert_eq!(names, ["kiro.run.duration", "kiro.run.findings.dropped"]);

    let traces: ExportTraceServiceRequest = serde_json::from_str(&lines.traces).unwrap();
    let spans = &traces.resource_spans[0].scope_spans[0].spans;
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].trace_id, vec![7; 16]);
    assert_eq!(spans[1].parent_span_id, vec![9; 8]);
}

#[test]
fn export_needs_a_selection() {
    let (_temp, path) = setup_tree(&core_fixture_path("minimal"));
    let out = run_cli(&[
        "--project",
        path.to_str().unwrap(),
        "telemetry",
        "export",
        "--dry-run",
    ]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("--all"));
}

// ------------------------------------------------------------- code checks

/// Nesting 4 (limit 3) in the file the minimal fixture maps to component Core.
const DEEP: &str = "pub fn owner(paths: &[&str], wanted: &str) -> Option<usize> {
    for (index, path) in paths.iter().enumerate() {
        if !path.is_empty() {
            let mut rest = *path;
            while let Some(stripped) = rest.strip_prefix('/') {
                if stripped == wanted {
                    return Some(index);
                }
                rest = stripped;
            }
        }
    }
    None
}
";

#[test]
fn a_code_check_writes_its_metric_points_and_findings_with_hint_and_owners() {
    let (_temp, path, telemetry) = minimal_project();
    std::fs::create_dir_all(path.join("src")).unwrap();
    std::fs::write(path.join("src").join("demo.rs"), DEEP).unwrap();
    let project = path.to_str().unwrap();
    let dir = telemetry.path().to_str().unwrap();

    let plain = run_cli(&["--project", project, "code", "check", "--task", "demo:1.1"]);
    let out = run_cli(&[
        "--project",
        project,
        "code",
        "check",
        "--task",
        "demo:1.1",
        "--telemetry-dir",
        dir,
    ]);
    assert_eq!(out.code, plain.code);
    assert_eq!(out.stdout, plain.stdout, "the report is byte-identical");
    assert_eq!(out.stderr, "");

    let written = read_written(telemetry.path());
    // Values a reader can check against the source above: one routine,
    // nesting 4, one breach, one measured file.
    assert_eq!(gauge(&written, "kiro.code.routines", "demo"), Some(1.0));
    assert_eq!(gauge(&written, "kiro.code.breaches", "demo"), Some(1.0));
    let names: Vec<&str> = written.metrics[0].resource_metrics[0].scope_metrics[0]
        .metrics
        .iter()
        .map(|metric| metric.name.as_str())
        .collect();
    for name in [
        "kiro.code.highest",
        "kiro.code.total",
        "kiro.code.files",
        "kiro.run.duration",
    ] {
        assert!(names.contains(&name), "{name} missing in {names:?}");
    }
    assert!(
        !names.iter().any(|name| name.starts_with("kiro.spec.")),
        "no spec source in this run"
    );

    let records = &written.logs[0].resource_logs[0].scope_logs[0].log_records;
    let breach = records
        .iter()
        .find(|record| {
            text_of(&record.attributes, "kiro.code").as_deref() == Some("CODE_ROUTINE_LIMIT")
        })
        .unwrap();
    assert_eq!(
        text_of(&breach.attributes, "kiro.tag").as_deref(),
        Some("simplify")
    );
    assert!(
        text_of(&breach.attributes, "kiro.hint")
            .unwrap()
            .starts_with("return early")
    );
    assert_eq!(
        text_of(&breach.attributes, "kiro.metric").as_deref(),
        Some("nesting")
    );
    assert_eq!(
        text_of(&breach.attributes, "kiro.component").as_deref(),
        Some("Core")
    );
    assert_eq!(
        text_of(&breach.attributes, "kiro.task").as_deref(),
        Some("1.1,2")
    );
    assert_eq!(
        text_of(&breach.attributes, "kiro.spec").as_deref(),
        Some("demo")
    );
    assert_eq!(
        text_of(&breach.attributes, "kiro.pre_existing").as_deref(),
        Some("false")
    );

    let spans = &written.traces[0].resource_spans[0].scope_spans[0].spans;
    let phases: Vec<&str> = spans.iter().map(|span| span.name.as_str()).collect();
    assert_eq!(phases, ["kiro.gate", "discover", "measure", "render"]);
}
