//! Integration tests for `kiro-cli` visualization commands (`status`, `graph`, `tasks`).

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::{core_fixture_path, run_cli, setup_tree};

#[test]
fn view_status_golden_parity() {
    let minimal_tree = core_fixture_path("minimal");
    let (_temp, path) = setup_tree(&minimal_tree);
    let path_str = path.to_str().unwrap();

    // 1. Status demo equals viz-status-demo.txt + '\n'
    let golden_demo = std::fs::read_to_string(core_fixture_path("golden/viz-status-demo.txt"))
        .unwrap()
        .replace("\r\n", "\n");
    let expected_demo = if golden_demo.ends_with('\n') {
        golden_demo
    } else {
        format!("{golden_demo}\n")
    };

    let out_demo = run_cli(&["--project", path_str, "status", "demo"]);
    assert_eq!(out_demo.code, 0);
    assert_eq!(out_demo.stdout, expected_demo);

    // 2. Status old (superseded) equals viz-status-old.txt + '\n'
    let golden_old = std::fs::read_to_string(core_fixture_path("golden/viz-status-old.txt"))
        .unwrap()
        .replace("\r\n", "\n");
    let expected_old = if golden_old.ends_with('\n') {
        golden_old
    } else {
        format!("{golden_old}\n")
    };

    let out_old = run_cli(&["--project", path_str, "status", "old"]);
    assert_eq!(out_old.code, 0);
    assert_eq!(out_old.stdout, expected_old);
}

#[test]
fn view_graph_golden_parity() {
    let minimal_tree = core_fixture_path("minimal");
    let (_temp, path) = setup_tree(&minimal_tree);
    let path_str = path.to_str().unwrap();

    // 1. Graph diagram equals viz-graph-demo.md + '\n'
    let golden_graph = std::fs::read_to_string(core_fixture_path("golden/viz-graph-demo.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let expected_graph = if golden_graph.ends_with('\n') {
        golden_graph
    } else {
        format!("{golden_graph}\n")
    };

    let out_graph = run_cli(&["--project", path_str, "graph", "demo"]);
    assert_eq!(out_graph.code, 0);
    assert_eq!(out_graph.stdout, expected_graph);

    // 2. Graph table equals viz-graph-demo-table.md + '\n'
    let golden_table = std::fs::read_to_string(core_fixture_path("golden/viz-graph-demo-table.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let expected_table = if golden_table.ends_with('\n') {
        golden_table
    } else {
        format!("{golden_table}\n")
    };

    let out_table = run_cli(&["--project", path_str, "graph", "demo", "--table"]);
    assert_eq!(out_table.code, 0);
    assert_eq!(out_table.stdout, expected_table);
}

#[test]
fn view_tasks_golden_parity() {
    let minimal_tree = core_fixture_path("minimal");
    let (_temp, path) = setup_tree(&minimal_tree);
    let path_str = path.to_str().unwrap();

    // 1. Tasks diagram equals viz-tasks-demo.md + '\n'
    let golden_tasks = std::fs::read_to_string(core_fixture_path("golden/viz-tasks-demo.md"))
        .unwrap()
        .replace("\r\n", "\n");
    let expected_tasks = if golden_tasks.ends_with('\n') {
        golden_tasks
    } else {
        format!("{golden_tasks}\n")
    };

    let out_tasks = run_cli(&["--project", path_str, "tasks", "demo"]);
    assert_eq!(out_tasks.code, 0);
    assert_eq!(out_tasks.stdout, expected_tasks);

    // 2. Tasks table equals viz-tasks-demo-table.md + '\n'
    let golden_tasks_table =
        std::fs::read_to_string(core_fixture_path("golden/viz-tasks-demo-table.md"))
            .unwrap()
            .replace("\r\n", "\n");
    let expected_tasks_table = if golden_tasks_table.ends_with('\n') {
        golden_tasks_table
    } else {
        format!("{golden_tasks_table}\n")
    };

    let out_tasks_table = run_cli(&["--project", path_str, "tasks", "demo", "--table"]);
    assert_eq!(out_tasks_table.code, 0);
    assert_eq!(out_tasks_table.stdout, expected_tasks_table);
}

#[test]
fn view_errors_and_undiscovered_spec() {
    let minimal_tree = core_fixture_path("minimal");
    let (_temp, path) = setup_tree(&minimal_tree);
    let path_str = path.to_str().unwrap();

    // Undiscovered spec: status nope exits 2 naming discovered specs demo, old
    let out_nope = run_cli(&["--project", path_str, "status", "nope"]);
    assert_eq!(out_nope.code, 2);
    assert!(out_nope.stderr.contains("spec `nope` was not found"));
    assert!(out_nope.stderr.contains("discovered: demo, old"));

    // Spec without tasks document: tasks old exits 2 with IDE's message
    let out_old_tasks = run_cli(&["--project", path_str, "tasks", "old"]);
    assert_eq!(out_old_tasks.code, 2);
    assert!(out_old_tasks.stderr.contains("spec `old` has no readable .kiro/specs/old/tasks.md"));
}
