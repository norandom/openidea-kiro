//! Plain-text rendering: graphs as one line per node followed by indented
//! outgoing edges under the same caps as the Mermaid form, findings as
//! `path:line: SEVERITY CODE message` lines in finding order, and Markdown
//! tables with `|` escaped inside cells.
//!
//! Design component `TextRenderer`, requirements 11.4, 11.5, 11.6, 11.8.

use super::{Caps, Graph, Rendered, apply_caps};
use crate::finding::{Finding, Severity, sort_findings};

/// One line per kept node `<label> [<id>]` followed by
/// `  -> <to label>[: <edge label>]` per outgoing kept edge, in input order;
/// same caps as the Mermaid form; trailer `… N more nodes, M more edges`
/// when the caps truncated the graph.
pub fn graph_text(graph: &Graph, caps: &Caps) -> Rendered {
    let capped = apply_caps(graph, caps);
    let mut out = String::new();
    for node in capped.nodes {
        out.push_str(&node.label);
        out.push_str(" [");
        out.push_str(&node.id);
        out.push_str("]\n");
        for edge in capped.edges.iter().filter(|edge| edge.from == node.id) {
            let to_label = capped
                .nodes
                .iter()
                .find(|n| n.id == edge.to)
                .map_or("", |n| n.label.as_str());
            match &edge.label {
                Some(label) => out.push_str(&format!("  -> {to_label}: {label}\n")),
                None => out.push_str(&format!("  -> {to_label}\n")),
            }
        }
    }
    let truncated = capped.omitted_nodes + capped.omitted_edges > 0;
    if truncated {
        out.push_str(&format!(
            "… {} more nodes, {} more edges\n",
            capped.omitted_nodes, capped.omitted_edges
        ));
    }
    Rendered {
        text: out,
        truncated,
        omitted_nodes: capped.omitted_nodes,
        omitted_edges: capped.omitted_edges,
    }
}

/// `<file>:<line>: <SEVERITY> <CODE> <message>` per finding in the finding
/// sort order (file, absent lines first, code, message); `:<line>` is
/// omitted when the finding has no line.
pub fn findings_text(findings: &[Finding]) -> String {
    let mut sorted = findings.to_vec();
    sort_findings(&mut sorted);
    let mut out = String::new();
    for finding in &sorted {
        match finding.line {
            Some(line) => out.push_str(&format!(
                "{}:{line}: {} {} {}\n",
                finding.file,
                severity_text(finding.severity),
                finding.code,
                finding.message
            )),
            None => out.push_str(&format!(
                "{}: {} {} {}\n",
                finding.file,
                severity_text(finding.severity),
                finding.code,
                finding.message
            )),
        }
    }
    out
}

/// Markdown table with the given headers; `|` in cells escaped as `\|`;
/// rows shorter than the headers are padded with empty cells.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let header_cells: Vec<String> = headers.iter().map(|h| escape_cell(h)).collect();
    let columns = header_cells.len();
    let mut out = row_line(&header_cells);
    let mut separator = String::from("|");
    for _ in 0..columns {
        separator.push_str(" --- |");
    }
    separator.push('\n');
    out.push_str(&separator);
    for row in rows {
        let mut cells: Vec<String> = row.iter().map(|c| escape_cell(c)).collect();
        for _ in cells.len()..columns {
            cells.push(String::new());
        }
        out.push_str(&row_line(&cells));
    }
    out
}

/// `| a | b |` line for one row of already-escaped cells.
fn row_line(cells: &[String]) -> String {
    let mut line = String::from("|");
    for cell in cells {
        line.push(' ');
        line.push_str(cell);
        line.push_str(" |");
    }
    line.push('\n');
    line
}

/// Escapes `|` so a cell cannot break out of its column.
fn escape_cell(cell: &str) -> String {
    cell.replace('|', "\\|")
}

/// Uppercase severity label for the text form.
fn severity_text(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "ERROR",
        Severity::Warning => "WARNING",
        Severity::Info => "INFO",
    }
}

