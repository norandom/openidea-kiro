//! Visualization views for spec structure: status, spec graph, and task graph
//! (design component `VizCommon`, requirements 1.4, 2.2, 2.3, 4.3, 6.1, 6.2, 6.3,
//! 7.1, 7.2, 7.3).
//!
//! Provides the shared label rules, preferred-label seam, output formats (fenced
//! Mermaid diagram and Markdown table), composition, and common truncation
//! summary line.

pub mod graph;
pub mod status;
pub mod tasks;

use std::collections::HashMap;

pub use crate::render::DEFAULT_CAPS as CAPS;
use crate::render::{Caps, Graph};
use crate::spec::Doc;
pub use crate::spec::phase_label;
use crate::tasks::Task;

/// Maximum characters in a truncated node label before appending an ellipsis.
pub const LABEL_CHARS: usize = 48;
/// Opening fence for rendered Mermaid diagrams.
pub const FENCE_OPEN: &str = "```mermaid";
/// Closing fence for rendered Mermaid diagrams.
pub const FENCE_CLOSE: &str = "```";
/// Option keyword selecting the table output form instead of a Mermaid diagram.
pub const TEXT_OPTION: &str = "text";

/// Preferred-label seam. Implementations must only answer within `topics` (the
/// spec's declared topics).
pub trait Labeler {
    /// Display label for a design component name, or `None` to use `raw`.
    fn component_label(&self, raw: &str, topics: &[String]) -> Option<String>;
}

/// Default identity labeler that always returns `None` (use raw name).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RawLabeler;

impl Labeler for RawLabeler {
    fn component_label(&self, _raw: &str, _topics: &[String]) -> Option<String> {
        None
    }
}

/// Output format for graph views: diagram or table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Mermaid,
    Table,
}

/// Rendered view body with its truncation facts and caps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    pub text: String,
    pub truncated: bool,
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
    pub caps: Caps,
}

/// Collapses whitespace runs to one space, trims, keeps the first `LABEL_CHARS`
/// (48) chars and appends '…' when anything was cut.
pub fn truncate_label(text: &str) -> String {
    let trimmed = text.trim();
    let mut collapsed = String::with_capacity(trimmed.len());
    let mut last_was_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                collapsed.push(' ');
                last_was_space = true;
            }
        } else {
            collapsed.push(ch);
            last_was_space = false;
        }
    }
    if collapsed.chars().count() > LABEL_CHARS {
        let mut truncated: String = collapsed.chars().take(LABEL_CHARS).collect();
        truncated.push('\u{2026}');
        truncated
    } else {
        collapsed
    }
}

/// Formats a task label: `"[x]"` or `"[ ]"`, `"*"` when optional, a space, the id,
/// `" (P)"` when parallel, a space, and `truncate_label(description)`.
pub fn task_label(task: &Task) -> String {
    let box_str = if task.checked { "[x]" } else { "[ ]" };
    let opt_str = if task.optional { "*" } else { "" };
    let par_str = if task.parallel { " (P)" } else { "" };
    let desc = truncate_label(&task.description);
    format!("{box_str}{opt_str} {id}{par_str} {desc}", id = task.id)
}

/// Bare document file name for a `Doc`: `"spec.json"`, `"requirements.md"`,
/// `"design.md"`, or `"tasks.md"`.
pub fn doc_file_name(doc: Doc) -> &'static str {
    match doc {
        Doc::SpecJson => "spec.json",
        Doc::Requirements => "requirements.md",
        Doc::Design => "design.md",
        Doc::Tasks => "tasks.md",
    }
}

/// Renders a graph body in the requested format under size caps.
///
/// In Mermaid format, wraps the diagram in ```mermaid ... ``` fences. In Table
/// format, renders a complete node table with no caps applied.
pub fn render_body(graph: &Graph, format: Format, caps: &Caps) -> Body {
    match format {
        Format::Mermaid => {
            let rendered = crate::render::mermaid::mermaid(graph, caps);
            let text = format!("{FENCE_OPEN}\n{}{FENCE_CLOSE}\n", rendered.text);
            Body {
                text,
                truncated: rendered.truncated,
                omitted_nodes: rendered.omitted_nodes,
                omitted_edges: rendered.omitted_edges,
                caps: *caps,
            }
        }
        Format::Table => {
            let text = node_table(graph);
            Body {
                text,
                truncated: false,
                omitted_nodes: 0,
                omitted_edges: 0,
                caps: *caps,
            }
        }
    }
}

