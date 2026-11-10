//! CLI compatibility contract (cli-contracts 1.2, 1.7, 2.2, 2.3).
//!
//! Each case runs the real binary and compares exit code, stdout and stderr
//! with a record captured from the binary before the shared argument library
//! existed. A migration must leave every record unchanged.
//!
//! To re-capture after an intended, reviewed change:
//! `KIRO_UPDATE_COMPAT=1 cargo test -p kiro-cli --test cli_standard`.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use common::{cli_fixture_path, core_fixture_path, run_cli};

/// One legacy invocation: record name, fixture tree, arguments after
/// `--project <tree>` (or `--root <tree>` when `root_alias` is set).
struct Case {
    name: &'static str,
    tree: &'static str,
    root_alias: bool,
    args: &'static [&'static str],
}

const fn case(name: &'static str, tree: &'static str, args: &'static [&'static str]) -> Case {
    Case {
        name,
        tree,
        root_alias: false,
        args,
    }
}

const CASES: &[Case] = &[
    case("check-good-text", "checks/good", &["check", "--all"]),
    case(
        "check-good-json",
        "checks/good",
        &["check", "--all", "--format", "json"],
    ),
    case(
        "check-good-explicit-defaults",
        "checks/good",
        &["check", "--all", "--format", "text", "--fail-on", "error"],
    ),
    case("check-bad-text", "checks/bad", &["check", "--all"]),
    case(
        "check-bad-json",
        "checks/bad",
        &["check", "--all", "--format", "json"],
    ),
    case(
        "check-bad-feature-warning",
        "checks/bad",
        &["check", "--feature", "broken", "--fail-on", "warning"],
    ),
    case(
        "check-bad-checks-subset",
        "checks/bad",
        &["check", "--all", "--checks", "ears", "--patterns"],
    ),
    Case {
        name: "check-root-alias",
        tree: "checks/good",
        root_alias: true,
        args: &["check", "--all"],
    },
    case("check-no-selection", "checks/good", &["check"]),
    case(
        "check-unknown-feature",
        "checks/good",
        &["check", "--feature", "nope"],
    ),
    case(
        "check-bad-severity",
        "checks/good",
        &["check", "--all", "--severity", "NOPE=info"],
    ),
    case("status", "minimal", &["status", "demo"]),
    case("graph-default", "minimal", &["graph", "demo"]),
    case("graph-table", "minimal", &["graph", "demo", "--table"]),
    case("tasks-default", "minimal", &["tasks", "demo"]),
    case("tasks-table", "minimal", &["tasks", "demo", "--table"]),
    case("vocab-validate", "checks/good", &["vocab", "validate"]),
    case(
        "vocab-validate-json",
        "checks/good",
        &["vocab", "validate", "--format", "json"],
    ),
];

fn record(case: &Case) -> String {
    let tree = core_fixture_path(case.tree);
    let tree = tree.to_str().unwrap();
    let flag = if case.root_alias {
        "--root"
    } else {
        "--project"
    };
    let mut args = vec![flag, tree];
    args.extend_from_slice(case.args);
    let out = run_cli(&args);
    // The only place a tree path can appear is a usage message; keep the
    // record independent of where the repository is checked out.
    let normalize = |text: &str| text.replace(tree, "<tree>").replace('\\', "/");
    format!(
        "exit={}\n--- stdout\n{}--- stderr\n{}",
        out.code,
        normalize(&out.stdout),
        normalize(&out.stderr)
    )
}

#[test]
fn legacy_invocations_are_unchanged() {
    let update = std::env::var_os("KIRO_UPDATE_COMPAT").is_some();
    let dir = cli_fixture_path("compat");
    if update {
        std::fs::create_dir_all(&dir).unwrap();
    }
    let mut failures = Vec::new();
    for case in CASES {
        let actual = record(case);
        let path = dir.join(format!("{}.txt", case.name));
        if update {
            std::fs::write(&path, &actual).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing compatibility record {}", path.display()))
            .replace("\r\n", "\n");
        if expected != actual {
            failures.push(format!(
                "== {}\n-- expected\n{expected}\n-- actual\n{actual}",
                case.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} legacy invocation(s) changed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_record_has_a_case_and_every_case_a_record() {
    let dir = cli_fixture_path("compat");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut declared: Vec<String> = CASES
        .iter()
        .map(|case| format!("{}.txt", case.name))
        .collect();
    declared.sort();
    assert_eq!(on_disk, declared);
}

#[test]
fn root_alias_equals_project() {
    let dir = cli_fixture_path("compat");
    let project = std::fs::read_to_string(dir.join("check-good-text.txt")).unwrap();
    let root = std::fs::read_to_string(dir.join("check-root-alias.txt")).unwrap();
    assert_eq!(project, root);
}
