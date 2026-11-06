//! Golden renderer tests: byte-for-byte comparisons of the Mermaid and
//! text renderers' output against the fixture files in `fixtures/golden`,
//! compiled against the crate-root re-exports only (task 5.1).

mod common;

use kiro_core::{
    Caps, Direction, Edge, Finding, Graph, Node, codes, findings_text, graph_text, mermaid,
};

/// The golden graph: labels with quotes and angle brackets, classes, and
/// labelled edges (task 5.1 observable).
fn golden_graph() -> Graph {
    Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            Node {
                id: "start".into(),
                label: "Start \"here\"".into(),
                class: Some("entry".into()),
            },
            Node {
                id: "check".into(),
                label: "<check>".into(),
                class: None,
            },
            Node {
                id: "done".into(),
                label: "Done".into(),
                class: Some("terminal".into()),
            },
        ],
        edges: vec![
            Edge {
                from: "start".into(),
                to: "check".into(),
                label: Some("fast path".into()),
            },
            Edge {
                from: "check".into(),
                to: "done".into(),
                label: None,
            },
        ],
    }
}

#[test]
fn golden_mermaid_matches_the_fixture() {
    let rendered = mermaid(
        &golden_graph(),
        &Caps {
            max_nodes: 10,
            max_edges: 10,
        },
    );
    assert!(!rendered.truncated);
    assert_eq!(
        rendered.text,
        include_str!("../fixtures/golden/graph.mermaid")
    );
}

#[test]
fn golden_text_matches_the_fixture() {
    let rendered = graph_text(
        &golden_graph(),
        &Caps {
            max_nodes: 10,
            max_edges: 10,
        },
    );
    assert!(!rendered.truncated);
    assert_eq!(rendered.text, include_str!("../fixtures/golden/graph.text"));
}

#[test]
fn golden_truncated_mermaid_matches_the_fixture() {
    let rendered = mermaid(
        &golden_graph(),
        &Caps {
            max_nodes: 1,
            max_edges: 10,
        },
    );
    assert!(rendered.truncated);
    assert_eq!(rendered.omitted_nodes, 2);
    assert_eq!(rendered.omitted_edges, 2);
    assert_eq!(
        rendered.text,
        include_str!("../fixtures/golden/graph-truncated.mermaid")
    );
}

#[test]
fn golden_findings_text_matches_the_fixture() {
    let findings = vec![
        Finding::new(
            codes::TASK_REQ_UNKNOWN,
            ".kiro/specs/demo/tasks.md",
            "reference '9.9' names no criterion",
        )
        .at(12)
        .feature("demo"),
        Finding::new(
            codes::MD_FENCE_UNCLOSED,
            ".kiro/specs/demo/design.md",
            "fence left open",
        )
        .at(3)
        .feature("demo"),
        Finding::new(
            codes::SPEC_DOC_MISSING,
            ".kiro/specs/demo/design.md",
            "document '.kiro/specs/demo/design.md' is missing",
        )
        .feature("demo"),
    ];
    assert_eq!(
        findings_text(&findings),
        include_str!("../fixtures/golden/findings.text")
    );
}

#[test]
fn minimal_tree_loads_cleanly_through_the_reexports() {
    let mut ws = common::minimal_workspace();
    let demo = ws.load("demo");
    assert_eq!(demo.name, "demo");
    assert!(
        demo.findings.is_empty(),
        "clean fixture loads without findings: {:?}",
        demo.findings
    );
}
