//! The code gate through the real binary (code-constraints 12.2, 6, 7, 9).
//!
//! First slice: a task-owned Rust routine breaks a limit, the same command
//! fails, the source is repaired, and the command passes. The minimal
//! fixture maps `src/demo.rs` to component Core (tasks 1.1 and 2) and
//! `src/ui.rs` to component Ui (task 1.2).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use std::path::Path;

use common::{core_fixture_path, run_cli_in, setup_tree};

/// Nesting 4 (limit 3): for > if > while > if.
const VIOLATING: &str = "pub fn owner(paths: &[&str], wanted: &str) -> Option<usize> {
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

/// The same behaviour with the inner loop extracted: nesting 2.
const REPAIRED: &str = "pub fn owner(paths: &[&str], wanted: &str) -> Option<usize> {
    for (index, path) in paths.iter().enumerate() {
        if !path.is_empty() && matches_suffix(path, wanted) {
            return Some(index);
        }
    }
    None
}

fn matches_suffix(path: &str, wanted: &str) -> bool {
    let mut rest = path;
    while let Some(stripped) = rest.strip_prefix('/') {
        if stripped == wanted {
            return true;
        }
        rest = stripped;
    }
    false
}
";

const CLEAN_UI: &str = "pub fn show(owner: Option<usize>) -> String {
    match owner {
        Some(index) => format!(\"component {index}\"),
        None => String::from(\"not covered\"),
    }
}
";

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn project(demo: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let (temp, path) = setup_tree(&core_fixture_path("minimal"));
    write(&path, "src/demo.rs", demo);
    write(&path, "src/ui.rs", CLEAN_UI);
    (temp, path)
}

const TASK: &[&str] = &[
    "code",
    "check",
    "--task",
    "demo:1.1",
    "--fail-on",
    "warning",
];

#[test]
fn a_task_owned_violation_fails_is_repaired_and_passes() {
    let (_temp, root) = project(VIOLATING);

    let failing = run_cli_in(&root, TASK);
    assert_eq!(
        failing.code, 1,
        "stdout:\n{}\nstderr:\n{}",
        failing.stdout, failing.stderr
    );
    assert_eq!(failing.stderr, "");
    assert!(
        failing.stdout.contains(
            "src/demo.rs:1: WARNING CODE_ROUTINE_LIMIT routine `owner`: nesting is 4, limit 3 (default)"
        ),
        "{}",
        failing.stdout
    );
    // The finding names an edit and the owner in the language of the spec.
    assert!(
        failing
            .stdout
            .contains("    simplify: return early or extract the inner block")
    );
    assert!(
        failing
            .stdout
            .contains("[spec demo, component Core, tasks 1.1, 2]")
    );
    assert!(failing.stdout.contains("fail-on warning: fail"));

    write(&root, "src/demo.rs", REPAIRED);

    let passing = run_cli_in(&root, TASK);
    assert_eq!(passing.code, 0, "stdout:\n{}", passing.stdout);
    assert!(!passing.stdout.contains("CODE_ROUTINE_LIMIT"));
    assert!(passing.stdout.contains("fail-on warning: pass"));
    assert!(passing.stdout.contains("1 of 1 selected files measured"));
}

#[test]
fn the_task_selects_only_the_files_of_its_boundary() {
    // The violation is in Core's file; the Ui task does not see it.
    let (_temp, root) = project(VIOLATING);
    let ui = run_cli_in(
        &root,
        &[
            "code",
            "check",
            "--task",
            "demo:1.2",
            "--fail-on",
            "warning",
        ],
    );
    assert_eq!(ui.code, 0, "{}", ui.stdout);
    assert!(!ui.stdout.contains("src/demo.rs"));

    // The spec selection and the whole tree both see it.
    for selection in [&["--spec", "demo"][..], &["--all"][..]] {
        let mut args = vec!["code", "check"];
        args.extend_from_slice(selection);
        args.extend_from_slice(&["--fail-on", "warning"]);
        let out = run_cli_in(&root, &args);
        assert_eq!(out.code, 1, "{selection:?}");
        assert!(out.stdout.contains("CODE_ROUTINE_LIMIT"));
        assert!(out.stdout.contains("2 of 2 selected files measured"));
    }
}

#[test]
fn limits_are_warnings_until_calibrated_so_the_default_gate_passes() {
    let (_temp, root) = project(VIOLATING);
    let out = run_cli_in(&root, &["code", "check", "--task", "demo:1.1"]);
    assert_eq!(
        out.code, 0,
        "uncalibrated limits do not block at --fail-on error"
    );
    assert!(
        out.stdout.contains("CODE_ROUTINE_LIMIT"),
        "but they are reported"
    );
    assert!(out.stdout.contains("fail-on error: pass"));
}

#[test]
fn a_selected_file_that_does_not_parse_blocks_and_is_not_judged() {
    let (_temp, root) = project("pub fn broken( {\n");
    let out = run_cli_in(&root, &["code", "check", "--task", "demo:1.1"]);
    assert_eq!(out.code, 1, "a parse failure is an error");
    assert!(
        out.stdout
            .contains("src/demo.rs:1: ERROR CODE_PARSE_FAILED")
    );
    assert!(out.stdout.contains("cannot be reported as checked"));
    assert!(out.stdout.contains("    fix: repair the syntax"));
    assert!(
        !out.stdout.contains("CODE_ROUTINE_LIMIT"),
        "no rule is judged on it"
    );
    assert!(out.stdout.contains("0 of 1 selected files measured"));
}

#[test]
fn a_file_outside_every_spec_is_reported_as_unowned() {
    let (_temp, root) = project(REPAIRED);
    write(&root, "tools/extra.rs", VIOLATING);
    let out = run_cli_in(
        &root,
        &[
            "code",
            "check",
            "--files",
            "tools/extra.rs",
            "--fail-on",
            "warning",
        ],
    );
    assert_eq!(out.code, 1);
    assert!(
        out.stdout
            .contains("tools/extra.rs:1: WARNING CODE_ROUTINE_LIMIT")
    );
    assert!(
        out.stdout
            .contains("tools/extra.rs: INFO CODE_FILE_UNOWNED")
    );
    assert!(out.stdout.contains("(no spec): 1 files"));
    assert_eq!(
        out.stdout.matches("CODE_FILE_UNOWNED").count(),
        1,
        "once per file"
    );
}

#[test]
fn a_file_without_a_profile_is_listed_as_not_measured() {
    let (_temp, root) = project(REPAIRED);
    write(&root, "notes/plan.txt", "not source code\n");
    let out = run_cli_in(&root, &["code", "check", "--files", "notes/plan.txt"]);
    assert_eq!(out.code, 0);
    assert!(
        out.stdout
            .contains("1 files selected: 0 measured, 0 failed to parse, 1 not measured")
    );
    assert!(out.stdout.contains("0 of 1 selected files measured"));
}

#[test]
fn the_coverage_line_is_always_present() {
    let (_temp, root) = project(REPAIRED);
    let out = run_cli_in(&root, &["code", "check", "--all"]);
    assert_eq!(out.code, 0);
    assert!(
        out.stdout
            .contains("INFO CODE_COVERAGE 2 files selected: 2 measured")
    );
    assert!(out.stdout.contains("before side: none"));
}

#[test]
fn usage_failures_exit_2_and_name_the_choices() {
    let (_temp, root) = project(REPAIRED);
    let cases: &[(&[&str], &str)] = &[
        (&["code", "check"], "exactly one of --all"),
        (
            &["code", "check", "--all", "--spec", "demo"],
            "exactly one of --all",
        ),
        (&["code", "check", "--task", "demo"], "expected <spec>:<id>"),
        (
            &["code", "check", "--task", "demo:9.9"],
            "available: 1, 1.1, 1.2, 2",
        ),
        (
            &["code", "check", "--task", "nope:1.1"],
            "spec `nope` was not found; available:",
        ),
        (&["code", "check", "--spec", "old"], "has no design"),
        (
            &["code", "check", "--files", "missing.rs"],
            "was not found under the project root",
        ),
    ];
    for (args, needle) in cases {
        let out = run_cli_in(&root, args);
        assert_eq!(out.code, 2, "{args:?}: {}", out.stdout);
        assert_eq!(out.stdout, "", "{args:?}");
        assert!(out.stderr.contains(needle), "{args:?}: {}", out.stderr);
        assert_eq!(out.stderr.lines().count(), 1, "{args:?}");
    }
}

#[test]
fn the_json_report_is_complete_and_identical_across_runs() {
    let (_temp, root) = project(VIOLATING);
    let args = ["code", "check", "--spec", "demo", "--format", "json"];
    let first = run_cli_in(&root, &args);
    let second = run_cli_in(&root, &args);
    assert_eq!(first.code, 0);
    assert_eq!(first.stdout, second.stdout, "byte-identical");
    assert!(
        !first.stdout.contains(root.to_str().unwrap()),
        "no absolute path"
    );

    let report: serde_json::Value = serde_json::from_str(&first.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["code"] == "CODE_ROUTINE_LIMIT")
        .unwrap();
    assert_eq!(finding["severity"], "warning");
    assert_eq!(finding["feature"], "demo");
    assert_eq!(finding["file"], "src/demo.rs");
    assert_eq!(finding["line"], 1);
    assert_eq!(finding["tag"], "simplify");
    assert!(
        finding["hint"]
            .as_str()
            .unwrap()
            .starts_with("return early")
    );
    assert_eq!(finding["metric"], "nesting");
    assert_eq!(finding["value"], 4.0);
    assert_eq!(finding["limit"], 3.0);
    assert_eq!(finding["limit_source"], "default");
    assert_eq!(finding["pre_existing"], false);
    assert_eq!(finding["entity"]["qualified_name"], "owner");
    assert_eq!(finding["entity"]["scope"], "routine");
    assert_eq!(finding["components"][0], "Core");
    assert_eq!(finding["tasks"], serde_json::json!(["1.1", "2"]));
    assert_eq!(report["coverage"]["selected"], 2);
    assert_eq!(report["coverage"]["measured"], 2);
    assert_eq!(report["specs"][0]["spec"], "demo");
}

#[test]
fn only_the_requested_output_is_written() {
    let (_temp, root) = project(VIOLATING);
    let before = std::fs::read_dir(&root).unwrap().count();
    let out = run_cli_in(
        &root,
        &[
            "code",
            "check",
            "--all",
            "--format",
            "json",
            "--output",
            "code.json",
        ],
    );
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout, "");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), before + 1);
    assert!(
        std::fs::read_to_string(root.join("code.json"))
            .unwrap()
            .contains("CODE_ROUTINE_LIMIT")
    );
}

#[test]
fn show_highest_reads_as_a_distribution() {
    let (_temp, root) = project(VIOLATING);
    let out = run_cli_in(&root, &["code", "check", "--all", "--show-highest"]);
    assert!(
        out.stdout
            .contains("Highest values (a distribution, not a verdict):")
    );
    assert!(
        out.stdout
            .contains("  routine nesting: 4 at src/demo.rs:1 (owner)")
    );
}
