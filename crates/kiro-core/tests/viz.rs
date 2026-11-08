//! Integration and golden tests for the visualization views: status, spec graph
//! and task dependency graph (design component `VizCoreTests`, requirements
//! 7.1, 7.2, 7.3, 7.4, 7.6, 8.1, 8.2, 8.3).

mod common;

use std::collections::HashSet;

use kiro_core::resolve::SuccessorChain;
use kiro_core::source::{Feature, MemorySource, Workspace};
use kiro_core::spec::{Doc, Phase, SpecMeta};
use kiro_core::viz::graph::{graph_summary, graph_view};
use kiro_core::viz::status::{ChainEnd, ChainLine, status_text, status_view};
use kiro_core::viz::tasks::{tasks_summary, tasks_view};
use kiro_core::viz::{CAPS, Format, RawLabeler, compose, render_body};

const GOLDEN_STATUS_DEMO: &str = include_str!("../fixtures/golden/viz-status-demo.txt");
const GOLDEN_STATUS_OLD: &str = include_str!("../fixtures/golden/viz-status-old.txt");
const GOLDEN_GRAPH_DEMO: &str = include_str!("../fixtures/golden/viz-graph-demo.md");
const GOLDEN_GRAPH_DEMO_TABLE: &str = include_str!("../fixtures/golden/viz-graph-demo-table.md");
const GOLDEN_TASKS_DEMO: &str = include_str!("../fixtures/golden/viz-tasks-demo.md");
const GOLDEN_TASKS_DEMO_TABLE: &str = include_str!("../fixtures/golden/viz-tasks-demo-table.md");

/// Lints that a Mermaid diagram body adheres strictly to valid Mermaid flowchart syntax:
/// 1. First line must be `graph LR` or `graph TD`
/// 2. All subsequent lines must be 4-space indented node definitions, class declarations,
///    or directed edges (optionally labelled).
/// 3. Identifiers must consist only of ASCII alphanumeric chars and underscores.
/// 4. Labels must not contain unescaped angle brackets or unescaped quotes.
/// 5. Edge endpoints must refer to declared node identifiers.
pub fn lint_mermaid(mermaid_text: &str) {
    let lines: Vec<&str> = mermaid_text.lines().collect();
    assert!(!lines.is_empty(), "mermaid text cannot be empty");

    let first = lines.first().copied().unwrap_or_default();
    assert!(
        first == "graph LR" || first == "graph TD",
        "first mermaid line must be 'graph LR' or 'graph TD', got: {first}"
    );

    let mut declared_ids = HashSet::new();

    for (i, line) in lines.iter().enumerate().skip(1) {
        assert!(
            line.starts_with("    "),
            "line {i} must be indented by 4 spaces: {line}"
        );
        let trimmed = line.get(4..).unwrap_or_default();

        if let Some(rest) = trimmed.strip_prefix("class ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            assert_eq!(parts.len(), 2, "line {i} invalid class syntax: {line}");
            let id = parts.first().copied().unwrap_or_default();
            assert!(
                declared_ids.contains(id),
                "line {i} class targets undeclared id '{id}': {line}"
            );
        } else if let Some((from, to_part)) = trimmed.split_once(" -->") {
            assert!(
                from.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "line {i} invalid from-id '{from}': {line}"
            );
            assert!(
                declared_ids.contains(from),
                "line {i} edge from undeclared id '{from}': {line}"
            );

            let to_part = to_part.trim_start();
            let to = if let Some(rest) = to_part.strip_prefix('|') {
                let Some((label_part, target)) = rest.split_once('|') else {
                    unreachable!("line {i} unclosed edge label: {line}");
                };
                assert!(
                    label_part.starts_with('"') && label_part.ends_with('"'),
                    "line {i} edge label must be quoted: {line}"
                );
                assert!(
                    !label_part.contains('<') && !label_part.contains('>'),
                    "line {i} edge label contains unescaped angle bracket: {line}"
                );
                target.trim()
            } else {
                to_part.trim()
            };

            assert!(
                to.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "line {i} invalid to-id '{to}': {line}"
            );
            assert!(
                declared_ids.contains(to),
                "line {i} edge to undeclared id '{to}': {line}"
            );
        } else if let Some((id, rest)) = trimmed.split_once("[\"") {
            assert!(
                id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "line {i} invalid node id '{id}': {line}"
            );
            assert!(
                rest.ends_with("\"]"),
                "line {i} node definition must end with '\"]': {line}"
            );
            let label = rest.get(..rest.len().saturating_sub(2)).unwrap_or_default();
            assert!(
                !label.contains('<') && !label.contains('>'),
                "line {i} node label contains unescaped angle bracket: {line}"
            );
            assert!(
                !label.contains('"'),
                "line {i} node label contains unescaped quote: {line}"
            );
            declared_ids.insert(id);
        } else {
            unreachable!("line {i} does not match any valid Mermaid syntax: {line}");
        }
    }
}

