//! Spec graph view (requirements → criteria → tasks → components → files) and
//! summary statistics (design component `SpecGraph`, requirements 4.1–4.7, 5.8,
//! 6.1, 6.4, 7.1).
//!
//! Renders the spec graph layer by layer, tracking unresolved references against
//! present documents, unscoped leaf tasks, and unassigned files.

use std::collections::{BTreeSet, HashSet};

use crate::join;
use crate::render::{Direction, Edge, Graph, Node};
use crate::requirements::ReqKind;
use crate::source::Feature;
use crate::spec::Doc;
use crate::tasks::{Task, TasksDoc};

use super::{Body, Labeler, doc_file_name, task_label, truncate_label, truncation_line};

/// Summary facts and counts for the spec graph view.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphStats {
    pub requirements: usize,
    pub criteria: usize,
    pub tasks: usize,
    pub components: usize,
    pub files: usize,
    pub edges: usize,
    pub unscoped_tasks: usize,
    pub unassigned_files: usize,
    pub unresolved_refs: usize,
    pub missing: Vec<Doc>,
}

/// The complete spec graph view with its graph model and statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphView {
    pub spec: String,
    pub graph: Graph,
    pub stats: GraphStats,
}

/// Returns leaf tasks (sub-tasks, plus majors without sub-tasks) in document order.
pub fn leaf_tasks(doc: &TasksDoc) -> Vec<&Task> {
    let mut leaves = Vec::new();
    for major in &doc.tasks {
        if major.subtasks.is_empty() {
            leaves.push(major);
        } else {
            for sub in &major.subtasks {
                leaves.push(sub);
            }
        }
    }
    leaves
}

