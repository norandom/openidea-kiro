//! The ratchet through the real binary and real Git repositories
//! (code-constraints 5, 11.6, 13.1, 13.2, 12.2).
//!
//! Only regression and new breaches block. Each test builds its own
//! repository in a temporary directory.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use std::path::Path;
use std::process::Command;

use common::{core_fixture_path, run_cli_in, setup_tree};

/// Nesting 4 (limit 3): existing debt.
const DEBT: &str = "pub fn owner(paths: &[&str], wanted: &str) -> Option<usize> {
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

/// The same routine one level deeper: nesting 5.
const WORSE: &str = "pub fn owner(paths: &[&str], wanted: &str) -> Option<usize> {
    for (index, path) in paths.iter().enumerate() {
        if !path.is_empty() {
            let mut rest = *path;
            while let Some(stripped) = rest.strip_prefix('/') {
                if stripped == wanted {
                    if index > 0 {
                        return Some(index);
                    }
                }
                rest = stripped;
            }
        }
    }
    None
}
";

/// The debt removed by extraction: nesting 2.
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

const HELPER: &str = "
pub fn count(paths: &[&str]) -> usize {
    paths.len()
}
";

const UI: &str = "pub fn show(owner: Option<usize>) -> String {
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

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(root)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(["-c", "core.autocrlf=false", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A repository with the minimal spec tree and one commit holding the debt.
fn repository() -> (tempfile::TempDir, std::path::PathBuf) {
    let (temp, root) = setup_tree(&core_fixture_path("minimal"));
    write(&root, "src/demo.rs", DEBT);
    write(&root, "src/ui.rs", UI);
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "initial"]);
    (temp, root)
}

const CHANGED: &[&str] = &["code", "check", "--changed", "HEAD"];
const CHANGED_STRICT: &[&str] = &["code", "check", "--changed", "HEAD", "--fail-on", "warning"];

#[test]
fn existing_debt_does_not_block_a_change_that_leaves_it_alone() {
    let (_temp, root) = repository();
    // Nothing changed: nothing is selected.
    let untouched = run_cli_in(&root, CHANGED_STRICT);
    assert_eq!(untouched.code, 0, "{}", untouched.stdout);
    assert!(untouched.stdout.contains("0 files selected"));

    // A routine is added next to the debt.
    write(&root, "src/demo.rs", &format!("{DEBT}{HELPER}"));
    let out = run_cli_in(&root, CHANGED_STRICT);
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(
        out.stdout.contains(
            "WARNING CODE_ROUTINE_LIMIT routine `owner`: nesting is 4, limit 3 (default); pre-existing, was 4 before"
        ),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("(1 pre-existing, not blocking)"));
    assert!(out.stdout.contains("before side: revision "));
    assert!(out.stdout.contains("fail-on warning: pass"));
    // The added routine grew the file by one statement and three lines.
    assert!(
        out.stdout.contains("net: +1 statements, +3 lines"),
        "{}",
        out.stdout
    );
}

#[test]
fn a_regression_blocks_even_at_the_default_threshold_and_a_repair_passes() {
    let (_temp, root) = repository();
    write(&root, "src/demo.rs", WORSE);

    let failing = run_cli_in(&root, CHANGED);
    assert_eq!(failing.code, 1, "{}", failing.stdout);
    assert!(
        failing.stdout.contains(
            "ERROR CODE_RATCHET routine `owner`: nesting rose from 4 to 5, limit 3 (default)"
        ),
        "{}",
        failing.stdout
    );
    assert!(
        failing
            .stdout
            .contains("[spec demo, component Core, tasks 1.1, 2]")
    );
    assert!(failing.stdout.contains("fail-on error: fail"));

    // Without a before side the same change is only a warning: the journey
    // depends on the ratchet.
    let no_ratchet = run_cli_in(&root, &["code", "check", "--files", "src/demo.rs"]);
    assert_eq!(no_ratchet.code, 0);
    assert!(!no_ratchet.stdout.contains("CODE_RATCHET"));

    write(&root, "src/demo.rs", REPAIRED);
    let passing = run_cli_in(&root, CHANGED_STRICT);
    assert_eq!(passing.code, 0, "{}", passing.stdout);
    assert!(!passing.stdout.contains("CODE_RATCHET"));
    assert!(!passing.stdout.contains("CODE_ROUTINE_LIMIT"));
}

