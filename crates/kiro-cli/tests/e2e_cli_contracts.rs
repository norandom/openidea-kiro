//! First migrated command through the real binary (cli-contracts 2.4, 1.3, 1.5).
//!
//! A temporary spec carries one known violation. The same `check` command
//! fails, the fixture is repaired, and the command passes. Environment
//! fallbacks, report routing and failure exits are exercised on the way.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use std::path::Path;

use common::{run_cli_env, run_cli_in};
use tempfile::TempDir;

const VIOLATING: &str = "# Requirements\n\n### Requirement 1: Alpha\n\n#### Acceptance Criteria\n\n1. The system works quickly.\n";
const REPAIRED: &str = "# Requirements\n\n### Requirement 1: Alpha\n\n#### Acceptance Criteria\n\n1. The system shall respond within one second.\n";
const FINDING: &str = ".kiro/specs/demo/requirements.md:7: ERROR EARS_NO_SHALL";

fn project(requirements: &str) -> TempDir {
    let temp = TempDir::new().unwrap();
    let spec = temp.path().join(".kiro").join("specs").join("demo");
    std::fs::create_dir_all(&spec).unwrap();
    std::fs::write(
        spec.join("spec.json"),
        r#"{"feature_name": "demo", "phase": "requirements-generated"}"#,
    )
    .unwrap();
    std::fs::write(spec.join("requirements.md"), requirements).unwrap();
    temp
}

fn write_requirements(root: &Path, text: &str) {
    std::fs::write(
        root.join(".kiro")
            .join("specs")
            .join("demo")
            .join("requirements.md"),
        text,
    )
    .unwrap();
}

#[test]
fn check_fails_is_repaired_and_passes() {
    let temp = project(VIOLATING);
    let root = temp.path();
    let args = ["check", "--all"];

    let failing = run_cli_in(root, &args);
    assert_eq!(failing.code, 1, "stderr: {}", failing.stderr);
    assert!(failing.stdout.contains(FINDING), "{}", failing.stdout);
    assert!(failing.stdout.contains("fail-on error: fail"));
    assert_eq!(failing.stderr, "");

    write_requirements(root, REPAIRED);

    let passing = run_cli_in(root, &args);
    assert_eq!(passing.code, 0, "stdout: {}", passing.stdout);
    assert!(!passing.stdout.contains("EARS_NO_SHALL"));
    assert!(passing.stdout.contains("fail-on error: pass"));
    assert_eq!(passing.stderr, "");
}

#[test]
fn the_journey_depends_on_the_check_itself() {
    // With the EARS suite switched off the violation is not found, so the
    // failing step above can only pass when the check really runs.
    let temp = project(VIOLATING);
    let out = run_cli_in(temp.path(), &["check", "--all", "--checks", "terms"]);
    assert_eq!(out.code, 0);
    assert!(!out.stdout.contains("EARS_NO_SHALL"));
}

#[test]
fn environment_is_a_fallback_and_the_command_line_wins() {
    let temp = project(REPAIRED);
    let root = temp.path();

    // KIRO_FORMAT selects JSON when no flag is given.
    let json = run_cli_env(root, &[("KIRO_FORMAT", "json")], &["check", "--all"]);
    assert_eq!(json.code, 0);
    assert!(json.stdout.starts_with("{\n  \"schema_version\""));

    // An explicit flag overrides it.
    let text = run_cli_env(
        root,
        &[("KIRO_FORMAT", "json")],
        &["check", "--all", "--format", "text"],
    );
    assert!(text.stdout.starts_with("Vocabulary:"), "{}", text.stdout);

    // KIRO_FAIL_ON is reflected in the verdict line; the flag overrides it.
    let warning = run_cli_env(root, &[("KIRO_FAIL_ON", "warning")], &["check", "--all"]);
    assert!(warning.stdout.contains("fail-on warning: pass"));
    let error = run_cli_env(
        root,
        &[("KIRO_FAIL_ON", "warning")],
        &["check", "--all", "--fail-on", "error"],
    );
    assert!(error.stdout.contains("fail-on error: pass"));

    // KIRO_PROJECT names the project when --project is absent.
    let elsewhere = TempDir::new().unwrap();
    let by_env = run_cli_env(
        elsewhere.path(),
        &[("KIRO_PROJECT", root.to_str().unwrap())],
        &["check", "--all"],
    );
    assert_eq!(by_env.code, 0, "stderr: {}", by_env.stderr);
    assert!(by_env.stdout.contains("Spec: demo"));
}

#[test]
fn invalid_environment_value_names_the_variable_and_exits_2() {
    let temp = project(REPAIRED);
    let out = run_cli_env(temp.path(), &[("KIRO_FORMAT", "yaml")], &["check", "--all"]);
    assert_eq!(out.code, 2);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr,
        "invalid value in environment variable KIRO_FORMAT; expected one of: text, json\n"
    );
}

#[test]
fn a_named_output_receives_the_report_and_stdout_stays_empty() {
    let temp = project(VIOLATING);
    let root = temp.path();
    let target = root.join("report.json");

    let out = run_cli_in(
        root,
        &[
            "check",
            "--all",
            "--format",
            "json",
            "--output",
            "report.json",
        ],
    );
    assert_eq!(out.code, 1, "the verdict is unchanged by the destination");
    assert_eq!(out.stdout, "");
    assert_eq!(out.stderr, "");

    let on_stdout = run_cli_in(root, &["check", "--all", "--format", "json"]);
    let written = std::fs::read_to_string(&target)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(written, on_stdout.stdout, "same document either way");

    let dash = run_cli_in(
        root,
        &["check", "--all", "--format", "json", "--output", "-"],
    );
    assert_eq!(dash.stdout, on_stdout.stdout, "'-' means standard output");
}

#[test]
fn a_failed_report_destination_exits_2_with_one_line() {
    let temp = project(REPAIRED);
    let out = run_cli_in(
        temp.path(),
        &["check", "--all", "--output", "missing-dir/report.txt"],
    );
    assert_eq!(out.code, 2);
    assert_eq!(out.stdout, "");
    assert!(out.stderr.starts_with("failed to write report to "));
    assert_eq!(out.stderr.lines().count(), 1);
}

#[test]
fn quiet_and_verbose_conflict_is_a_usage_error() {
    let temp = project(REPAIRED);
    let out = run_cli_in(temp.path(), &["-q", "-v", "check", "--all"]);
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        "--quiet and --verbose cannot be used together\n"
    );
}
