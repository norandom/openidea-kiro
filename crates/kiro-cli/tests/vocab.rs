//! Integration tests for `kiro-cli vocab validate` and `kiro-cli vocab import`.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::{cli_fixture_path, core_fixture_path, run_cli, setup_tree, workspace_root};

#[test]
fn vocab_validate_seed_missing_semantic_and_unreadable() {
    let good_tree = core_fixture_path("checks/good");
    let (_temp, path) = setup_tree(&good_tree);
    let path_str = path.to_str().unwrap();

    // 1. Seed vocabulary in repo exits 0
    let repo_root = workspace_root();
    let out_seed = run_cli(&[
        "--project",
        repo_root.to_str().unwrap(),
        "vocab",
        "validate",
    ]);
    assert_eq!(out_seed.code, 0, "stderr: {}", out_seed.stderr);
    assert!(out_seed.stdout.contains("loaded"));

    // 2. Validate with JSON format
    let out_json = run_cli(&[
        "--project",
        repo_root.to_str().unwrap(),
        "vocab",
        "validate",
        "--format",
        "json",
    ]);
    assert_eq!(out_json.code, 0);
    let json_val: serde_json::Value = serde_json::from_str(&out_json.stdout).unwrap();
    assert_eq!(json_val["status"], "loaded");

    // 3. Missing default vocabulary exits 1 with VOCAB_MISSING
    let empty_temp = tempfile::tempdir().unwrap();
    let out_missing = run_cli(&[
        "--project",
        empty_temp.path().to_str().unwrap(),
        "vocab",
        "validate",
    ]);
    assert_eq!(out_missing.code, 1);
    assert!(out_missing.stdout.contains("VOCAB_MISSING"));

    // 4. Unreadable explicit file exits 2
    let unreadable = path.join("nonexistent_vocab.toml");
    let out_unreadable = run_cli(&[
        "--project",
        path_str,
        "vocab",
        "validate",
        unreadable.to_str().unwrap(),
    ]);
    assert_eq!(out_unreadable.code, 2);
    assert!(out_unreadable.stderr.contains("failed to read file"));
}

#[test]
fn vocab_import_page_vs_array_byte_equal() {
    let temp = tempfile::tempdir().unwrap();
    let page_src = cli_fixture_path("import/terms-page.json");
    let array_src = cli_fixture_path("import/terms-array.json");

    let out_page = temp.path().join("from_page.toml");
    let out_array = temp.path().join("from_array.toml");

    let run_page = run_cli(&[
        "vocab",
        "import",
        page_src.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        out_page.to_str().unwrap(),
    ]);
    assert_eq!(run_page.code, 0, "stderr: {}", run_page.stderr);

    let run_array = run_cli(&[
        "vocab",
        "import",
        array_src.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        out_array.to_str().unwrap(),
    ]);
    assert_eq!(run_array.code, 0, "stderr: {}", run_array.stderr);

    // Both files produce byte-equal TOML (modulo the filename in header comment)
    let page_text = std::fs::read_to_string(&out_page).unwrap();
    let array_text = std::fs::read_to_string(&out_array).unwrap();

    let page_body: Vec<&str> = page_text.lines().skip(2).collect();
    let array_body: Vec<&str> = array_text.lines().skip(2).collect();
    assert_eq!(page_body, array_body);

    // Findings appeared on stderr
    assert!(run_page.stderr.contains("IMPORT_TERM_SKIPPED"));
    assert!(run_page.stderr.contains("IMPORT_DUPLICATE_ID"));
}

#[test]
fn vocab_import_detail_fixture_and_related_comment() {
    let temp = tempfile::tempdir().unwrap();
    let detail_src = cli_fixture_path("import/terms-detail.json");
    let out_file = temp.path().join("detail.toml");

    let run = run_cli(&[
        "vocab",
        "import",
        detail_src.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        out_file.to_str().unwrap(),
    ]);
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);

    let content = std::fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("# related: Order"));
    assert!(content.contains("# related: Invoice"));
}