/// Complete node table with columns `Kind`, `Node`, and `Links to`.
///
/// Tolerates dangling edges by skipping endpoints not in `graph.nodes`.
pub fn node_table(graph: &Graph) -> String {
    let node_labels: HashMap<&str, &str> = graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.label.as_str()))
        .collect();

    let mut rows: Vec<Vec<String>> = Vec::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        let kind = node.class.clone().unwrap_or_default();
        let mut links = Vec::new();
        for edge in graph.edges.iter().filter(|e| e.from == node.id) {
            if let Some(&target_label) = node_labels.get(edge.to.as_str()) {
                let link = match &edge.label {
                    Some(lbl) if !lbl.is_empty() => format!("{target_label} ({lbl})"),
                    _ => target_label.to_string(),
                };
                links.push(link);
            }
        }
        let links_to = links.join("; ");
        rows.push(vec![kind, node.label.clone(), links_to]);
    }
    crate::render::text::table(&["Kind", "Node", "Links to"], &rows)
}

/// Composes summary lines and a body. When summary is empty, returns body unchanged.
pub fn compose(summary: &[String], body: &str) -> String {
    if summary.is_empty() {
        body.to_string()
    } else {
        format!("{}\n\n{body}", summary.join("\n"))
    }
}

/// Formats the standard truncation note if the diagram was truncated under caps.
pub fn truncation_line(body: &Body) -> Option<String> {
    if body.truncated {
        Some(format!(
            "Diagram truncated: {} nodes and {} edges omitted (caps {} nodes / {} edges); run with the `text` option for the complete table.",
            body.omitted_nodes, body.omitted_edges, body.caps.max_nodes, body.caps.max_edges
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Direction, Edge, Node};
    use crate::spec::{Doc, SpecMeta};
    use crate::tasks::Task;

    #[test]
    fn truncate_label_lengths_and_multibyte() {
        // 47 chars: unchanged
        let text47 = "a".repeat(47);
        assert_eq!(truncate_label(&text47), text47);

        // 48 chars: unchanged
        let text48 = "a".repeat(48);
        assert_eq!(truncate_label(&text48), text48);

        // 49 chars: truncated at 48 + ellipsis
        let text49 = "a".repeat(49);
        assert_eq!(
            truncate_label(&text49),
            format!("{}\u{2026}", "a".repeat(48))
        );

        // Whitespace runs collapsed and trimmed
        assert_eq!(
            truncate_label("   hello \t\n  world   this   is   a test   "),
            "hello world this is a test"
        );

        // Multibyte characters (Chinese characters count as 1 char each)
        let ch47: String = "中".repeat(47);
        assert_eq!(truncate_label(&ch47), ch47);

        let ch48: String = "中".repeat(48);
        assert_eq!(truncate_label(&ch48), ch48);

        let ch49: String = "中".repeat(49);
        let expected49 = format!("{}\u{2026}", "中".repeat(48));
        assert_eq!(truncate_label(&ch49), expected49);
    }

    #[test]
    fn task_label_matrix() {
        let base_task = Task {
            id: "1.1".to_string(),
            description: "Build engine".to_string(),
            ..Default::default()
        };

        // 2 (checked) × 2 (optional) × 2 (parallel) = 8 combinations
        let mut t = base_task.clone();
        t.checked = false;
        t.optional = false;
        t.parallel = false;
        assert_eq!(task_label(&t), "[ ] 1.1 Build engine");

        t.checked = true;
        assert_eq!(task_label(&t), "[x] 1.1 Build engine");

        t.checked = false;
        t.optional = true;
        assert_eq!(task_label(&t), "[ ]* 1.1 Build engine");

        t.checked = true;
        t.optional = true;
        assert_eq!(task_label(&t), "[x]* 1.1 Build engine");

        t.checked = false;
        t.optional = false;
        t.parallel = true;
        assert_eq!(task_label(&t), "[ ] 1.1 (P) Build engine");

        t.checked = true;
        t.optional = false;
        t.parallel = true;
        assert_eq!(task_label(&t), "[x] 1.1 (P) Build engine");

        t.checked = false;
        t.optional = true;
        t.parallel = true;
        assert_eq!(task_label(&t), "[ ]* 1.1 (P) Build engine");

        t.checked = true;
        t.optional = true;
        t.parallel = true;
        assert_eq!(task_label(&t), "[x]* 1.1 (P) Build engine");
    }

    #[test]
    fn phase_label_cases() {
        // None meta
        assert_eq!(phase_label(None), "missing");

        // SpecMeta with absent phase_raw
        let meta_none = SpecMeta::default();
        assert_eq!(phase_label(Some(&meta_none)), "missing");

        // SpecMeta with empty phase_raw
        let meta_empty = SpecMeta {
            phase_raw: Some("   ".to_string()),
            ..Default::default()
        };
        assert_eq!(phase_label(Some(&meta_empty)), "missing");

        // SpecMeta with padded phase_raw
        let meta_padded = SpecMeta {
            phase_raw: Some("  tasks-generated  ".to_string()),
            ..Default::default()
        };
        assert_eq!(phase_label(Some(&meta_padded)), "tasks-generated");
    }

    #[test]
    fn doc_file_name_variants() {
        assert_eq!(doc_file_name(Doc::SpecJson), "spec.json");
        assert_eq!(doc_file_name(Doc::Requirements), "requirements.md");
        assert_eq!(doc_file_name(Doc::Design), "design.md");
        assert_eq!(doc_file_name(Doc::Tasks), "tasks.md");
    }

    #[test]
    fn render_body_fence_shape_and_caps() {
        let graph = Graph {
            direction: Direction::LeftRight,
            nodes: vec![
                Node {
                    id: "a".to_string(),
                    label: "Alpha".to_string(),
                    class: Some("req".to_string()),
                },
                Node {
                    id: "b".to_string(),
                    label: "Beta".to_string(),
                    class: Some("crit".to_string()),
                },
            ],
            edges: vec![Edge {
                from: "a".to_string(),
                to: "b".to_string(),
                label: None,
            }],
        };

        let caps = Caps {
            max_nodes: 10,
            max_edges: 10,
        };
        let body = render_body(&graph, Format::Mermaid, &caps);
        assert!(!body.truncated);
        assert_eq!(body.omitted_nodes, 0);
        assert_eq!(body.omitted_edges, 0);
        assert_eq!(body.caps, caps);
        assert!(body.text.starts_with("```mermaid\n"));
        assert!(body.text.ends_with("```\n"));

        // Truncated graph
        let small_caps = Caps {
            max_nodes: 1,
            max_edges: 0,
        };
        let truncated_body = render_body(&graph, Format::Mermaid, &small_caps);
        assert!(truncated_body.truncated);
        assert_eq!(truncated_body.omitted_nodes, 1);
        assert_eq!(truncated_body.omitted_edges, 1);
        assert!(truncated_body.text.contains("… 1 more nodes, 1 more edges"));
    }

    #[test]
    fn node_table_labelled_unlabelled_and_dangling_edges() {
        let graph = Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "n1".to_string(),
                    label: "First".to_string(),
                    class: Some("major".to_string()),
                },
                Node {
                    id: "n2".to_string(),
                    label: "Second".to_string(),
                    class: None,
                },
            ],
            edges: vec![
                Edge {
                    from: "n1".to_string(),
                    to: "n2".to_string(),
                    label: Some("order".to_string()),
                },
                Edge {
                    from: "n1".to_string(),
                    to: "dangling".to_string(),
                    label: Some("skip".to_string()),
                },
                Edge {
                    from: "n2".to_string(),
                    to: "n1".to_string(),
                    label: None,
                },
            ],
        };

        let table = node_table(&graph);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines.first().copied(), Some("| Kind | Node | Links to |"));
        assert_eq!(lines.get(1).copied(), Some("| --- | --- | --- |"));
        assert_eq!(
            lines.get(2).copied(),
            Some("| major | First | Second (order) |")
        );
        assert_eq!(lines.get(3).copied(), Some("|  | Second | First |"));
    }

    #[test]
    fn compose_and_truncation_line() {
        let body = Body {
            text: "body\n".to_string(),
            truncated: false,
            omitted_nodes: 0,
            omitted_edges: 0,
            caps: CAPS,
        };

        // Empty summary returns body
        assert_eq!(compose(&[], &body.text), "body\n");

        // Non-empty summary
        let summary = vec!["Summary line 1".to_string(), "Summary line 2".to_string()];
        assert_eq!(
            compose(&summary, &body.text),
            "Summary line 1\nSummary line 2\n\nbody\n"
        );

        // Truncation line absent when not truncated
        assert_eq!(truncation_line(&body), None);

        // Truncation line present when truncated
        let trunc_body = Body {
            text: "body\n".to_string(),
            truncated: true,
            omitted_nodes: 5,
            omitted_edges: 12,
            caps: Caps {
                max_nodes: 200,
                max_edges: 400,
            },
        };
        assert_eq!(
            truncation_line(&trunc_body),
            Some("Diagram truncated: 5 nodes and 12 edges omitted (caps 200 nodes / 400 edges); run with the `text` option for the complete table.".to_string())
        );
    }

    #[test]
    fn raw_labeler_always_returns_none() {
        let labeler = RawLabeler;
        assert_eq!(labeler.component_label("core", &[]), None);
        assert_eq!(
            labeler.component_label("parser", &["topic".to_string()]),
            None
        );
    }
}
