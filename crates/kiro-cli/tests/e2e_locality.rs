//! Scattered definitions and scattered state through the real binary
//! (code-locality-data 1, 2, 4.4): the finding appears, the code is
//! consolidated, the finding is gone.

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

fn project() -> (tempfile::TempDir, std::path::PathBuf) {
    setup_tree(&core_fixture_path("minimal"))
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
fn a_constant_repeated_in_four_files_is_reported_until_it_is_defined_once() {
    let (_temp, root) = project();
    let with_constant = |body: &str| format!("pub const TIMEOUT_MS: u64 = 30_000;\n\n{body}");
    write(
        &root,
        "src/demo.rs",
        &with_constant("pub fn core() -> u64 {\n    TIMEOUT_MS\n}\n"),
    );
    write(
        &root,
        "src/ui.rs",
        &with_constant("pub fn ui() -> u64 {\n    TIMEOUT_MS\n}\n"),
    );
    write(
        &root,
        "src/net.rs",
        &with_constant("pub fn net() -> u64 {\n    TIMEOUT_MS\n}\n"),
    );

    // Three files: within the allowance.
    let three = run_cli_in(&root, CORE_TASK);
    assert_eq!(three.code, 0, "{}", three.stdout);
    assert!(!three.stdout.contains("CODE_SCATTERED_DEFINITION"));

    // The agent writes it a fourth time.
    write(
        &root,
        "src/job.rs",
        &with_constant("pub fn job() -> u64 {\n    TIMEOUT_MS\n}\n"),
    );
    let failing = run_cli_in(&root, CORE_TASK);
    assert_eq!(failing.code, 1, "{}", failing.stdout);
    assert!(
        failing.stdout.contains(
            "src/demo.rs:1: WARNING CODE_SCATTERED_DEFINITION `TIMEOUT_MS = 30_000` is defined in 4 files; also in src/job.rs, src/net.rs, src/ui.rs"
        ),
        "{}",
        failing.stdout
    );
    assert!(
        failing.stdout.contains(
            "    collect: define `TIMEOUT_MS` once in the module that owns it and import it in the other 3 files [spec demo, component Core, tasks 1.1, 2]"
        ),
        "{}",
        failing.stdout
    );
    assert!(
        failing
            .stdout
            .contains("locality: 4 files searched, 4 module-level definitions")
    );

    // Consolidation: one definition, the others import it.
    let importing = |body: &str| format!("use crate::demo::TIMEOUT_MS;\n\n{body}");
    write(
        &root,
        "src/ui.rs",
        &importing("pub fn ui() -> u64 {\n    TIMEOUT_MS\n}\n"),
    );
    write(
        &root,
        "src/net.rs",
        &importing("pub fn net() -> u64 {\n    TIMEOUT_MS\n}\n"),
    );
    write(
        &root,
        "src/job.rs",
        &importing("pub fn job() -> u64 {\n    TIMEOUT_MS\n}\n"),
    );
    let passing = run_cli_in(&root, CORE_TASK);
    assert_eq!(passing.code, 0, "{}", passing.stdout);
    assert!(!passing.stdout.contains("CODE_SCATTERED_DEFINITION"));
}

#[test]
fn a_state_spelled_out_in_four_files_is_reported_as_information() {
    let (_temp, root) = project();
    let branch = |name: &str, value: &str| {
        format!(
            "pub fn {name}(status: &str) -> bool {{\n    if status == \"{value}\" {{\n        return true;\n    }}\n    false\n}}\n"
        )
    };
    write(&root, "src/demo.rs", &branch("core", "open"));
    write(&root, "src/ui.rs", &branch("ui", "paid"));
    write(&root, "src/net.rs", &branch("net", "open"));
    write(&root, "src/job.rs", &branch("job", "cancelled"));

    let out = run_cli_in(&root, CORE_TASK);
    assert_eq!(out.code, 0, "information does not block: {}", out.stdout);
    assert!(
        out.stdout.contains(
            "src/demo.rs:2: INFO CODE_SCATTERED_STATE `status` is compared with 3 literal values (\"cancelled\", \"open\", \"paid\") in 4 files; also in src/job.rs, src/net.rs, src/ui.rs"
        ),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("    collect: declare the values of `status` once")
    );

    // The decisions move into one place: an enumeration with its own methods.
    write(
        &root,
        "src/demo.rs",
        "pub enum Status {\n    Open,\n    Paid,\n    Cancelled,\n}\n\nimpl Status {\n    pub fn parse(text: &str) -> Option<Self> {\n        match text {\n            \"open\" => Some(Self::Open),\n            \"paid\" => Some(Self::Paid),\n            \"cancelled\" => Some(Self::Cancelled),\n            _ => None,\n        }\n    }\n}\n",
    );
    for file in ["src/ui.rs", "src/net.rs", "src/job.rs"] {
        write(
            &root,
            file,
            "pub fn is_open(status: &crate::demo::Status) -> bool {\n    matches!(status, crate::demo::Status::Open)\n}\n",
        );
    }
    let consolidated = run_cli_in(&root, &["code", "check", "--all", "--fail-on", "warning"]);
    assert!(
        !consolidated.stdout.contains("CODE_SCATTERED_STATE"),
        "{}",
        consolidated.stdout
    );
}

#[test]
fn the_json_report_carries_the_locality_findings() {
    let (_temp, root) = project();
    for file in ["src/demo.rs", "src/ui.rs", "src/a.rs", "src/b.rs"] {
        write(&root, file, "pub const RETRIES: u8 = 3;\n");
    }
    let out = run_cli_in(
        &root,
        &["code", "check", "--spec", "demo", "--format", "json"],
    );
    let report: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    let found: Vec<&serde_json::Value> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["code"] == "CODE_SCATTERED_DEFINITION")
        .collect();
    assert_eq!(found.len(), 2, "the two files the spec maps");
    assert_eq!(found[0]["tag"], "collect");
    assert_eq!(found[0]["feature"], "demo");
    assert_eq!(found[0]["severity"], "warning");
}
