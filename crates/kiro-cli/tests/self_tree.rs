//! Integration test verifying kiro-cli self-tree determinism.
//!
//! Requirement 13.6: When run over this repository's own `.kiro` tree,
//! `check --all` shall exit with code 0 or 1, never 2, and produce byte-identical
//! output across two runs.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use common::{run_cli, workspace_root};

#[test]
fn self_tree_check_all_deterministic() {
    let root = workspace_root();
    let root_str = root.to_str().unwrap();

    let out1 = run_cli(&["--project", root_str, "check", "--all"]);
    let out2 = run_cli(&["--project", root_str, "check", "--all"]);

    assert!(
        out1.code == 0 || out1.code == 1,
        "self-tree check exited with code {} (expected 0 or 1): stderr:\n{}",
        out1.code,
        out1.stderr
    );
    assert_eq!(
        out1.code, out2.code,
        "exit code differed between runs: run 1 = {}, run 2 = {}",
        out1.code, out2.code
    );
    assert_eq!(
        out1.stdout, out2.stdout,
        "stdout was not byte-identical between runs"
    );
    assert_eq!(
        out1.stderr, out2.stderr,
        "stderr was not byte-identical between runs"
    );
}