#[test]
fn the_summary_names_what_worsened_and_what_improved() {
    let (_temp, root) = repository();
    write(&root, "src/demo.rs", WORSE);
    let worse = run_cli_in(&root, &["code", "check", "--changed", "HEAD", "--summary"]);
    assert_eq!(worse.code, 1);
    let lines: Vec<&str> = worse.stdout.lines().collect();
    assert_eq!(lines[0], "kiro code check: FAIL (fail-on error)");
    assert!(lines[1].starts_with("before side: revision "));
    // Nesting rose above its limit (the blocking error); cyclomatic, code
    // lines and statements rose within theirs (three warnings).
    assert_eq!(lines[2], "findings: 1 blocking, 3 warnings, 0 pre-existing");
    assert!(worse.stdout.contains("worsened: 4\n"), "{}", worse.stdout);
    assert!(
        worse
            .stdout
            .contains("  src/demo.rs:1 owner nesting 4 -> 5 (limit 3)\n")
    );
    assert!(worse.stdout.contains("improved: 0\n"));
    assert!(lines.len() <= 25);
    assert!(worse.stdout.is_ascii());
    assert!(!worse.stdout.contains('\r'));

    write(&root, "src/demo.rs", REPAIRED);
    let better = run_cli_in(&root, &["code", "check", "--changed", "HEAD", "--summary"]);
    assert_eq!(better.code, 0);
    assert!(
        better
            .stdout
            .starts_with("kiro code check: PASS (fail-on error)\n")
    );
    assert!(
        better
            .stdout
            .contains("  src/demo.rs:1 owner nesting 4 -> 2 (limit 3)\n")
    );
    assert!(better.stdout.contains("worsened: 0\n"));
}

#[test]
fn a_routine_moved_to_a_new_file_keeps_its_debt_status() {
    let (_temp, root) = repository();
    // Split: the routine with the debt moves, unchanged, to its own file.
    write(&root, "src/demo.rs", HELPER);
    write(&root, "src/owner.rs", DEBT);
    git(&root, &["add", "-A"]);

    let out = run_cli_in(&root, CHANGED_STRICT);
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(
        out.stdout.contains(
            "src/owner.rs:1: WARNING CODE_ROUTINE_LIMIT routine `owner`: nesting is 4, limit 3 (default); pre-existing, was 4 before"
        ),
        "{}",
        out.stdout
    );
    let json = run_cli_in(
        &root,
        &["code", "check", "--changed", "HEAD", "--format", "json"],
    );
    let report: serde_json::Value = serde_json::from_str(&json.stdout).unwrap();
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["code"] == "CODE_ROUTINE_LIMIT")
        .unwrap();
    assert_eq!(finding["match"], "moved");
    assert_eq!(finding["pre_existing"], true);
    assert_eq!(finding["before"], 4.0);

    // An edited copy is a new routine: its breach blocks at this threshold.
    write(
        &root,
        "src/owner.rs",
        &DEBT.replace("stripped == wanted", "wanted == stripped"),
    );
    let edited = run_cli_in(&root, CHANGED_STRICT);
    assert_eq!(edited.code, 1, "{}", edited.stdout);
    assert!(!edited.stdout.contains("pre-existing"));
}

