//! Task dependency graph (`_Depends:_`, ordering, `(P)`, completion) and
//! summary statistics (design component `TaskGraph`, requirements 5.1–5.8, 7.1).
//!
//! Renders the task graph top-down, with containment edges, sequential ordering
//! edges (skipping parallel tasks at both major and sub-task level), and explicit
//! dependency edges.

use crate::render::{Direction, Edge, Graph, Node};
use crate::tasks::TasksDoc;

use super::{Body, task_label, truncation_line};

/// Summary facts and statistics for the task dependency view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskStats {
    pub total: usize,
    pub checked: usize,
    pub majors: usize,
    pub parallel: usize,
    pub optional: usize,
    pub depends: usize,
    pub unknown_depends: usize,
    pub order: usize,
    pub contains: usize,
}

/// The complete task dependency view with its graph model and statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasksView {
    pub spec: String,
    pub graph: Graph,
    pub stats: TaskStats,
}

/// Builds a `TasksView` from a spec name and a `TasksDoc`.
pub fn tasks_view(spec: &str, doc: &TasksDoc) -> TasksView {
    let mut graph = Graph {
        direction: Direction::TopDown,
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    let mut stats = TaskStats {
        total: doc.counts.total,
        checked: doc.counts.checked,
        majors: doc.tasks.len(),
        parallel: doc.all().filter(|t| t.parallel).count(),
        optional: doc.all().filter(|t| t.optional).count(),
        ..Default::default()
    };

    // 1. Nodes in doc.all() order
    for t in doc.all() {
        let is_major = t.sub.is_none() && doc.tasks.iter().any(|m| m.id == t.id);
        let class = if is_major {
            if t.checked { "major_done" } else { "major" }
        } else if t.checked {
            "task_done"
        } else {
            "task"
        };

        graph.nodes.push(Node {
            id: format!("task:{}", t.id),
            label: task_label(t),
            class: Some(class.to_string()),
        });
    }

    // 2. Edges in order:
    // (1) Containment edges: major -> sub (no label)
    for major in &doc.tasks {
        let major_id = format!("task:{}", major.id);
        for sub in &major.subtasks {
            graph.edges.push(Edge {
                from: major_id.clone(),
                to: format!("task:{}", sub.id),
                label: None,
            });
            stats.contains += 1;
        }
    }

    // (2) Ordering edges:
    // Consecutive top-level entries unless next is parallel
    for pair in doc.tasks.windows(2) {
        if let (Some(a), Some(b)) = (pair.first(), pair.get(1))
            && !b.parallel
        {
            graph.edges.push(Edge {
                from: format!("task:{}", a.id),
                to: format!("task:{}", b.id),
                label: Some("order".to_string()),
            });
            stats.order += 1;
        }
    }

    // Consecutive sub-tasks of each major unless next is parallel
    for major in &doc.tasks {
        for pair in major.subtasks.windows(2) {
            if let (Some(a), Some(b)) = (pair.first(), pair.get(1))
                && !b.parallel
            {
                graph.edges.push(Edge {
                    from: format!("task:{}", a.id),
                    to: format!("task:{}", b.id),
                    label: Some("order".to_string()),
                });
                stats.order += 1;
            }
        }
    }

    // (3) Explicit depends edges:
    for t in doc.all() {
        let to_id = format!("task:{}", t.id);
        for entry in &t.depends {
            if doc.task(entry).is_some() {
                graph.edges.push(Edge {
                    from: format!("task:{entry}"),
                    to: to_id.clone(),
                    label: Some("depends".to_string()),
                });
                stats.depends += 1;
            } else {
                stats.unknown_depends += 1;
            }
        }
    }

    TasksView {
        spec: spec.to_string(),
        graph,
        stats,
    }
}

/// Generates the summary lines for `viz.tasks`.
pub fn tasks_summary(view: &TasksView, body: &Body) -> Vec<String> {
    let s = &view.stats;
    let mut lines = Vec::with_capacity(3);
    lines.push(format!(
        "Tasks for {}: {}/{} done; {} majors; parallel {}; optional {}",
        view.spec, s.checked, s.total, s.majors, s.parallel, s.optional
    ));
    lines.push(format!(
        "Dependencies: {} explicit ({} unknown); {} ordering edges; {} containment edges",
        s.depends, s.unknown_depends, s.order, s.contains
    ));

    if let Some(trunc) = truncation_line(body) {
        lines.push(trunc);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Caps;
    use crate::tasks;
    use crate::viz::{Format, render_body};

    #[test]
    fn ordering_skips_parallel_at_both_levels() {
        let doc = tasks::parse(
            "- [ ] 1. Major one\n  - [ ] 1.1 First sub\n  - [ ] 1.2 (P) Second parallel sub\n  - [ ] 1.3 Third sub\n- [ ]* 2. (P) Major two parallel\n- [ ] 3. Major three\n",
            "tasks.md",
        );

        let view = tasks_view("order_test", &doc);
        assert_eq!(view.graph.direction, Direction::TopDown);

        // Ordering edges:
        // Top-level: 1 -> 2 is skipped because 2 is parallel!
        // 2 -> 3 is NOT skipped (3 is not parallel), so 2 -> 3 exists.
        // Sub-tasks of Major 1: 1.1 -> 1.2 is skipped because 1.2 is parallel!
        // 1.2 -> 1.3 is NOT skipped (1.3 is not parallel), so 1.2 -> 1.3 exists.
        let order_edges: Vec<(&str, &str)> = view
            .graph
            .edges
            .iter()
            .filter(|e| e.label.as_deref() == Some("order"))
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();

        assert_eq!(
            order_edges,
            vec![("task:2", "task:3"), ("task:1.2", "task:1.3")]
        );
        assert_eq!(view.stats.order, 2);
    }

    #[test]
    fn containment_edges() {
        let doc = tasks::parse(
            "- [ ] 1. Major one\n  - [x] 1.1 Sub one\n  - [ ] 1.2 Sub two\n- [ ] 2. Major without subtasks\n",
            "tasks.md",
        );

        let view = tasks_view("contain_test", &doc);
        let contain_edges: Vec<(&str, &str)> = view
            .graph
            .edges
            .iter()
            .filter(|e| e.label.is_none())
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();

        assert_eq!(
            contain_edges,
            vec![("task:1", "task:1.1"), ("task:1", "task:1.2")]
        );
        assert_eq!(view.stats.contains, 2);
    }

    #[test]
    fn known_unknown_and_self_referencing_dependencies() {
        let doc = tasks::parse(
            "- [ ] 1. Major one\n  - [ ] 1.1 Sub one\n    _Depends: 1.1_\n  - [ ] 1.2 Sub two\n    _Depends: 1.1, 9.9_\n",
            "tasks.md",
        );

        let view = tasks_view("dep_test", &doc);

        // 1.1 depends on 1.1 -> self-loop edge!
        // 1.2 depends on 1.1 -> known edge
        // 1.2 depends on 9.9 -> unknown depends (counted, no edge)
        let depends_edges: Vec<(&str, &str)> = view
            .graph
            .edges
            .iter()
            .filter(|e| e.label.as_deref() == Some("depends"))
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();

        assert_eq!(
            depends_edges,
            vec![("task:1.1", "task:1.1"), ("task:1.1", "task:1.2")]
        );
        assert_eq!(view.stats.depends, 2);
        assert_eq!(view.stats.unknown_depends, 1);
    }

    #[test]
    fn node_classes_and_summary_lines() {
        let doc = tasks::parse(
            "- [x] 1. Major one\n  - [x] 1.1 Sub one\n  - [ ] 1.2 (P) Sub two\n- [ ]* 2. Major two\n",
            "tasks.md",
        );

        let view = tasks_view("stats_spec", &doc);
        assert_eq!(view.stats.total, 4);
        assert_eq!(view.stats.checked, 2);
        assert_eq!(view.stats.majors, 2);
        assert_eq!(view.stats.parallel, 1);
        assert_eq!(view.stats.optional, 1);

        // Check node classes:
        // 1 is major_done
        // 1.1 is task_done
        // 1.2 is task
        // 2 is major
        let node_classes: Vec<(&str, Option<&str>)> = view
            .graph
            .nodes
            .iter()
            .map(|n| (n.id.as_str(), n.class.as_deref()))
            .collect();

        assert_eq!(
            node_classes,
            vec![
                ("task:1", Some("major_done")),
                ("task:1.1", Some("task_done")),
                ("task:1.2", Some("task")),
                ("task:2", Some("major")),
            ]
        );

        let caps = Caps {
            max_nodes: 200,
            max_edges: 400,
        };
        let body = render_body(&view.graph, Format::Mermaid, &caps);
        let summary = tasks_summary(&view, &body);

        assert_eq!(
            summary.first().map(String::as_str),
            Some("Tasks for stats_spec: 2/4 done; 2 majors; parallel 1; optional 1")
        );
        assert_eq!(
            summary.get(1).map(String::as_str),
            Some("Dependencies: 0 explicit (0 unknown); 1 ordering edges; 2 containment edges")
        );
        assert_eq!(summary.len(), 2);

        // With truncation
        let trunc_caps = Caps {
            max_nodes: 2,
            max_edges: 1,
        };
        let trunc_body = render_body(&view.graph, Format::Mermaid, &trunc_caps);
        let trunc_summary = tasks_summary(&view, &trunc_body);
        assert_eq!(trunc_summary.len(), 3);
        assert_eq!(
            trunc_summary.get(2).map(String::as_str),
            Some(
                "Diagram truncated: 2 nodes and 2 edges omitted (caps 2 nodes / 1 edges); run with the `text` option for the complete table."
            )
        );
    }
}
