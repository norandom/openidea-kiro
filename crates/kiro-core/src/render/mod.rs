//! The graph model and the machinery shared by both renderers (design
//! component `RenderGraph`, requirements 11.1, 11.3, 11.7, 11.8):
//! `Graph` with its nodes and edges, direction, size caps, safe unique
//! identifiers for Mermaid, the rendered result with its truncation flag and
//! omitted counts, and line clipping under a byte limit with an omitted-lines
//! trailer (optionally carrying a hint naming the full-output path).

pub mod mermaid;
pub mod text;

/// Graph direction, mapping to `graph TD` / `graph LR`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    TopDown,
    LeftRight,
}

/// One graph node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub class: Option<String>,
}

/// One directed edge with an optional label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

/// The renderer-independent graph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Size caps; no `Default` impl — `DEFAULT_CAPS` is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    pub max_nodes: usize,
    pub max_edges: usize,
}

pub const DEFAULT_CAPS: Caps = Caps {
    max_nodes: 200,
    max_edges: 400,
};

/// A render result with its truncation facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub text: String,
    pub truncated: bool,
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
}

/// The cap application result (crate-internal; consumed by the Mermaid and
/// text backends in tasks 3.2/3.3).
#[allow(dead_code)]
pub(crate) struct Capped<'a> {
    pub nodes: &'a [Node],
    pub edges: Vec<&'a Edge>,
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
}

/// Maps arbitrary ids to `[A-Za-z0-9_]+`, prefixing `n_` when the result
/// would start with a digit or be empty, and suffixing `_2`, `_3`, … on
/// collisions; stable for a given insertion order.
#[derive(Default)]
pub struct IdMap {
    assigned: std::collections::HashMap<String, String>,
    used: std::collections::HashSet<String>,
}

impl IdMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&mut self, id: &str) -> String {
        if let Some(existing) = self.assigned.get(id) {
            return existing.clone();
        }
        let sanitised: String = id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let mut safe = if sanitised.is_empty() {
            "n_".to_owned()
        } else if sanitised.starts_with(|c: char| c.is_ascii_digit()) {
            format!("n_{sanitised}")
        } else {
            sanitised
        };
        let mut candidate = safe.clone();
        let mut suffix = 1usize;
        while !self.used.insert(candidate.clone()) {
            suffix += 1;
            candidate = format!("{safe}_{suffix}");
        }
        if suffix > 1 {
            safe = candidate;
        }
        self.assigned.insert(id.to_owned(), safe.clone());
        safe
    }
}

/// First `max_nodes` nodes in input order; edges kept in input order when
/// both ends are kept, up to `max_edges` (crate-internal; consumed by the
/// backends in tasks 3.2/3.3).
#[allow(dead_code)]
pub(crate) fn apply_caps<'a>(graph: &'a Graph, caps: &Caps) -> Capped<'a> {
    let kept = graph.nodes.len().min(caps.max_nodes);
    let nodes = graph.nodes.get(..kept).unwrap_or(&[]);
    let mut edges = Vec::new();
    let mut edge_budget = caps.max_edges;
    for edge in &graph.edges {
        if edge_budget == 0 {
            break;
        }
        let from_kept = nodes.iter().any(|n| n.id == edge.from);
        let to_kept = nodes.iter().any(|n| n.id == edge.to);
        if from_kept && to_kept {
            edges.push(edge);
            edge_budget -= 1;
        }
    }
    Capped {
        nodes,
        omitted_nodes: graph.nodes.len() - nodes.len(),
        omitted_edges: graph.edges.len() - edges.len(),
        edges,
    }
}