#[test]
fn renamed_and_deleted_files_are_accounted() {
    let (_temp, root) = repository();
    // Rename the file with the debt (content unchanged) and delete the other.
    git(&root, &["mv", "src/demo.rs", "src/core.rs"]);
    git(&root, &["rm", "-q", "src/ui.rs"]);

    let out = run_cli_in(&root, CHANGED_STRICT);
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(
        out.stdout
            .contains("src/core.rs:1: WARNING CODE_ROUTINE_LIMIT")
    );
    assert!(out.stdout.contains("pre-existing, was 4 before"));
    // The deleted file held one routine of one statement on six lines.
    assert!(
        out.stdout.contains("net: -1 statements, -6 lines"),
        "{}",
        out.stdout
    );
}

#[test]
fn an_unknown_revision_is_a_usage_failure_that_fetches_nothing() {
    let (_temp, root) = repository();
    let out = run_cli_in(&root, &["code", "check", "--changed", "no-such-revision"]);
    assert_eq!(out.code, 2);
    assert_eq!(out.stdout, "");
    assert!(
        out.stderr
            .contains("revision `no-such-revision` was not found")
    );
    assert_eq!(out.stderr.lines().count(), 1);

    let option = run_cli_in(&root, &["code", "check", "--changed=--output=x"]);
    assert_eq!(option.code, 2);
    assert!(option.stderr.contains("is not a revision"));
}

#[test]
fn the_baseline_is_deterministic_and_makes_only_regression_block() {
    let (_temp, root) = repository();
    let first = run_cli_in(&root, &["code", "baseline"]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(first.stdout, "");
    let path = root.join(".kiro").join("code-baseline.json");
    let captured = std::fs::read_to_string(&path).unwrap();
    assert!(captured.contains("\"schema_version\": 2"));
    assert!(
        !captured.contains(root.to_str().unwrap()),
        "no absolute path"
    );
    run_cli_in(&root, &["code", "baseline"]);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        captured,
        "byte-identical"
    );

    // Against the baseline the existing debt does not block.
    let all = ["code", "check", "--all", "--fail-on", "warning"];
    let clean = run_cli_in(&root, &all);
    assert_eq!(clean.code, 0, "{}", clean.stdout);
    assert!(
        clean
            .stdout
            .contains("before side: baseline .kiro/code-baseline.json")
    );
    assert!(clean.stdout.contains("pre-existing, was 4 before"));

    // A regression blocks at the default threshold.
    write(&root, "src/demo.rs", WORSE);
    let worse = run_cli_in(&root, &["code", "check", "--all"]);
    assert_eq!(worse.code, 1, "{}", worse.stdout);
    assert!(worse.stdout.contains("ERROR CODE_RATCHET"));

    // A named revision wins over the baseline.
    let by_revision = run_cli_in(&root, CHANGED);
    assert!(by_revision.stdout.contains("before side: revision "));
}

#[test]
fn a_baseline_needs_a_complete_measurement() {
    let (_temp, root) = repository();
    write(&root, "src/demo.rs", "pub fn broken( {\n");
    let out = run_cli_in(&root, &["code", "baseline"]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("these files do not parse: src/demo.rs"));
    assert!(!root.join(".kiro").join("code-baseline.json").exists());
}

#[test]
fn a_baseline_of_another_schema_is_not_compared() {
    let (_temp, root) = repository();
    write(
        &root,
        ".kiro/code-baseline.json",
        "{\"schema_version\": 1, \"config_digest\": \"x\", \"facts\": {\"schema_version\": 1, \"observations\": [], \"coverage\": []}}\n",
    );
    let out = run_cli_in(&root, &["code", "check", "--all", "--fail-on", "warning"]);
    assert_eq!(out.code, 1, "the debt is judged on the limit alone");
    assert!(
        out.stdout
            .contains("has schema version 1 and this build reads 2")
    );
    assert!(!out.stdout.contains("pre-existing"));

    write(&root, ".kiro/code-baseline.json", "not json");
    let broken = run_cli_in(&root, &["code", "check", "--all"]);
    assert_eq!(broken.code, 2);
    assert!(broken.stderr.contains("is not a baseline document"));
}