/// Lints that a Markdown table has:
/// 1. A valid header row
/// 2. A separator row matching `| --- | ... |` with valid dash cells
/// 3. Equal cell counts across all rows
pub fn lint_table(table_text: &str) {
    let lines: Vec<&str> = table_text.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        lines.len() >= 2,
        "table must have at least header and separator"
    );

    let header = lines.first().copied().unwrap_or_default();
    assert!(
        header.starts_with('|') && header.ends_with('|'),
        "header must start and end with '|': {header}"
    );
    let expected_cols = header.split('|').count();

    let separator = lines.get(1).copied().unwrap_or_default();
    assert!(
        separator.starts_with('|') && separator.ends_with('|'),
        "separator must start and end with '|': {separator}"
    );
    assert_eq!(
        separator.split('|').count(),
        expected_cols,
        "separator must have same column count as header"
    );
    for cell in separator.split('|').filter(|c| !c.is_empty()) {
        let trimmed = cell.trim();
        assert!(
            trimmed.chars().all(|c| c == '-' || c == ':'),
            "separator cell must contain only dashes and colons: '{trimmed}'"
        );
        assert!(
            trimmed.contains('-'),
            "separator cell must have at least one dash: '{trimmed}'"
        );
    }

    for (i, row) in lines.iter().enumerate().skip(2) {
        assert!(
            row.starts_with('|') && row.ends_with('|'),
            "row {i} must start and end with '|': {row}"
        );
        assert_eq!(
            row.split('|').count(),
            expected_cols,
            "row {i} has mismatched column count: {row}"
        );
    }
}

fn extract_mermaid_body(text: &str) -> &str {
    let open = "```mermaid\n";
    let close = "```";
    let start = text
        .find(open)
        .map_or(0, |idx| idx.saturating_add(open.len()));
    let rest = text.get(start..).unwrap_or_default();
    let end = rest.find(close).unwrap_or(rest.len());
    rest.get(..end).unwrap_or_default()
}

fn extract_table_from_text(text: &str) -> String {
    let mut table_lines = Vec::new();
    for line in text.lines() {
        if line.starts_with('|') && line.ends_with('|') {
            table_lines.push(line);
        }
    }
    table_lines.join("\n")
}