/// Cuts at the last '\n' whose cut still leaves room for the trailer within
/// `max_bytes` and appends `"… {N} more lines"`, or
/// `"… {N} more lines — {hint}"` when a hint is supplied (amendment
/// 2026-11-04). When no cut fits, the whole text is replaced by the trailer
/// counting every line. Never panics and never cuts inside a character.
pub fn clip_lines(text: &str, max_bytes: usize, hint: Option<&str>) -> String {
    if text.len() <= max_bytes {
        return text.to_owned();
    }
    let trailer_for = |omitted: usize| {
        format!(
            "… {omitted} more lines{}",
            hint.map(|h| format!(" — {h}")).unwrap_or_default()
        )
    };
    let mut best: Option<(usize, String)> = None;
    for (idx, byte) in text.bytes().enumerate() {
        if byte != b'\n' {
            continue;
        }
        let candidate = idx + 1;
        let omitted = text[candidate..].lines().count();
        let trailer = trailer_for(omitted);
        if candidate + trailer.len() <= max_bytes {
            best = Some((candidate, trailer));
        }
    }
    match best {
        Some((cut, trailer)) => {
            let kept = text.get(..cut).unwrap_or_default().trim_end_matches('\n');
            format!("{kept}\n{trailer}")
        }
        None => {
            let total = text.lines().count().max(1);
            trailer_for(total)
        }
    }
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::render::{
        Caps, DEFAULT_CAPS, Direction, Edge, Graph, IdMap, Node, Rendered, clip_lines,
    };

    #[test]
    fn default_caps_values() {
        assert_eq!(
            DEFAULT_CAPS,
            Caps {
                max_nodes: 200,
                max_edges: 400
            }
        );
    }

    #[test]
    fn id_map_sanitises_prefixes_and_suffixes() {
        let mut ids = IdMap::new();
        assert_eq!(ids.get("req:1"), "req_1");
        assert_eq!(ids.get("crit:1.2"), "crit_1_2");
        assert_eq!(ids.get("2fast"), "n_2fast", "digit start gets the prefix");
        assert_eq!(ids.get(""), "n_", "empty gets the prefix");
        assert_eq!(
            ids.get("grün"),
            "gr_n",
            "non-ascii chars become underscores"
        );
    }

    #[test]
    fn id_map_collision_suffixes_are_stable_for_insertion_order() {
        let mut ids = IdMap::new();
        assert_eq!(ids.get("a b"), "a_b");
        assert_eq!(ids.get("a/b"), "a_b_2", "same sanitised id gets _2");
        assert_eq!(ids.get("a-b"), "a_b_3");
        assert_eq!(ids.get("a_b"), "a_b_4");
        assert_eq!(ids.get("a b"), "a_b", "repeated lookups are stable");
        // A fresh map with the same insertion order yields the same names.
        let mut fresh = IdMap::new();
        assert_eq!(fresh.get("a b"), "a_b");
        assert_eq!(fresh.get("a/b"), "a_b_2");
    }

    #[test]
    fn caps_keep_first_nodes_and_surviving_edges_in_order() {
        let graph = Graph {
            direction: Direction::LeftRight,
            nodes: vec![
                Node {
                    id: "a".into(),
                    label: "A".into(),
                    class: None,
                },
                Node {
                    id: "b".into(),
                    label: "B".into(),
                    class: None,
                },
                Node {
                    id: "c".into(),
                    label: "C".into(),
                    class: None,
                },
            ],
            edges: vec![
                Edge {
                    from: "a".into(),
                    to: "b".into(),
                    label: None,
                },
                Edge {
                    from: "b".into(),
                    to: "c".into(),
                    label: Some("x".into()),
                },
                Edge {
                    from: "a".into(),
                    to: "c".into(),
                    label: None,
                },
            ],
        };
        let capped = crate::render::apply_caps(
            &graph,
            &Caps {
                max_nodes: 2,
                max_edges: 10,
            },
        );
        assert_eq!(capped.nodes.len(), 2, "first two nodes kept");
        assert_eq!(capped.omitted_nodes, 1);
        assert_eq!(capped.edges.len(), 1, "edge to the dropped node is omitted");
        assert_eq!(
            capped.omitted_edges, 2,
            "endpoint-dropped edges count as omitted"
        );
        // Edge cap alone.
        let capped = crate::render::apply_caps(
            &graph,
            &Caps {
                max_nodes: 10,
                max_edges: 1,
            },
        );
        assert_eq!(capped.edges.len(), 1);
        assert_eq!(capped.omitted_edges, 2);
    }

    #[test]
    fn clip_lines_at_exact_boundary_is_unchanged() {
        let text = "abc\ndef\n";
        assert_eq!(clip_lines(text, text.len(), None), text);
        assert_eq!(clip_lines(text, text.len(), Some("hint")), text);
    }

    #[test]
    fn clip_lines_cuts_at_newline_with_omitted_count() {
        let text = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight";
        assert!(text.len() > 26);
        let clipped = clip_lines(text, 26, None);
        assert_eq!(clipped, "one\ntwo\n… 6 more lines");
        assert!(
            clipped.len() <= 26,
            "the result fits the budget including the trailer"
        );
        let long = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\neleven\ntwelve";
        let clipped = clip_lines(long, 60, Some("run x.insert for the full text"));
        assert_eq!(
            clipped,
            "one\ntwo\n… 10 more lines — run x.insert for the full text"
        );
        assert!(clipped.len() <= 60);
    }

    #[test]
    fn clip_lines_leaves_room_for_the_trailer() {
        // The long hint cannot fit with any kept prefix at a small budget,
        // so the whole text is replaced by the trailer counting every line.
        let text = "one\ntwo\nthree\nfour";
        let clipped = clip_lines(text, 12, Some("a fairly long hint"));
        assert_eq!(clipped, "… 4 more lines — a fairly long hint");
        let clipped = clip_lines(text, 12, None);
        assert_eq!(
            clipped, "… 4 more lines",
            "no cut fits at 12 bytes without a hint either"
        );
    }

    #[test]
    fn clip_lines_without_newline_before_the_limit() {
        // No newline to cut at: the whole text is replaced by the trailer.
        let text = "abcdefghij";
        let clipped = clip_lines(text, 5, None);
        assert_eq!(clipped, "… 1 more lines");
    }

    #[test]
    fn clip_lines_is_multibyte_safe() {
        let text = "äöü\nääää\nbbbb\ncccc\n";
        let clipped = clip_lines(text, 20, None);
        assert!(clipped.ends_with("more lines"));
        // No panic and no interior char boundary cuts.
        assert!(clipped.chars().all(|c| c != '\u{FFFD}'));
    }

    #[test]
    fn rendered_carries_truncation_and_counts() {
        let r = Rendered {
            text: String::new(),
            truncated: true,
            omitted_nodes: 3,
            omitted_edges: 4,
        };
        assert!(r.truncated);
        assert_eq!((r.omitted_nodes, r.omitted_edges), (3, 4));
    }
}
