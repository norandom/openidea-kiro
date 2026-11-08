//! Integration test verifying that every CLI option named in `docs/cli.md`
//! actually exists in `kiro-cli <command> --help`.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

mod common;

use common::{run_cli, workspace_root};
use std::fs;

#[test]
fn docs_cli_options_match_help() {
    let docs_path = workspace_root().join("docs").join("cli.md");
    let content = fs::read_to_string(&docs_path).unwrap();

    // Extract all option names starting with `--` documented in docs/cli.md
    let mut documented_options: Vec<&str> = content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("- `--") {
                let end = rest.find(['`', ' ', '<'])?;
                Some(&trimmed[3..5 + end])
            } else {
                None
            }
        })
        .collect();

    documented_options.sort_unstable();
    documented_options.dedup();

    assert!(
        !documented_options.is_empty(),
        "failed to parse any options from docs/cli.md"
    );

    // Collect all help outputs from top-level and subcommands
    let mut all_help = String::new();
    all_help.push_str(&run_cli(&["--help"]).stdout);
    all_help.push_str(&run_cli(&["check", "--help"]).stdout);
    all_help.push_str(&run_cli(&["status", "--help"]).stdout);
    all_help.push_str(&run_cli(&["graph", "--help"]).stdout);
    all_help.push_str(&run_cli(&["tasks", "--help"]).stdout);
    all_help.push_str(&run_cli(&["vocab", "validate", "--help"]).stdout);
    all_help.push_str(&run_cli(&["vocab", "import", "--help"]).stdout);

    for opt in documented_options {
        assert!(
            all_help.contains(opt),
            "option `{}` from docs/cli.md was not found in any `kiro-cli --help` output",
            opt
        );
    }
}