#[test]
fn golden_outputs_are_byte_identical_and_pass_lints() {
    let mut ws = common::minimal_workspace();

    // 1. Status demo
    let demo = ws.load("demo").clone();
    let demo_chain = (demo.phase() == Phase::Superseded).then(|| ws.successor_chain("demo"));
    let status_demo_view = status_view(&demo, demo_chain.as_ref());
    let status_demo_text = status_text(&status_demo_view);
    assert_eq!(
        status_demo_text, GOLDEN_STATUS_DEMO,
        "viz-status-demo.txt mismatch"
    );
    lint_table(&extract_table_from_text(&status_demo_text));

    // 2. Status old
    let old = ws.load("old").clone();
    let old_chain = (old.phase() == Phase::Superseded).then(|| ws.successor_chain("old"));
    let status_old_view = status_view(&old, old_chain.as_ref());
    let status_old_text = status_text(&status_old_view);
    assert_eq!(
        status_old_text, GOLDEN_STATUS_OLD,
        "viz-status-old.txt mismatch"
    );
    lint_table(&extract_table_from_text(&status_old_text));

    // 3. Graph demo (diagram form)
    let graph_demo_view = graph_view(&demo, &RawLabeler);
    assert!(graph_demo_view.is_some());
    let Some(graph_demo_view) = graph_demo_view else {
        return;
    };
    let graph_demo_body = render_body(&graph_demo_view.graph, Format::Mermaid, &CAPS);
    let graph_demo_summary = graph_summary(&graph_demo_view, &graph_demo_body);
    let graph_demo_md = compose(&graph_demo_summary, &graph_demo_body.text);
    assert_eq!(
        graph_demo_md, GOLDEN_GRAPH_DEMO,
        "viz-graph-demo.md mismatch"
    );
    lint_mermaid(extract_mermaid_body(&graph_demo_md));

    // 4. Graph demo table
    let graph_demo_tbl_body = render_body(&graph_demo_view.graph, Format::Table, &CAPS);
    let graph_demo_tbl_summary = graph_summary(&graph_demo_view, &graph_demo_tbl_body);
    let graph_demo_tbl_md = compose(&graph_demo_tbl_summary, &graph_demo_tbl_body.text);
    assert_eq!(
        graph_demo_tbl_md, GOLDEN_GRAPH_DEMO_TABLE,
        "viz-graph-demo-table.md mismatch"
    );
    lint_table(&extract_table_from_text(&graph_demo_tbl_md));

    // 5. Tasks demo (diagram form)
    assert!(demo.tasks.is_some());
    let Some(demo_tasks) = demo.tasks.as_ref() else {
        return;
    };
    let tasks_demo_view = tasks_view("demo", demo_tasks);
    let tasks_demo_body = render_body(&tasks_demo_view.graph, Format::Mermaid, &CAPS);
    let tasks_demo_summary = tasks_summary(&tasks_demo_view, &tasks_demo_body);
    let tasks_demo_md = compose(&tasks_demo_summary, &tasks_demo_body.text);
    assert_eq!(
        tasks_demo_md, GOLDEN_TASKS_DEMO,
        "viz-tasks-demo.md mismatch"
    );
    lint_mermaid(extract_mermaid_body(&tasks_demo_md));

    // 6. Tasks demo table
    let tasks_demo_tbl_body = render_body(&tasks_demo_view.graph, Format::Table, &CAPS);
    let tasks_demo_tbl_summary = tasks_summary(&tasks_demo_view, &tasks_demo_tbl_body);
    let tasks_demo_tbl_md = compose(&tasks_demo_tbl_summary, &tasks_demo_tbl_body.text);
    assert_eq!(
        tasks_demo_tbl_md, GOLDEN_TASKS_DEMO_TABLE,
        "viz-tasks-demo-table.md mismatch"
    );
    lint_table(&extract_table_from_text(&tasks_demo_tbl_md));
}

#[test]
fn chain_rendering_terminal_cycle_and_no_successor() {
    let feature = Feature {
        name: "test_spec".to_string(),
        dir: ".kiro/specs/test_spec".to_string(),
        meta: Some(SpecMeta {
            phase: Phase::Superseded,
            phase_raw: Some("superseded".to_string()),
            ..Default::default()
        }),
        requirements: None,
        design: None,
        tasks: None,
        findings: Vec::new(),
    };

    // Terminal with terminal already last in path
    let chain1 = SuccessorChain {
        path: vec!["test_spec".to_string(), "v2".to_string()],
        terminal: Some("v2".to_string()),
        cyclic: false,
        findings: Vec::new(),
    };
    let view1 = status_view(&feature, Some(&chain1));
    assert_eq!(
        view1.chain,
        Some(ChainLine {
            path: vec!["test_spec".to_string(), "v2".to_string()],
            end: ChainEnd::Terminal,
        })
    );
    assert!(status_text(&view1).contains("Successor chain: test_spec -> v2 (terminal)"));

    // Terminal with terminal NOT in path
    let chain2 = SuccessorChain {
        path: vec!["test_spec".to_string()],
        terminal: Some("v2".to_string()),
        cyclic: false,
        findings: Vec::new(),
    };
    let view2 = status_view(&feature, Some(&chain2));
    assert_eq!(
        view2.chain,
        Some(ChainLine {
            path: vec!["test_spec".to_string(), "v2".to_string()],
            end: ChainEnd::Terminal,
        })
    );

    // Cycle
    let chain_cycle = SuccessorChain {
        path: vec![
            "test_spec".to_string(),
            "b".to_string(),
            "test_spec".to_string(),
        ],
        terminal: None,
        cyclic: true,
        findings: Vec::new(),
    };
    let view_cycle = status_view(&feature, Some(&chain_cycle));
    assert_eq!(
        view_cycle.chain,
        Some(ChainLine {
            path: vec![
                "test_spec".to_string(),
                "b".to_string(),
                "test_spec".to_string()
            ],
            end: ChainEnd::Cycle,
        })
    );
    assert!(
        status_text(&view_cycle).contains("Successor chain: test_spec -> b -> test_spec (cycle)")
    );

    // No successor
    let chain_no_succ = SuccessorChain {
        path: vec!["test_spec".to_string()],
        terminal: None,
        cyclic: false,
        findings: Vec::new(),
    };
    let view_no_succ = status_view(&feature, Some(&chain_no_succ));
    assert_eq!(
        view_no_succ.chain,
        Some(ChainLine {
            path: vec!["test_spec".to_string()],
            end: ChainEnd::NoSuccessor,
        })
    );
    assert!(status_text(&view_no_succ).contains("Successor chain: test_spec (no known successor)"));
}

