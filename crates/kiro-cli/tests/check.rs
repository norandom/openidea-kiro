//! Integration tests for `kiro-cli check` gate command.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::{cli_fixture_path, core_fixture_path, run_cli, setup_tree};

#[test]
fn check_known_good_tree_exits_clean() {
    let good_tree = core_fixture_path("checks/good");
    let (_temp, path) = setup_tree(&good_tree);
    let path_str = path.to_str().unwrap();

    let out = run_cli(&["--project", path_str, "check", "--all"]);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert!(out.stdout.contains("fail-on error: pass"));
    assert!(out.stdout.contains("Checked 1 specs: 0 errors, 0 warnings, 3 info"));
}

#[test]
fn check_known_bad_tree_exits_findings() {
    let bad_tree = core_fixture_path("checks/bad");
    let (_temp, path) = setup_tree(&bad_tree);
    let path_str = path.to_str().unwrap();

    let out = run_cli(&["--project", path_str, "check", "--all"]);
    assert_eq!(out.code, 1, "stderr: {}", out.stderr);
    assert!(out.stdout.contains("CONTRADICTION"));
    assert!(out.stdout.contains("fail-on error: fail"));
}

#[test]
fn check_warnings_tree_fail_on_default_vs_warning() {
    let warnings_tree = cli_fixture_path("trees/warnings");
    let (_temp, path) = setup_tree(&warnings_tree);
    let path_str = path.to_str().unwrap();

    // Default --fail-on error exits 0
    let out_default = run_cli(&["--project", path_str, "check", "--all"]);
    assert_eq!(out_default.code, 0, "stderr: {}", out_default.stderr);
    assert!(out_default.stdout.contains("fail-on error: pass"));

    // --fail-on warning exits 1
    let out_warn = run_cli(&["--project", path_str, "check", "--all", "--fail-on", "warning"]);
    assert_eq!(out_warn.code, 1, "stderr: {}", out_warn.stderr);
    assert!(out_warn.stdout.contains("fail-on warning: fail"));
}

#[test]
fn check_missing_root_unknown_feature_and_selection_errors() {
    let good_tree = core_fixture_path("checks/good");
    let (_temp, path) = setup_tree(&good_tree);
    let path_str = path.to_str().unwrap();

    // 1. Missing root directory
    let missing_root = path.join("nonexistent_subdir");
    let out_missing = run_cli(&[
        "--project",
        missing_root.to_str().unwrap(),
        "check",
        "--all",
    ]);
    assert_eq!(out_missing.code, 2);
    assert!(out_missing.stderr.contains("is not a directory"));

    // 2. Unknown feature
    let out_unknown = run_cli(&["--project", path_str, "check", "--feature", "nope"]);
    assert_eq!(out_unknown.code, 2);
    assert!(out_unknown.stderr.contains("spec `nope` was not found"));
    assert!(out_unknown.stderr.contains("discovered: clean"));

    // 3. Neither --all nor --feature
    let out_none = run_cli(&["--project", path_str, "check"]);
    assert_eq!(out_none.code, 2);
    assert!(out_none.stderr.contains("specify either --all or one or more --feature arguments"));

    // 4. Both --all and --feature
    let out_both = run_cli(&[
        "--project",
        path_str,
        "check",
        "--all",
        "--feature",
        "clean",
    ]);
    assert_eq!(out_both.code, 2);
    assert!(out_both.stderr.contains("specify either --all or one or more --feature arguments, not both"));
}

#[test]
fn check_severity_override_and_disabled_checks() {
    let bad_tree = core_fixture_path("checks/bad");
    let (_temp, path) = setup_tree(&bad_tree);
    let path_str = path.to_str().unwrap();

    // In bad tree, turning EARS errors into info
    let out_override = run_cli(&[
        "--project",
        path_str,
        "check",
        "--all",
        "--severity",
        "EARS_NO_SHALL=info",
    ]);
    assert!(out_override.stdout.contains("INFO EARS_NO_SHALL"));

    // Enabling only ears check: terms and contradictions skipped
    let out_ears_only = run_cli(&[
        "--project",
        path_str,
        "check",
        "--all",
        "--checks",
        "ears",
    ]);
    assert!(out_ears_only.stdout.contains("Terms: skipped (disabled by configuration)"));
    assert!(out_ears_only.stdout.contains("Contradictions: skipped (disabled by configuration)"));
}

#[test]
fn check_json_output_schema_and_keys() {
    let good_tree = core_fixture_path("checks/good");
    let (_temp, path) = setup_tree(&good_tree);
    let path_str = path.to_str().unwrap();

    let out = run_cli(&["--project", path_str, "check", "--all", "--format", "json"]);
    assert_eq!(out.code, 0);

    let val: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(val["schema_version"], "1");
    assert_eq!(val["verdict"], "pass");
    assert_eq!(val["fail_on"], "error");

    let findings = val["findings"].as_array().unwrap();
    assert!(!findings.is_empty());
    let first = &findings[0];
    let obj = first.as_object().unwrap();
    // Exactly 6 documented keys
    assert_eq!(obj.len(), 6);
    assert!(obj.contains_key("severity"));
    assert!(obj.contains_key("code"));
    assert!(obj.contains_key("feature"));
    assert!(obj.contains_key("file"));
    assert!(obj.contains_key("line"));
    assert!(obj.contains_key("message"));

    // No timestamp
    assert!(!out.stdout.contains("timestamp"));
    assert!(!out.stdout.contains("created_at"));
}

#[test]
fn check_vocabulary_findings_printed_once_and_determinism() {
    let vocab_tree = cli_fixture_path("trees/vocab-findings");
    let (_temp, path) = setup_tree(&vocab_tree);
    let path_str = path.to_str().unwrap();

    let out1 = run_cli(&["--project", path_str, "check", "--all"]);
    let out2 = run_cli(&["--project", path_str, "check", "--all"]);

    // Deterministic byte-for-byte output
    assert_eq!(out1.stdout, out2.stdout);
    assert_eq!(out1.stderr, out2.stderr);
    assert_eq!(out1.code, out2.code);

    // TERM_NO_DEFINITION printed in vocabulary section only once
    assert_eq!(out1.stdout.matches("TERM_NO_DEFINITION").count(), 1);
}

#[test]
fn check_verbose_logging_failed_read() {
    let warnings_tree = cli_fixture_path("trees/warnings");
    let (_temp, path) = setup_tree(&warnings_tree);
    let path_str = path.to_str().unwrap();

    // Without --verbose: no read refused/failed lines on stderr
    let out_normal = run_cli(&["--project", path_str, "check", "--all"]);
    assert!(!out_normal.stderr.contains("read refused"));
    assert!(!out_normal.stderr.contains("read failed"));

    // With --verbose: prints discovered specs and read failed line for missing design.md
    let out_verbose = run_cli(&["--project", path_str, "--verbose", "check", "--all"]);
    assert!(out_verbose.stderr.contains("discovered specs: late"));
    assert!(out_verbose.stderr.contains("read failed: .kiro/specs/late/design.md"));
}
