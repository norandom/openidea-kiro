//! `Graph` → Mermaid `graph TD|LR` text that renders in strict mode (design
//! component `MermaidRenderer`, requirements 11.1–11.3): quoted node labels
//! with `#quot;`/`#lt;`/`#gt;` escaping, class lines, optional edge labels,
//! and one ellipsis node stating the omitted counts when the caps truncated
//! the graph. Output always ends with a single `\n`.

use super::{Caps, Direction, Graph, IdMap, Rendered, apply_caps};

/// Renders `graph` as Mermaid under `caps`.
pub fn mermaid(graph: &Graph, caps: &Caps) -> Rendered {
    let capped = apply_caps(graph, caps);
    let mut ids = IdMap::new();
    let mut classes = IdMap::new();
    let mut out = String::new();
    out.push_str(match graph.direction {
        Direction::TopDown => "graph TD",
        Direction::LeftRight => "graph LR",
    });
    out.push('\n');
    for node in capped.nodes {
        let id = ids.get(&node.id);
        let label = escape(&node.label);
        out.push_str(&format!("    {id}[\"{label}\"]\n"));
        if let Some(class) = &node.class {
            let class_name = classes.get(class);
            out.push_str(&format!("    class {id} {class_name}\n"));
        }
    }
    for edge in &capped.edges {
        let from = ids.get(&edge.from);
        let to = ids.get(&edge.to);
        match &edge.label {
            Some(label) => out.push_str(&format!("    {from} -->|\"{}\"| {to}\n", escape(label))),
            None => out.push_str(&format!("    {from} --> {to}\n")),
        }
    }
    let truncated = capped.omitted_nodes + capped.omitted_edges > 0;
    if truncated {
        let id = ids.get("n more");
        out.push_str(&format!(
            "    {id}[\"… {} more nodes, {} more edges\"]\n",
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

/// Mermaid label escaping: `"` → `#quot;`, `<` → `#lt;`, `>` → `#gt;`,
/// newlines → spaces.
fn escape(label: &str) -> String {
    label
        .replace('\n', " ")
        .replace('"', "#quot;")
        .replace('<', "#lt;")
        .replace('>', "#gt;")
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::render::mermaid::mermaid;
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
    fn top_down_graph_with_classes_and_edges() {
        let rendered = mermaid(
            &graph(),
            &Caps {
                max_nodes: 10,
                max_edges: 10,
            },
        );
        assert_eq!(
            rendered.text,
            "graph TD\n    req_1[\"R1: Own a file\"]\n    class req_1 req\n    task_1_1[\"Scan lines\"]\n    req_1 --> task_1_1\n    task_1_1 -->|\"feeds\"| req_1\n"
        );
        assert!(!rendered.truncated);
        assert_eq!((rendered.omitted_nodes, rendered.omitted_edges), (0, 0));
    }

    #[test]
    fn left_right_direction_header() {
        let mut g = graph();
        g.direction = Direction::LeftRight;
        let rendered = mermaid(
            &g,
            &Caps {
                max_nodes: 10,
                max_edges: 10,
            },
        );
        assert!(rendered.text.starts_with("graph LR\n"));
    }

    #[test]
    fn labels_escape_specials_and_newlines() {
        let g = Graph {
            direction: Direction::TopDown,
            nodes: vec![Node {
                id: "a".into(),
                label: "quote \" angle <bold> newline\nhere".into(),
                class: None,
            }],
            edges: vec![],
        };
        let rendered = mermaid(
            &g,
            &Caps {
                max_nodes: 10,
                max_edges: 10,
            },
        );
        assert_eq!(
            rendered.text,
            "graph TD\n    a[\"quote #quot; angle #lt;bold#gt; newline here\"]\n"
        );
        let label_line = rendered.text.lines().nth(1).unwrap_or_default();
        assert!(!label_line.contains('<'), "no raw angle bracket in labels");
        assert!(
            !label_line.contains('>'),
            "no raw closing angle bracket in labels"
        );
    }

    #[test]
    fn truncation_yields_exactly_one_ellipsis_node() {
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
        let rendered = mermaid(
            &g,
            &Caps {
                max_nodes: 2,
                max_edges: 10,
            },
        );
        assert!(rendered.truncated);
        assert_eq!(rendered.omitted_nodes, 1);
        assert_eq!(
            rendered.omitted_edges, 1,
            "edges touching the dropped node are omitted"
        );
        let ellipsis_lines: Vec<&str> = rendered
            .text
            .lines()
            .filter(|l| l.contains("more nodes"))
            .collect();
        assert_eq!(
            ellipsis_lines,
            ["    n_more[\"… 1 more nodes, 1 more edges\"]"]
        );
        assert!(rendered.text.ends_with('\n'));
        assert!(!rendered.text.ends_with("\n\n"), "single trailing newline");
    }

    #[test]
    fn empty_graph_is_just_the_header() {
        let rendered = mermaid(
            &Graph::default(),
            &Caps {
                max_nodes: 10,
                max_edges: 10,
            },
        );
        assert_eq!(rendered.text, "graph TD\n");
        assert!(!rendered.truncated);
    }
}