#[test]
fn over_cap_synthetic_spec_truncates_and_table_remains_complete() {
    let mut req_text =
        String::from("### Requirement 1: Massive Requirement\n\n#### Acceptance Criteria\n");
    for i in 1..=250 {
        req_text.push_str(&format!("{i}. Criterion number {i}\n"));
    }

    let mut tasks_text = String::from("- [ ] 1. Major Task\n");
    for i in 1..=300 {
        let crit_id = if i <= 250 { i } else { 250 };
        tasks_text.push_str(&format!(
            "  - [ ] 1.{i} Subtask {i}\n    _Requirements: 1.{crit_id}_\n"
        ));
    }

    let src = MemorySource::new()
        .with(
            ".kiro/specs/synth/spec.json",
            r#"{"phase": "tasks-generated"}"#,
        )
        .with(".kiro/specs/synth/requirements.md", req_text)
        .with(".kiro/specs/synth/tasks.md", tasks_text);

    let mut ws = Workspace::new(src, vec!["synth".to_string()]);
    let feature = ws.load("synth").clone();

    let view = graph_view(&feature, &RawLabeler);
    assert!(view.is_some());
    let Some(view) = view else {
        return;
    };

    // Diagram form with default caps (200 nodes / 400 edges)
    let body_diag = render_body(&view.graph, Format::Mermaid, &CAPS);
    assert!(body_diag.truncated, "diagram body must be truncated");
    assert!(
        body_diag.omitted_nodes > 0,
        "omitted_nodes must be positive"
    );
    assert!(
        body_diag.omitted_edges > 0,
        "omitted_edges must be positive"
    );

    let summary = graph_summary(&view, &body_diag);
    let trunc_line = summary.last().map(String::as_str).unwrap_or_default();
    assert!(
        trunc_line.starts_with("Diagram truncated:"),
        "last summary line must report truncation: {trunc_line}"
    );
    assert!(trunc_line.contains("caps 200 nodes / 400 edges"));

    let composed_diagram = compose(&summary, &body_diag.text);
    // Lint mermaid passes on truncated diagram!
    lint_mermaid(extract_mermaid_body(&composed_diagram));

    // Table form with default caps (no truncation)
    let body_tbl = render_body(&view.graph, Format::Table, &CAPS);
    assert!(!body_tbl.truncated, "table form must never be truncated");
    assert_eq!(body_tbl.omitted_nodes, 0);
    assert_eq!(body_tbl.omitted_edges, 0);

    let tbl_summary = graph_summary(&view, &body_tbl);
    assert_eq!(
        tbl_summary.len(),
        3,
        "table summary has 3 lines (no truncation line)"
    );
    let composed_tbl = compose(&tbl_summary, &body_tbl.text);
    lint_table(&extract_table_from_text(&composed_tbl));
}