#[cfg(test)]
mod tests {
    use crate::finding::{Finding, Severity};
    use crate::render::text::{findings_text, graph_text, table};
    use crate::render::{Caps, Direction, Edge, Graph, Node};

    fn graph() -> Graph {
        Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "req:1".into(),
                    label: "R1: Own a file".into(),
                    class: Some("req".into()),
                },
                Node {
                    id: "task:1.1".into(),
                    label: "Scan lines".into(),
                    class: None,
                },
            ],
            edges: vec![
                Edge {
                    from: "req:1".into(),
                    to: "task:1.1".into(),
                    label: None,
                },
                Edge {
                    from: "task:1.1".into(),
                    to: "req:1".into(),
                    label: Some("feeds".into()),
                },
            ],
        }
    }

    #[test]
    fn graph_lines_with_labelled_and_unlabelled_edges() {
        let rendered = graph_text(
            &graph(),
            &Caps {
                max_nodes: 10,
                max_edges: 10,
            },
        );
        assert_eq!(
            rendered.text,
            "R1: Own a file [req:1]\n  -> Scan lines\nScan lines [task:1.1]\n  -> R1: Own a file: feeds\n"
        );
        assert!(!rendered.truncated);
        assert_eq!((rendered.omitted_nodes, rendered.omitted_edges), (0, 0));
    }

    #[test]
    fn graph_truncation_appends_omitted_counts_trailer() {
        let mut g = graph();
        g.nodes.push(Node {
            id: "extra:1".into(),
            label: "Extra".into(),
            class: None,
        });
        g.edges.push(Edge {
            from: "extra:1".into(),
            to: "req:1".into(),
            label: None,
        });
        let rendered = graph_text(
            &g,
            &Caps {
                max_nodes: 2,
                max_edges: 10,
            },
        );
        assert!(rendered.truncated);
        assert_eq!((rendered.omitted_nodes, rendered.omitted_edges), (1, 1));
        assert_eq!(
            rendered.text,
            "R1: Own a file [req:1]\n  -> Scan lines\nScan lines [task:1.1]\n  -> R1: Own a file: feeds\n… 1 more nodes, 1 more edges\n"
        );
    }

    #[test]
    fn empty_graph_is_empty_text() {
        let rendered = graph_text(
            &Graph::default(),
            &Caps {
                max_nodes: 10,
                max_edges: 10,
            },
        );
        assert_eq!(rendered.text, "");
        assert!(!rendered.truncated);
    }

    #[test]
    fn findings_without_line_omit_the_line_number() {
        let finding = Finding::new(
            crate::finding::codes::MD_FENCE_UNCLOSED,
            "docs\\plan.md",
            "fence left open",
        );
        let out = findings_text(&[finding]);
        assert_eq!(
            out,
            "docs/plan.md: INFO MD_FENCE_UNCLOSED fence left open\n"
        );
    }

    #[test]
    fn findings_are_sorted_and_severity_is_uppercase() {
        let second = Finding::new(
            crate::finding::codes::SPEC_JSON_INVALID,
            ".kiro/specs/beta/spec.json",
            "not an object",
        )
        .at(3)
        .severity(Severity::Error);
        let first = Finding::new(
            crate::finding::codes::TASK_REQ_UNKNOWN,
            ".kiro/specs/beta/tasks.md",
            "unknown 1.9",
        )
        .at(2);
        let out = findings_text(&[second, first]);
        assert_eq!(
            out,
            ".kiro/specs/beta/spec.json:3: ERROR SPEC_JSON_INVALID not an object\n.kiro/specs/beta/tasks.md:2: WARNING TASK_REQ_UNKNOWN unknown 1.9\n"
        );
    }

    #[test]
    fn table_escapes_pipes_and_pads_short_rows() {
        let out = table(
            &["Feature", "Status"],
            &[vec!["ki|ro".into(), "done".into()], vec!["cli".into()]],
        );
        assert_eq!(
            out,
            "| Feature | Status |\n| --- | --- |\n| ki\\|ro | done |\n| cli |  |\n"
        );
    }
}