/// Builds a `GraphView` from a loaded `Feature` and a component `Labeler`.
///
/// Returns `None` when `requirements.md`, `design.md`, and `tasks.md` are all absent.
pub fn graph_view(feature: &Feature, labeler: &dyn Labeler) -> Option<GraphView> {
    if feature.requirements.is_none() && feature.design.is_none() && feature.tasks.is_none() {
        return None;
    }

    let mut graph = Graph {
        direction: Direction::LeftRight,
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let mut seen_nodes = BTreeSet::new();

    let mut stats = GraphStats::default();

    // 1. Missing documents in Doc order (Requirements, Design, Tasks)
    if feature.requirements.is_none() {
        stats.missing.push(Doc::Requirements);
    }
    if feature.design.is_none() {
        stats.missing.push(Doc::Design);
    }
    if feature.tasks.is_none() {
        stats.missing.push(Doc::Tasks);
    }

    // --- NODES LAYER BY LAYER ---

    // Layer 1: Requirements and Criteria
    if let Some(req_doc) = &feature.requirements {
        for r in &req_doc.requirements {
            let req_id = format!("req:{}", r.id);
            if seen_nodes.insert(req_id.clone()) {
                let title_truncated = truncate_label(&r.title);
                let label = match r.kind {
                    ReqKind::Baseline => {
                        if title_truncated.is_empty() {
                            format!("R{}", r.id)
                        } else {
                            format!("R{}: {title_truncated}", r.id)
                        }
                    }
                    ReqKind::Amendment => {
                        if title_truncated.is_empty() {
                            r.id.clone()
                        } else {
                            format!("{}: {title_truncated}", r.id)
                        }
                    }
                };
                graph.nodes.push(Node {
                    id: req_id,
                    label,
                    class: Some("req".to_string()),
                });
                stats.requirements += 1;
            }

            for c in &r.criteria {
                let crit_id = format!("crit:{}", c.id);
                if seen_nodes.insert(crit_id.clone()) {
                    let text_truncated = truncate_label(&c.text);
                    let label = format!("{}: {text_truncated}", c.id);
                    graph.nodes.push(Node {
                        id: crit_id,
                        label,
                        class: Some("crit".to_string()),
                    });
                    stats.criteria += 1;
                }
            }
        }
    }

    // Layer 2: Leaf Tasks
    let leaves = feature.tasks.as_ref().map(leaf_tasks).unwrap_or_default();
    let leaf_ids: HashSet<String> = leaves.iter().map(|t| t.id.clone()).collect();
    for t in &leaves {
        let task_node_id = format!("task:{}", t.id);
        if seen_nodes.insert(task_node_id.clone()) {
            let label = task_label(t);
            let class = if t.checked { "task_done" } else { "task" };
            graph.nodes.push(Node {
                id: task_node_id,
                label,
                class: Some(class.to_string()),
            });
            stats.tasks += 1;
        }
    }

    // Layer 3: Components and Files
    if let Some(design_doc) = &feature.design {
        let topics = feature
            .meta
            .as_ref()
            .map_or(&[][..], |m| m.topics.as_slice());
        for c in &design_doc.components {
            let comp_id = format!("comp:{}", c.name);
            if seen_nodes.insert(comp_id.clone()) {
                let label = labeler
                    .component_label(&c.name, topics)
                    .unwrap_or_else(|| c.name.clone());
                graph.nodes.push(Node {
                    id: comp_id,
                    label,
                    class: Some("comp".to_string()),
                });
                stats.components += 1;
            }

            for file in design_doc.files_of(&c.name) {
                let file_id = format!("file:{}", file.path);
                if seen_nodes.insert(file_id.clone()) {
                    graph.nodes.push(Node {
                        id: file_id,
                        label: file.path.clone(),
                        class: Some("file".to_string()),
                    });
                    stats.files += 1;
                }
            }
        }
        stats.unassigned_files = design_doc.unassigned_files().count();
    }

    // --- EDGES IN LAYER ORDER ---

    // 1. req -> crit
    if let Some(req_doc) = &feature.requirements {
        for r in &req_doc.requirements {
            let req_id = format!("req:{}", r.id);
            for c in &r.criteria {
                let crit_id = format!("crit:{}", c.id);
                graph.edges.push(Edge {
                    from: req_id.clone(),
                    to: crit_id,
                    label: None,
                });
            }
        }
    }

    // 2. crit or whole req -> task
    for t in &leaves {
        let task_id = format!("task:{}", t.id);
        for entry in &t.requirements {
            if let Some(req_doc) = &feature.requirements {
                if entry.contains('.') {
                    if req_doc.criterion(entry).is_some() {
                        graph.edges.push(Edge {
                            from: format!("crit:{entry}"),
                            to: task_id.clone(),
                            label: None,
                        });
                    } else {
                        stats.unresolved_refs += 1;
                    }
                } else if req_doc.requirement(entry).is_some() {
                    graph.edges.push(Edge {
                        from: format!("req:{entry}"),
                        to: task_id.clone(),
                        label: None,
                    });
                } else {
                    stats.unresolved_refs += 1;
                }
            }
        }
    }

    // 3. task -> comp
    for t in &leaves {
        let task_id = format!("task:{}", t.id);
        if t.boundary.is_empty() {
            stats.unscoped_tasks += 1;
        } else if let Some(design_doc) = &feature.design {
            for entry in &t.boundary {
                if let Some(comp) = design_doc.component(entry) {
                    graph.edges.push(Edge {
                        from: task_id.clone(),
                        to: format!("comp:{}", comp.name),
                        label: None,
                    });
                }
            }
        }
    }

    // Unresolved boundary names counted via join::coverage filtered to leaf tasks
    if feature.design.is_some() && feature.tasks.is_some() {
        let cov = join::coverage(feature.design.as_ref(), feature.tasks.as_ref());
        for (task_ref, unknown_names) in &cov.unknown_boundary_tasks {
            if leaf_ids.contains(&task_ref.id) {
                stats.unresolved_refs += unknown_names.len();
            }
        }
    }

    // 4. comp -> file
    if let Some(design_doc) = &feature.design {
        for c in &design_doc.components {
            let comp_id = format!("comp:{}", c.name);
            for file in design_doc.files_of(&c.name) {
                graph.edges.push(Edge {
                    from: comp_id.clone(),
                    to: format!("file:{}", file.path),
                    label: None,
                });
            }
        }
    }

    stats.edges = graph.edges.len();

    Some(GraphView {
        spec: feature.name.clone(),
        graph,
        stats,
    })
}

/// Generates the summary lines for `viz.graph`.
pub fn graph_summary(view: &GraphView, body: &Body) -> Vec<String> {
    let s = &view.stats;
    let mut lines = Vec::with_capacity(4);
    lines.push(format!(
        "Graph for {}: {} requirements, {} criteria, {} tasks, {} components, {} files; {} edges",
        view.spec, s.requirements, s.criteria, s.tasks, s.components, s.files, s.edges
    ));
    lines.push(format!(
        "Unscoped tasks: {}; unassigned files: {}; unresolved references: {}",
        s.unscoped_tasks, s.unassigned_files, s.unresolved_refs
    ));

    if !s.missing.is_empty() {
        let missing_names: Vec<&str> = s.missing.iter().map(|d| doc_file_name(*d)).collect();
        lines.push(format!("Missing: {}", missing_names.join(", ")));
    }

    if let Some(trunc) = truncation_line(body) {
        lines.push(trunc);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design;
    use crate::render::Caps;
    use crate::requirements;
    use crate::source::Feature;
    use crate::tasks;
    use crate::viz::{Format, RawLabeler, render_body};

    struct TestLabeler {
        target: String,
        replacement: String,
    }

    impl Labeler for TestLabeler {
        fn component_label(&self, raw: &str, _topics: &[String]) -> Option<String> {
            if raw == self.target {
                Some(self.replacement.clone())
            } else {
                None
            }
        }
    }

    #[test]
    fn nothing_to_draw_when_all_three_docs_absent() {
        let feature = Feature {
            name: "empty".to_string(),
            dir: ".kiro/specs/empty".to_string(),
            meta: None,
            requirements: None,
            design: None,
            tasks: None,
            findings: Vec::new(),
        };

        assert_eq!(graph_view(&feature, &RawLabeler), None);
    }

    #[test]
    fn leaf_task_rule() {
        let tasks_doc = tasks::parse(
            "- [ ] 1. Major one\n  - [x] 1.1 Sub one\n  - [ ] 1.2 Sub two\n- [ ] 2. Major without subtasks\n",
            "tasks.md",
        );
        let leaves = leaf_tasks(&tasks_doc);
        let ids: Vec<&str> = leaves.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["1.1", "1.2", "2"]);
    }

    #[test]
    fn whole_requirement_versus_criterion_references_and_layer_order() {
        let req_doc = requirements::parse(
            "### Requirement 1: Whole\n\n#### Acceptance Criteria\n1. Crit A\n2. Crit B\n\n### Requirement 2: Another\n\n#### Acceptance Criteria\n1. Crit C\n",
            "requirements.md",
        );
        let design_doc = design::parse(
            "## File Structure Plan\n\n| Component | File |\n| --- | --- |\n| Core | `src/lib.rs` |\n",
            "design.md",
        );
        let tasks_doc = tasks::parse(
            "- [ ] 1. Major\n  - [x] 1.1 Task referencing whole req\n    _Requirements: 1_\n    _Boundary: Core_\n  - [ ] 1.2 Task referencing criterion\n    _Requirements: 2.1_\n    _Boundary: Core_\n",
            "tasks.md",
        );

        let feature = Feature {
            name: "demo".to_string(),
            dir: ".kiro/specs/demo".to_string(),
            meta: None,
            requirements: Some(req_doc),
            design: Some(design_doc),
            tasks: Some(tasks_doc),
            findings: Vec::new(),
        };

        let view = graph_view(&feature, &RawLabeler);
        assert!(view.is_some());
        let Some(view) = view else {
            return;
        };
        assert_eq!(view.graph.direction, Direction::LeftRight);
        assert_eq!(view.stats.requirements, 2);
        assert_eq!(view.stats.criteria, 3);
        assert_eq!(view.stats.tasks, 2);
        assert_eq!(view.stats.components, 1);
        assert_eq!(view.stats.files, 1);
        assert_eq!(view.stats.unresolved_refs, 0);
        assert_eq!(view.stats.unscoped_tasks, 0);
        assert_eq!(view.stats.unassigned_files, 0);
        assert!(view.stats.missing.is_empty());

        // Verify node layer order: reqs, crits, tasks, comps, files
        let node_ids: Vec<&str> = view.graph.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(
            node_ids,
            vec![
                "req:1",
                "crit:1.1",
                "crit:1.2",
                "req:2",
                "crit:2.1",
                "task:1.1",
                "task:1.2",
                "comp:Core",
                "file:src/lib.rs",
            ]
        );

        // Verify edges:
        // 1. req -> crit (1 -> 1.1, 1 -> 1.2, 2 -> 2.1)
        // 2. whole req 1 -> task 1.1
        // 3. crit 2.1 -> task 1.2
        // 4. task 1.1 -> comp Core, task 1.2 -> comp Core
        // 5. comp Core -> file src/lib.rs
        let edge_pairs: Vec<(&str, &str)> = view
            .graph
            .edges
            .iter()
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();
        assert_eq!(
            edge_pairs,
            vec![
                ("req:1", "crit:1.1"),
                ("req:1", "crit:1.2"),
                ("req:2", "crit:2.1"),
                ("req:1", "task:1.1"),
                ("crit:2.1", "task:1.2"),
                ("task:1.1", "comp:Core"),
                ("task:1.2", "comp:Core"),
                ("comp:Core", "file:src/lib.rs"),
            ]
        );
    }

    #[test]
    fn unresolved_counting_only_against_present_documents() {
        // Case 1: Requirements is absent -> requirement annotations on tasks are skipped, NOT unresolved
        let design_doc = design::parse(
            "## File Structure Plan\n\n| Component | File |\n| --- | --- |\n| Core | `src/lib.rs` |\n",
            "design.md",
        );
        let tasks_doc = tasks::parse(
            "- [ ] 1. Leaf task\n  _Requirements: 9.9, 8_\n  _Boundary: UnknownBoundary_\n",
            "tasks.md",
        );
        let feature_no_req = Feature {
            name: "test".to_string(),
            dir: ".kiro/specs/test".to_string(),
            meta: None,
            requirements: None,
            design: Some(design_doc),
            tasks: Some(tasks_doc),
            findings: Vec::new(),
        };

        let view1 = graph_view(&feature_no_req, &RawLabeler);
        assert!(view1.is_some());
        let Some(view1) = view1 else {
            return;
        };
        // UnknownBoundary resolves against present design doc -> 1 unresolved ref
        assert_eq!(view1.stats.unresolved_refs, 1);
        assert_eq!(view1.stats.missing, vec![Doc::Requirements]);

        // Case 2: Design is absent -> boundary annotations are skipped, NOT unresolved
        let req_doc = requirements::parse(
            "### Requirement 1: Found\n\n#### Acceptance Criteria\n1. Found crit\n",
            "requirements.md",
        );
        let tasks_doc2 = tasks::parse(
            "- [ ] 1. Leaf task\n  _Requirements: 1.1, 9.9_\n  _Boundary: UnknownBoundary_\n",
            "tasks.md",
        );
        let feature_no_design = Feature {
            name: "test".to_string(),
            dir: ".kiro/specs/test".to_string(),
            meta: None,
            requirements: Some(req_doc),
            design: None,
            tasks: Some(tasks_doc2),
            findings: Vec::new(),
        };

        let view2 = graph_view(&feature_no_design, &RawLabeler);
        assert!(view2.is_some());
        let Some(view2) = view2 else {
            return;
        };
        // Requirements 9.9 fails against present requirements -> 1 unresolved ref
        // Boundary UnknownBoundary is skipped because design is absent
        assert_eq!(view2.stats.unresolved_refs, 1);
        assert_eq!(view2.stats.missing, vec![Doc::Design]);
    }

    #[test]
    fn labeler_changes_component_label_not_node_identity() {
        let design_doc = design::parse(
            "## File Structure Plan\n\n| Component | File |\n| --- | --- |\n| CoreEngine | `src/engine.rs` |\n",
            "design.md",
        );
        let feature = Feature {
            name: "test".to_string(),
            dir: ".kiro/specs/test".to_string(),
            meta: None,
            requirements: None,
            design: Some(design_doc),
            tasks: None,
            findings: Vec::new(),
        };

        let labeler = TestLabeler {
            target: "CoreEngine".to_string(),
            replacement: "Preferred Engine Label".to_string(),
        };

        let view = graph_view(&feature, &labeler);
        assert!(view.is_some());
        let Some(view) = view else {
            return;
        };
        let comp_node = view.graph.nodes.iter().find(|n| n.id == "comp:CoreEngine");
        assert!(comp_node.is_some());
        let Some(comp_node) = comp_node else {
            return;
        };
        assert_eq!(comp_node.label, "Preferred Engine Label");
        assert_eq!(comp_node.id, "comp:CoreEngine");
    }

    #[test]
    fn summary_lines_untruncated_and_truncated() {
        let req_doc = requirements::parse(
            "### Requirement 1: Test\n\n#### Acceptance Criteria\n1. Crit\n",
            "requirements.md",
        );
        let feature = Feature {
            name: "alpha".to_string(),
            dir: ".kiro/specs/alpha".to_string(),
            meta: None,
            requirements: Some(req_doc),
            design: None,
            tasks: None,
            findings: Vec::new(),
        };

        let view = graph_view(&feature, &RawLabeler);
        assert!(view.is_some());
        let Some(view) = view else {
            return;
        };
        let caps = Caps {
            max_nodes: 200,
            max_edges: 400,
        };
        let body = render_body(&view.graph, Format::Mermaid, &caps);
        let summary = graph_summary(&view, &body);

        assert_eq!(
            summary.first().map(String::as_str),
            Some(
                "Graph for alpha: 1 requirements, 1 criteria, 0 tasks, 0 components, 0 files; 1 edges"
            )
        );
        assert_eq!(
            summary.get(1).map(String::as_str),
            Some("Unscoped tasks: 0; unassigned files: 0; unresolved references: 0")
        );
        assert_eq!(
            summary.get(2).map(String::as_str),
            Some("Missing: design.md, tasks.md")
        );
        assert_eq!(summary.len(), 3);

        // With truncation
        let trunc_caps = Caps {
            max_nodes: 1,
            max_edges: 0,
        };
        let trunc_body = render_body(&view.graph, Format::Mermaid, &trunc_caps);
        let trunc_summary = graph_summary(&view, &trunc_body);
        assert_eq!(trunc_summary.len(), 4);
        assert_eq!(
            trunc_summary.get(3).map(String::as_str),
            Some(
                "Diagram truncated: 1 nodes and 1 edges omitted (caps 1 nodes / 0 edges); run with the `text` option for the complete table."
            )
        );
    }
}