#[test]
fn partial_graphs_from_minimal_fixture() {
    let src = common::minimal_source();

    // 1. demo with design.md removed
    let mut ws1 = Workspace::new(src.clone(), vec!["demo".to_string()]);
    let mut demo_no_design = ws1.load("demo").clone();
    demo_no_design.design = None;

    let view1 = graph_view(&demo_no_design, &RawLabeler);
    assert!(view1.is_some());
    let Some(view1) = view1 else {
        return;
    };
    assert_eq!(view1.stats.components, 0);
    assert_eq!(view1.stats.files, 0);
    assert_eq!(view1.stats.missing, vec![Doc::Design]);

    let body1 = render_body(&view1.graph, Format::Mermaid, &CAPS);
    let summary1 = graph_summary(&view1, &body1);
    assert!(summary1.iter().any(|l| l == "Missing: design.md"));

    // 2. Only tasks.md present
    let mut demo_tasks_only = ws1.load("demo").clone();
    demo_tasks_only.requirements = None;
    demo_tasks_only.design = None;

    let view2 = graph_view(&demo_tasks_only, &RawLabeler);
    assert!(view2.is_some());
    let Some(view2) = view2 else {
        return;
    };
    assert_eq!(view2.stats.requirements, 0);
    assert_eq!(view2.stats.criteria, 0);
    assert_eq!(view2.stats.components, 0);
    assert_eq!(view2.stats.files, 0);
    assert_eq!(view2.stats.missing, vec![Doc::Requirements, Doc::Design]);

    let body2 = render_body(&view2.graph, Format::Mermaid, &CAPS);
    let summary2 = graph_summary(&view2, &body2);
    assert!(
        summary2
            .iter()
            .any(|l| l == "Missing: requirements.md, design.md")
    );

    // 3. Nothing present
    let mut demo_nothing = ws1.load("demo").clone();
    demo_nothing.requirements = None;
    demo_nothing.design = None;
    demo_nothing.tasks = None;

    assert_eq!(graph_view(&demo_nothing, &RawLabeler), None);
}

/// Minimal fixed-seed pseudo-random generator (LCG) for garbage testing.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn random_string(&mut self, len: usize) -> String {
        let chars =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \n\t#-_|*`[]()";
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            let idx = (self.next_u32() as usize) % chars.len();
            if let Some(&b) = chars.get(idx) {
                s.push(b as char);
            }
        }
        s
    }
}

#[test]
fn no_panic_over_garbage_documents() {
    let mut rng = SimpleRng::new(42);

    for _ in 0..50 {
        let req_text = rng.random_string(200);
        let design_text = rng.random_string(200);
        let tasks_text = rng.random_string(200);
        let spec_text = rng.random_string(100);

        let src = MemorySource::new()
            .with(".kiro/specs/rand/spec.json", spec_text)
            .with(".kiro/specs/rand/requirements.md", req_text)
            .with(".kiro/specs/rand/design.md", design_text)
            .with(".kiro/specs/rand/tasks.md", tasks_text);

        let mut ws = Workspace::new(src, vec!["rand".to_string()]);
        let feature = ws.load("rand").clone();

        // Status view
        let chain = (feature.phase() == Phase::Superseded).then(|| ws.successor_chain("rand"));
        let s_view = status_view(&feature, chain.as_ref());
        let _ = status_text(&s_view);

        // Graph view
        if let Some(g_view) = graph_view(&feature, &RawLabeler) {
            let g_body = render_body(&g_view.graph, Format::Mermaid, &CAPS);
            let g_summary = graph_summary(&g_view, &g_body);
            let _ = compose(&g_summary, &g_body.text);
        }

        // Tasks view
        if let Some(t_doc) = &feature.tasks {
            let t_view = tasks_view("rand", t_doc);
            let t_body = render_body(&t_view.graph, Format::Mermaid, &CAPS);
            let t_summary = tasks_summary(&t_view, &t_body);
            let _ = compose(&t_summary, &t_body.text);
        }
    }
}

#[test]
fn determinism_two_builds_are_byte_identical() {
    let mut ws = common::minimal_workspace();
    let demo1 = ws.load("demo").clone();
    let demo2 = ws.load("demo").clone();

    // Status view determinism
    let status1 = status_text(&status_view(&demo1, None));
    let status2 = status_text(&status_view(&demo2, None));
    assert_eq!(status1, status2, "status view must be deterministic");

    // Graph view determinism
    let g1 = graph_view(&demo1, &RawLabeler);
    let g2 = graph_view(&demo2, &RawLabeler);
    assert_eq!(g1, g2, "graph view must be deterministic");

    assert!(g1.is_some());
    let Some(g1) = g1 else {
        return;
    };
    let g_body1 = render_body(&g1.graph, Format::Mermaid, &CAPS);
    let g_body2 = render_body(&g1.graph, Format::Mermaid, &CAPS);
    assert_eq!(g_body1, g_body2, "graph body must be deterministic");

    // Tasks view determinism
    assert!(demo1.tasks.is_some());
    let Some(tasks1) = demo1.tasks.as_ref() else {
        return;
    };
    assert!(demo2.tasks.is_some());
    let Some(tasks2) = demo2.tasks.as_ref() else {
        return;
    };
    let t1 = tasks_view("demo", tasks1);
    let t2 = tasks_view("demo", tasks2);
    assert_eq!(t1, t2, "tasks view must be deterministic");
}