#[test]
fn vocab_import_fresh_refusal_merge_and_stdout_mode() {
    let temp = tempfile::tempdir().unwrap();
    let page_src = cli_fixture_path("import/terms-page.json");
    let existing_src = cli_fixture_path("import/existing.toml");

    let out_file = temp.path().join("vocab.toml");

    // 1. Fresh write
    let run_fresh = run_cli(&[
        "vocab",
        "import",
        page_src.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        out_file.to_str().unwrap(),
    ]);
    assert_eq!(run_fresh.code, 0);

    // 2. Refused overwrite without --merge
    let run_refuse = run_cli(&[
        "vocab",
        "import",
        page_src.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        out_file.to_str().unwrap(),
    ]);
    assert_eq!(run_refuse.code, 2);
    assert!(run_refuse.stderr.contains("exists; use --merge"));

    // 3. Merge into existing fixture
    let merge_file = temp.path().join("existing.toml");
    std::fs::copy(&existing_src, &merge_file).unwrap();

    let run_merge = run_cli(&[
        "vocab",
        "import",
        page_src.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        merge_file.to_str().unwrap(),
        "--merge",
    ]);
    assert_eq!(run_merge.code, 0);

    // Merge reports two conflicts on stderr
    assert_eq!(
        run_merge.stderr.matches("IMPORT_MERGE_CONFLICT").count(),
        2
    );

    // Unchanged prefix
    let existing_orig = std::fs::read_to_string(&existing_src).unwrap();
    let merged_text = std::fs::read_to_string(&merge_file).unwrap();
    assert!(merged_text.starts_with(&existing_orig));

    // 4. Standard-output mode: --out -
    let run_stdout = run_cli(&[
        "vocab",
        "import",
        page_src.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        "-",
    ]);
    assert_eq!(run_stdout.code, 0);
    // stdout contains TOML text alone
    assert!(run_stdout.stdout.contains("schema_version = \"2\""));
    assert!(run_stdout.stdout.contains("[terms.order]"));
    // findings appear on stderr
    assert!(run_stdout.stderr.contains("IMPORT_DUPLICATE_ID"));
}

#[test]
fn vocab_import_skos_turtle_fixture_and_invalid_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let concepts_src = cli_fixture_path("import/concepts.ttl");
    let out_file = temp.path().join("from_skos.toml");

    let run_skos = run_cli(&[
        "vocab",
        "import",
        concepts_src.to_str().unwrap(),
        "--format",
        "skos-turtle",
        "--topic",
        "sec",
        "--out",
        out_file.to_str().unwrap(),
    ]);
    assert_eq!(run_skos.code, 0, "stderr: {}", run_skos.stderr);

    let content = std::fs::read_to_string(&out_file).unwrap();
    // Mapping-example ids
    assert!(content.contains("[terms.insider-threat]"));
    assert!(content.contains("[terms.https-proxy]"));
    assert!(content.contains("[terms.level2-cache]"));

    // Annotations and scheme title
    assert!(content.contains("kind = \"enum\""));
    assert!(content.contains("kind = \"int\""));
    assert!(content.contains("deprecated = true"));
    assert!(content.contains("label = \"Security & Architecture Concepts\""));

    // Validate the generated file through vocab validate
    let val_run = run_cli(&["vocab", "validate", out_file.to_str().unwrap()]);
    assert_eq!(val_run.code, 0, "stderr: {}", val_run.stderr);
    assert!(val_run.stdout.contains("loaded"));

    // Invalid JSON exits 2 with line
    let invalid_json = cli_fixture_path("import/invalid.json");
    let run_invalid_json = run_cli(&[
        "vocab",
        "import",
        invalid_json.to_str().unwrap(),
        "--format",
        "auto-ontology-json",
        "--topic",
        "sdd",
        "--out",
        "-",
    ]);
    assert_eq!(run_invalid_json.code, 2);
    assert!(run_invalid_json.stderr.contains("IMPORT_INPUT_INVALID"));
    assert!(run_invalid_json.stderr.contains("invalid.json"));
    assert!(run_invalid_json.stderr.contains(":3:"));

    // Invalid Turtle exits 2 with line
    let invalid_ttl = cli_fixture_path("import/invalid.ttl");
    let run_invalid_ttl = run_cli(&[
        "vocab",
        "import",
        invalid_ttl.to_str().unwrap(),
        "--format",
        "skos-turtle",
        "--topic",
        "sec",
        "--out",
        "-",
    ]);
    assert_eq!(run_invalid_ttl.code, 2);
    assert!(run_invalid_ttl.stderr.contains("IMPORT_INPUT_INVALID"));
    assert!(run_invalid_ttl.stderr.contains("invalid.ttl"));
    assert!(run_invalid_ttl.stderr.contains(":4:"));
}
