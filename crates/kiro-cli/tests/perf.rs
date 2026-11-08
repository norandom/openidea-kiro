//! Integration test measuring performance over a 20-spec tree (~100 KiB per document).
//!
//! Requirement 13.7: The kiro-cli shall check a tree of twenty specs of typical size
//! (up to 100 KiB per document) with the default configuration in under 5 seconds
//! in a native release build.
//!
//! In debug builds, the timing assertion is skipped with a visible diagnostic message.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use std::fs;
use std::time::Instant;
use tempfile::TempDir;

use common::run_cli;

fn build_perf_tree() -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let kiro_dir = root.join(".kiro");
    let specs_dir = kiro_dir.join("specs");
    let steering_dir = kiro_dir.join("steering");

    fs::create_dir_all(&specs_dir).unwrap();
    fs::create_dir_all(&steering_dir).unwrap();

    fs::write(kiro_dir.join("vocabulary.toml"), "# Empty vocabulary\n").unwrap();
    fs::write(steering_dir.join("roadmap.md"), "# Roadmap\n").unwrap();

    // Generate ~100 KiB requirements.md content
    let mut req_content = String::with_capacity(110_000);
    req_content.push_str("# Requirements Document\n\n## Introduction\nPerformance test benchmark document.\n\n## Requirements\n\n");

    let mut i = 1;
    while req_content.len() < 100 * 1024 {
        req_content.push_str(&format!(
            "### Requirement {i}: Component verification block {i}\n\n**Objective:** The controller manages component {i}.\n\n#### Acceptance Criteria\n\n1. The system shall process request {i} properly.\n\n"
        ));
        i += 1;
    }

    for spec_idx in 1..=20 {
        let spec_name = format!("spec-{spec_idx:02}");
        let spec_path = specs_dir.join(&spec_name);
        fs::create_dir_all(&spec_path).unwrap();

        let spec_json = format!(
            r#"{{
  "feature_name": "{spec_name}",
  "phase": "requirements-generated",
  "topics": []
}}
"#
        );
        fs::write(spec_path.join("spec.json"), spec_json).unwrap();
        fs::write(spec_path.join("requirements.md"), &req_content).unwrap();
    }

    temp
}

#[test]
fn perf_twenty_specs_under_five_seconds() {
    if cfg!(debug_assertions) {
        eprintln!(
            "SKIPPED: perf test skipped in debug builds (release build required for < 5s assertion)"
        );
        return;
    }

    let temp = build_perf_tree();
    let root_str = temp.path().to_str().unwrap();

    let start = Instant::now();
    let out = run_cli(&["--project", root_str, "check", "--all"]);
    let elapsed = start.elapsed();

    assert!(
        out.code == 0 || out.code == 1,
        "perf check failed with code {}: stderr:\n{}\nstdout:\n{}",
        out.code,
        out.stderr,
        out.stdout
    );
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "check of 20 specs took {:?}, exceeding 5.0s threshold",
        elapsed
    );
    println!(
        "checked 20 specs (~100 KiB/doc) in {:.2?} (target < 5.0s)",
        elapsed
    );
}
