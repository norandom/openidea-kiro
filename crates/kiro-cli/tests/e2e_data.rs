//! Positional data access through the real binary (code-locality-data
//! 3.1, 3.3, 4.4): the access is reported, a name replaces the position,
//! the finding is gone.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use std::path::Path;

use common::{core_fixture_path, run_cli_in, setup_tree};

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

const CORE_TASK: &[&str] = &[
    "code",
    "check",
    "--task",
    "demo:1.1",
    "--fail-on",
    "warning",
];

#[test]
fn a_column_by_position_is_reported_until_it_is_selected_by_name() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    write(
        &root,
        "src/demo.rs",
        "pub fn price(frame: &Frame) -> &Column {\n    frame.select_at_idx(3)\n}\n",
    );
    let failing = run_cli_in(&root, CORE_TASK);
    assert_eq!(failing.code, 1, "{}", failing.stdout);
    assert!(
        failing.stdout.contains(
            "src/demo.rs:2: WARNING CODE_POSITIONAL_ACCESS `frame.select_at_idx(3)` addresses a dataframe column by position 3"
        ),
        "{}",
        failing.stdout
    );
    assert!(
        failing.stdout.contains(
            "    name: select the column by its role name, or give the row a typed record [spec demo, component Core, tasks 1.1, 2]"
        ),
        "{}",
        failing.stdout
    );
    // The coverage line says what was not checked.
    assert!(
        failing.stdout.contains(
            "data access: 1 files searched, 1 positional accesses; column names were not grounded (no data topic is configured)"
        ),
        "{}",
        failing.stdout
    );

    write(
        &root,
        "src/demo.rs",
        "pub fn price(frame: &Frame) -> &Column {\n    frame.column(\"price\")\n}\n",
    );
    let passing = run_cli_in(&root, CORE_TASK);
    assert_eq!(passing.code, 0, "{}", passing.stdout);
    assert!(!passing.stdout.contains("CODE_POSITIONAL_ACCESS"));
}

#[test]
fn python_positions_are_reported_for_the_named_file_only() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    write(
        &root,
        "tools/report.py",
        "def totals(frame, rows):\n    price = frame.iloc[:, 3]\n    return [row[2] for row in rows], price\n",
    );
    write(
        &root,
        "tools/other.py",
        "def last(frame):\n    return frame.iloc[:, -1]\n",
    );
    let out = run_cli_in(
        &root,
        &[
            "code",
            "check",
            "--files",
            "tools/report.py",
            "--format",
            "json",
        ],
    );
    let report: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    let found: Vec<(&str, u64, &str)> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["code"] == "CODE_POSITIONAL_ACCESS")
        .map(|finding| {
            (
                finding["message"].as_str().unwrap(),
                finding["line"].as_u64().unwrap(),
                finding["tag"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (
                "`frame.iloc[:, 3]` addresses a dataframe column by position 3",
                2,
                "name"
            ),
            (
                "`row[2]` addresses a row record field by position 2",
                3,
                "name"
            ),
        ]
    );
}
