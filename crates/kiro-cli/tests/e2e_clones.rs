//! Duplicate blocks through the real binary (clone-constraints 1.1, 3.3):
//! a copy is introduced, the check reports it with the original's
//! location, the copy is removed, and the check passes.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use std::path::Path;

use common::{core_fixture_path, run_cli_in, setup_tree};

/// Fourteen code lines that will stand in two files.
const SHARED: &str = "pub fn normalize(parts: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        let cleaned = lower.replace('\\\\', \"/\");
        out.push(cleaned);
    }
    out.sort();
    out.dedup();
    out
}
";

const UI_ONLY: &str = "pub fn show(owner: Option<usize>) -> String {
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

const CHECK: &[&str] = &[
    "code",
    "check",
    "--task",
    "demo:1.2",
    "--fail-on",
    "warning",
];

#[test]
fn an_introduced_copy_is_reported_and_its_removal_passes() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    // The original lives in Core's file; task 1.2 owns the Ui file.
    write(&root, "src/demo.rs", SHARED);
    write(&root, "src/ui.rs", UI_ONLY);
    let clean = run_cli_in(&root, CHECK);
    assert_eq!(clean.code, 0, "{}", clean.stdout);
    assert!(
        clean
            .stdout
            .contains("duplicate blocks: 2 files searched, 0 blocks")
    );

    // The agent writes the same routine again, with other spacing and a comment.
    let copy = SHARED
        .replace(
            "    let mut out",
            "    // collect the parts\n    let mut out",
        )
        .replace("out.push(cleaned);", "out.push( cleaned );");
    write(&root, "src/ui.rs", &format!("{UI_ONLY}\n{copy}"));

    let failing = run_cli_in(&root, CHECK);
    assert_eq!(failing.code, 1, "{}", failing.stdout);
    assert!(
        failing.stdout.contains(
            "src/ui.rs:8: WARNING CODE_DUPLICATE_BLOCK lines 8-23 (15 code lines) also stand at src/demo.rs:1-15"
        ),
        "{}",
        failing.stdout
    );
    assert!(
        failing.stdout.contains(
            "    delete: keep one copy: move the block into one routine and call it from src/demo.rs:1-15 [spec demo, component Ui, tasks 1.2]"
        ),
        "{}",
        failing.stdout
    );
    // Only the selected side is reported for this task.
    assert_eq!(failing.stdout.matches("CODE_DUPLICATE_BLOCK").count(), 1);

    // Both sides when both files are selected.
    let both = run_cli_in(&root, &["code", "check", "--spec", "demo"]);
    assert_eq!(both.stdout.matches("CODE_DUPLICATE_BLOCK").count(), 2);

    // The repair: call the original instead of copying it.
    write(&root, "src/ui.rs", UI_ONLY);
    let passing = run_cli_in(&root, CHECK);
    assert_eq!(passing.code, 0, "{}", passing.stdout);
    assert!(!passing.stdout.contains("CODE_DUPLICATE_BLOCK"));
}

#[test]
fn a_copy_shorter_than_twelve_lines_is_not_reported() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    write(&root, "src/demo.rs", UI_ONLY);
    write(&root, "src/ui.rs", UI_ONLY);
    let out = run_cli_in(&root, &["code", "check", "--all", "--fail-on", "warning"]);
    assert_eq!(out.code, 0, "{}", out.stdout);
    assert!(!out.stdout.contains("CODE_DUPLICATE_BLOCK"));
}

#[test]
fn the_json_report_carries_the_duplicate_with_tag_and_owner() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    write(&root, "src/demo.rs", SHARED);
    write(&root, "src/ui.rs", SHARED);
    let out = run_cli_in(
        &root,
        &["code", "check", "--task", "demo:1.2", "--format", "json"],
    );
    let report: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["code"] == "CODE_DUPLICATE_BLOCK")
        .unwrap();
    assert_eq!(finding["file"], "src/ui.rs");
    assert_eq!(finding["line"], 1);
    assert_eq!(finding["tag"], "delete");
    assert_eq!(finding["feature"], "demo");
    assert_eq!(finding["components"][0], "Ui");
    assert_eq!(finding["severity"], "warning");
}
