//! Similar-routine families through the real binary (clone-constraints
//! 1.3, 3.3): a renamed copy is reported, and the report is gone once one
//! routine is kept.

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

/// A routine of eight statements that reads the non-empty lines of a text.
fn reader(name: &str, text: &str, item: &str, out: &str) -> String {
    format!(
        "pub fn {name}(source: &str) -> Vec<String> {{
    let {text} = source.to_string();
    let mut {out} = Vec::new();
    for line in {text}.lines() {{
        let {item} = line.trim();
        if {item}.is_empty() {{
            continue;
        }}
        {out}.push({item}.to_string());
    }}
    {out}.sort();
    {out}
}}
"
    )
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
fn a_renamed_copy_in_another_file_is_reported_until_one_routine_is_kept() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    write(
        &root,
        "src/demo.rs",
        &reader("load_users", "text", "trimmed", "users"),
    );
    write(
        &root,
        "src/ui.rs",
        "pub fn show(count: usize) -> String {\n    format!(\"{count} users\")\n}\n",
    );
    let alone = run_cli_in(&root, CORE_TASK);
    assert_eq!(alone.code, 0, "{}", alone.stdout);
    assert!(!alone.stdout.contains("CODE_SIMILAR_FAMILY"));
    assert!(
        alone
            .stdout
            .contains("similar routines: 1 of 2 routines compared, 0 pairs scored, 0 refused before scoring, 0 families"),
        "{}",
        alone.stdout
    );

    // The agent writes the same routine again under other names.
    write(
        &root,
        "src/ui.rs",
        &reader("read_orders", "content", "entry", "orders"),
    );
    let failing = run_cli_in(&root, CORE_TASK);
    assert_eq!(failing.code, 1, "{}", failing.stdout);
    assert!(
        failing.stdout.contains(
            "src/demo.rs:1: WARNING CODE_SIMILAR_FAMILY routine `load_users` is one of 2 similar routines (weakest similarity 1.00); also `read_orders` at src/ui.rs:1"
        ),
        "{}",
        failing.stdout
    );
    assert!(
        failing.stdout.contains(
            "    delete: keep one routine in the module that owns it and call it from the other 1 places [spec demo, component Core, tasks 1.1, 2]"
        ),
        "{}",
        failing.stdout
    );
    assert!(failing.stdout.contains("1 pairs scored"));

    // One routine is kept and called.
    write(
        &root,
        "src/ui.rs",
        "pub fn read_orders(source: &str) -> Vec<String> {\n    crate::demo::load_users(source)\n}\n",
    );
    let passing = run_cli_in(&root, CORE_TASK);
    assert_eq!(passing.code, 0, "{}", passing.stdout);
    assert!(!passing.stdout.contains("CODE_SIMILAR_FAMILY"));
}

#[test]
fn two_copies_in_one_file_are_to_be_merged() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    let both = format!(
        "{}\n{}",
        reader("load_users", "text", "trimmed", "users"),
        reader("load_groups", "body", "name", "groups")
    );
    write(&root, "src/demo.rs", &both);
    let out = run_cli_in(
        &root,
        &[
            "code",
            "check",
            "--files",
            "src/demo.rs",
            "--format",
            "json",
        ],
    );
    let report: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    let found: Vec<&serde_json::Value> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["code"] == "CODE_SIMILAR_FAMILY")
        .collect();
    assert_eq!(found.len(), 1, "one finding for the family");
    assert_eq!(found[0]["tag"], "shrink");
    assert_eq!(found[0]["line"], 1);
    assert_eq!(found[0]["severity"], "warning");
    assert_eq!(
        found[0]["hint"],
        "merge the 2 routines into one and pass what differs as a parameter"
    );
}

#[test]
fn a_short_routine_and_another_language_are_not_compared() {
    let (_temp, root) = setup_tree(&core_fixture_path("minimal"));
    // Two statements each: below the floor of six.
    let short = "pub fn tiny(a: u32) -> u32 {\n    let b = a + 1;\n    b * 2\n}\n";
    write(&root, "src/demo.rs", short);
    write(&root, "src/ui.rs", &short.replace("tiny", "small"));
    // The same logic in Python next to a Rust routine.
    write(
        &root,
        "tools/load.py",
        "def load_users(source):\n    text = str(source)\n    users = []\n    for line in text.lines():\n        trimmed = line.trim()\n        if trimmed.is_empty():\n            continue\n        users.push(trimmed.to_string())\n    users.sort()\n    return users\n",
    );
    write(
        &root,
        "src/job.rs",
        &reader("load_users", "text", "trimmed", "users"),
    );
    let out = run_cli_in(&root, &["code", "check", "--all", "--fail-on", "warning"]);
    assert!(
        !out.stdout.contains("CODE_SIMILAR_FAMILY"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("similar routines: 2 of 4 routines compared, 0 pairs scored"),
        "{}",
        out.stdout
    );
}
