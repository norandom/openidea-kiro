//! Status view model and fixed-text presentation (design component `StatusView`,
//! requirements 3.1–3.6).
//!
//! Provides the status model of one spec: phase, approvals, task completion,
//! successor chain for superseded specs, document figures, and finding counts.

use crate::finding::Severity;
use crate::resolve::SuccessorChain;
use crate::source::Feature;
use crate::spec::{Approvals, Phase};

use super::{phase_label, truncate_label};

/// Row for one major task in the status tasks table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MajorRow {
    pub id: String,
    pub checked: usize,
    pub total: usize,
    pub description: String,
}

/// Task totals and major rows for the status view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTotals {
    pub total: usize,
    pub checked: usize,
    pub parallel: usize,
    pub optional: usize,
    pub majors: Vec<MajorRow>,
}

/// Design component, file, and unassigned file counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesignFigures {
    pub components: usize,
    pub files: usize,
    pub unassigned: usize,
}

/// Classification of how a successor chain ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainEnd {
    Terminal,
    Cycle,
    NoSuccessor,
}

/// Derived successor chain line for superseded specs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainLine {
    pub path: Vec<String>,
    pub end: ChainEnd,
}

/// Complete status view model for one feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub spec: String,
    pub phase: String,
    pub ready: bool,
    pub approvals: Approvals,
    pub chain: Option<ChainLine>,
    pub tasks: Option<TaskTotals>,
    pub requirements: Option<(usize, usize)>,
    pub design: Option<DesignFigures>,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

/// Builds a `StatusView` from a loaded `Feature` and an optional successor chain.
///
/// `chain` is consulted only when the feature is superseded; callers may pass `None` otherwise.
pub fn status_view(feature: &Feature, chain: Option<&SuccessorChain>) -> StatusView {
    let phase = phase_label(feature.meta.as_ref());
    let ready = feature
        .meta
        .as_ref()
        .is_some_and(|m| m.ready_for_implementation);
    let approvals = feature
        .meta
        .as_ref()
        .map_or_else(Approvals::default, |m| m.approvals);

    let chain_line = if feature.phase() == Phase::Superseded {
        if let Some(ch) = chain {
            let mut path = ch.path.clone();
            if path.is_empty() {
                path.push(feature.name.clone());
            }
            if let Some(t) = &ch.terminal
                && path.last() != Some(t)
            {
                path.push(t.clone());
            }
            let end = if ch.cyclic {
                ChainEnd::Cycle
            } else if ch.terminal.is_some() {
                ChainEnd::Terminal
            } else {
                ChainEnd::NoSuccessor
            };
            Some(ChainLine { path, end })
        } else {
            Some(ChainLine {
                path: vec![feature.name.clone()],
                end: ChainEnd::NoSuccessor,
            })
        }
    } else {
        None
    };

    let tasks = feature.tasks.as_ref().map(|doc| {
        let parallel = doc.all().filter(|t| t.parallel).count();
        let optional = doc.all().filter(|t| t.optional).count();
        let majors = doc
            .tasks
            .iter()
            .map(|m| MajorRow {
                id: m.id.clone(),
                checked: m.counts.checked,
                total: m.counts.total,
                description: truncate_label(&m.description),
            })
            .collect();
        TaskTotals {
            total: doc.counts.total,
            checked: doc.counts.checked,
            parallel,
            optional,
            majors,
        }
    });

    let requirements = feature.requirements.as_ref().map(|doc| {
        let req_count = doc.requirements.len();
        let crit_count: usize = doc.requirements.iter().map(|r| r.criteria.len()).sum();
        (req_count, crit_count)
    });

    let design = feature.design.as_ref().map(|doc| DesignFigures {
        components: doc.components.len(),
        files: doc.files.len(),
        unassigned: doc.unassigned_files().count(),
    });

    let mut errors = 0;
    let mut warnings = 0;
    let mut infos = 0;
    for f in &feature.findings {
        match f.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => infos += 1,
        }
    }

    StatusView {
        spec: feature.name.clone(),
        phase,
        ready,
        approvals,
        chain: chain_line,
        tasks,
        requirements,
        design,
        errors,
        warnings,
        infos,
    }
}

/// Renders a `StatusView` into the exact fixed status text format.
pub fn status_text(view: &StatusView) -> String {
    let mut header = format!("Spec: {}\nPhase: {}\n", view.spec, view.phase);
    if let Some(chain) = &view.chain {
        let end_str = match chain.end {
            ChainEnd::Terminal => "terminal",
            ChainEnd::Cycle => "cycle",
            ChainEnd::NoSuccessor => "no known successor",
        };
        header.push_str(&format!(
            "Successor chain: {} ({end_str})\n",
            chain.path.join(" -> ")
        ));
    }
    header.push_str(&format!(
        "Ready for implementation: {}",
        if view.ready { "yes" } else { "no" }
    ));

    let approvals_table = crate::render::text::table(
        &["Document", "Generated", "Approved"],
        &[
            vec![
                "requirements".to_string(),
                yes_no(view.approvals.requirements.generated),
                yes_no(view.approvals.requirements.approved),
            ],
            vec![
                "design".to_string(),
                yes_no(view.approvals.design.generated),
                yes_no(view.approvals.design.approved),
            ],
            vec![
                "tasks".to_string(),
                yes_no(view.approvals.tasks.generated),
                yes_no(view.approvals.tasks.approved),
            ],
        ],
    );

    let tasks_block = match &view.tasks {
        Some(t) => {
            let pct = t
                .checked
                .saturating_mul(100)
                .checked_div(t.total)
                .unwrap_or(0);
            let line = format!(
                "Tasks: {}/{} done ({}%); parallel {}; optional {}",
                t.checked, t.total, pct, t.parallel, t.optional
            );
            if t.majors.is_empty() {
                line
            } else {
                let major_rows: Vec<Vec<String>> = t
                    .majors
                    .iter()
                    .map(|m| {
                        vec![
                            m.id.clone(),
                            format!("{}/{}", m.checked, m.total),
                            m.description.clone(),
                        ]
                    })
                    .collect();
                let table =
                    crate::render::text::table(&["Major", "Done", "Description"], &major_rows);
                format!("{line}\n{}", table.trim_end_matches('\n'))
            }
        }
        None => "Tasks: missing".to_string(),
    };

    let mut figures = String::new();
    match view.requirements {
        Some((r, c)) => figures.push_str(&format!("Requirements: {r} ({c} criteria)\n")),
        None => figures.push_str("Requirements: missing\n"),
    }
    match view.design {
        Some(d) => figures.push_str(&format!(
            "Design: {} components, {} files ({} unassigned)\n",
            d.components, d.files, d.unassigned
        )),
        None => figures.push_str("Design: missing\n"),
    }
    figures.push_str(&format!(
        "Findings: {} errors, {} warnings, {} info\n",
        view.errors, view.warnings, view.infos
    ));

    format!(
        "{header}\n\n{}\n\n{tasks_block}\n\n{figures}",
        approvals_table.trim_end_matches('\n')
    )
}

fn yes_no(val: bool) -> String {
    if val {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design;
    use crate::finding::{Finding, codes};
    use crate::requirements;
    use crate::resolve::SuccessorChain;
    use crate::spec::{Approval, Approvals, Phase, SpecMeta};
    use crate::tasks;

    #[test]
    fn defaults_with_unreadable_spec_json() {
        let feature = Feature {
            name: "unreadable".to_string(),
            dir: ".kiro/specs/unreadable".to_string(),
            meta: None,
            requirements: None,
            design: None,
            tasks: None,
            findings: Vec::new(),
        };

        let view = status_view(&feature, None);
        assert_eq!(view.spec, "unreadable");
        assert_eq!(view.phase, "missing");
        assert!(!view.ready);
        assert_eq!(view.approvals, Approvals::default());
        assert_eq!(view.chain, None);
        assert_eq!(view.tasks, None);
        assert_eq!(view.requirements, None);
        assert_eq!(view.design, None);
        assert_eq!(view.errors, 0);
        assert_eq!(view.warnings, 0);
        assert_eq!(view.infos, 0);
    }

    #[test]
    fn zero_over_zero_percentage() {
        let view = StatusView {
            spec: "empty-tasks".to_string(),
            phase: "initialized".to_string(),
            ready: false,
            approvals: Approvals::default(),
            chain: None,
            tasks: Some(TaskTotals {
                total: 0,
                checked: 0,
                parallel: 0,
                optional: 0,
                majors: Vec::new(),
            }),
            requirements: None,
            design: None,
            errors: 0,
            warnings: 0,
            infos: 0,
        };

        let text = status_text(&view);
        assert!(text.contains("Tasks: 0/0 done (0%); parallel 0; optional 0"));
    }

    #[test]
    fn chain_terminal_with_and_without_terminal_in_path() {
        let mut feature = Feature {
            name: "old".to_string(),
            dir: ".kiro/specs/old".to_string(),
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

        // Case 1: terminal NOT already in path
        let chain1 = SuccessorChain {
            path: vec!["old".to_string()],
            terminal: Some("demo".to_string()),
            cyclic: false,
            findings: Vec::new(),
        };
        let view1 = status_view(&feature, Some(&chain1));
        assert_eq!(
            view1.chain,
            Some(ChainLine {
                path: vec!["old".to_string(), "demo".to_string()],
                end: ChainEnd::Terminal,
            })
        );
        let text1 = status_text(&view1);
        assert!(text1.contains("Successor chain: old -> demo (terminal)"));

        // Case 2: terminal ALREADY last in path
        let chain2 = SuccessorChain {
            path: vec!["old".to_string(), "demo".to_string()],
            terminal: Some("demo".to_string()),
            cyclic: false,
            findings: Vec::new(),
        };
        let view2 = status_view(&feature, Some(&chain2));
        assert_eq!(
            view2.chain,
            Some(ChainLine {
                path: vec!["old".to_string(), "demo".to_string()],
                end: ChainEnd::Terminal,
            })
        );

        // Case 3: empty path in chain starts with feature spec
        let chain3 = SuccessorChain {
            path: Vec::new(),
            terminal: Some("demo".to_string()),
            cyclic: false,
            findings: Vec::new(),
        };
        let view3 = status_view(&feature, Some(&chain3));
        assert_eq!(
            view3.chain,
            Some(ChainLine {
                path: vec!["old".to_string(), "demo".to_string()],
                end: ChainEnd::Terminal,
            })
        );

        // Case 4: non-superseded feature ignores chain
        feature.meta = Some(SpecMeta {
            phase: Phase::TasksGenerated,
            phase_raw: Some("tasks-generated".to_string()),
            ..Default::default()
        });
        let view4 = status_view(&feature, Some(&chain1));
        assert_eq!(view4.chain, None);
    }

    #[test]
    fn chain_cycle_and_no_successor() {
        let feature = Feature {
            name: "looping".to_string(),
            dir: ".kiro/specs/looping".to_string(),
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

        // Cycle by direct construction
        let cycle_chain = SuccessorChain {
            path: vec![
                "looping".to_string(),
                "other".to_string(),
                "looping".to_string(),
            ],
            terminal: None,
            cyclic: true,
            findings: Vec::new(),
        };
        let cycle_view = status_view(&feature, Some(&cycle_chain));
        assert_eq!(
            cycle_view.chain,
            Some(ChainLine {
                path: vec![
                    "looping".to_string(),
                    "other".to_string(),
                    "looping".to_string()
                ],
                end: ChainEnd::Cycle,
            })
        );
        let cycle_text = status_text(&cycle_view);
        assert!(cycle_text.contains("Successor chain: looping -> other -> looping (cycle)"));

        // No successor by direct construction (None chain)
        let no_succ_view = status_view(&feature, None);
        assert_eq!(
            no_succ_view.chain,
            Some(ChainLine {
                path: vec!["looping".to_string()],
                end: ChainEnd::NoSuccessor,
            })
        );
        let no_succ_text = status_text(&no_succ_view);
        assert!(no_succ_text.contains("Successor chain: looping (no known successor)"));

        // No successor with explicit chain having no terminal and not cyclic
        let dangling_chain = SuccessorChain {
            path: vec!["looping".to_string(), "dangling".to_string()],
            terminal: None,
            cyclic: false,
            findings: Vec::new(),
        };
        let dangling_view = status_view(&feature, Some(&dangling_chain));
        assert_eq!(
            dangling_view.chain,
            Some(ChainLine {
                path: vec!["looping".to_string(), "dangling".to_string()],
                end: ChainEnd::NoSuccessor,
            })
        );
        assert!(
            status_text(&dangling_view)
                .contains("Successor chain: looping -> dangling (no known successor)")
        );
    }

    #[test]
    fn missing_figures_text() {
        let view = StatusView {
            spec: "missing-docs".to_string(),
            phase: "initialized".to_string(),
            ready: false,
            approvals: Approvals::default(),
            chain: None,
            tasks: None,
            requirements: None,
            design: None,
            errors: 1,
            warnings: 2,
            infos: 3,
        };

        let text = status_text(&view);
        assert!(text.contains("Tasks: missing"));
        assert!(text.contains("Requirements: missing"));
        assert!(text.contains("Design: missing"));
        assert!(text.contains("Findings: 1 errors, 2 warnings, 3 info"));
    }

    #[test]
    fn full_text_of_hand_built_feature_line_by_line() {
        let req_doc = requirements::parse(
            "### Requirement 1: First\n\n#### Acceptance Criteria\n1. Crit 1\n2. Crit 2\n",
            "requirements.md",
        );
        let design_doc = design::parse(
            "## File Structure Plan\n\n| Component | File |\n| --- | --- |\n| Core | `src/core.rs` |\n| Core | `src/extra.rs` |\n",
            "design.md",
        );
        let tasks_doc = tasks::parse(
            "- [ ] 1. Major task one\n  - [x] 1.1 (P) Sub one\n  - [x] 1.2 Sub two\n- [ ]* 2. (P) Major task two\n",
            "tasks.md",
        );

        let feature = Feature {
            name: "sample".to_string(),
            dir: ".kiro/specs/sample".to_string(),
            meta: Some(SpecMeta {
                feature_name: Some("sample".to_string()),
                phase: Phase::TasksGenerated,
                phase_raw: Some("tasks-generated".to_string()),
                ready_for_implementation: true,
                approvals: Approvals {
                    requirements: Approval {
                        generated: true,
                        approved: true,
                    },
                    design: Approval {
                        generated: true,
                        approved: false,
                    },
                    tasks: Approval {
                        generated: false,
                        approved: false,
                    },
                },
                ..Default::default()
            }),
            requirements: Some(req_doc),
            design: Some(design_doc),
            tasks: Some(tasks_doc),
            findings: vec![Finding::new(
                codes::TASK_HIERARCHY,
                "tasks.md",
                "sub-task warning",
            )],
        };

        let view = status_view(&feature, None);
        assert_eq!(view.spec, "sample");
        assert_eq!(view.phase, "tasks-generated");
        assert!(view.ready);
        assert_eq!(view.warnings, 1);
        assert_eq!(view.errors, 0);
        assert_eq!(view.infos, 0);

        let text = status_text(&view);
        let lines: Vec<&str> = text.lines().collect();

        // Header block
        assert_eq!(lines.first().copied(), Some("Spec: sample"));
        assert_eq!(lines.get(1).copied(), Some("Phase: tasks-generated"));
        assert_eq!(lines.get(2).copied(), Some("Ready for implementation: yes"));
        assert_eq!(lines.get(3).copied(), Some(""));

        // Approvals table
        assert_eq!(
            lines.get(4).copied(),
            Some("| Document | Generated | Approved |")
        );
        assert_eq!(lines.get(5).copied(), Some("| --- | --- | --- |"));
        assert_eq!(lines.get(6).copied(), Some("| requirements | yes | yes |"));
        assert_eq!(lines.get(7).copied(), Some("| design | yes | no |"));
        assert_eq!(lines.get(8).copied(), Some("| tasks | no | no |"));
        assert_eq!(lines.get(9).copied(), Some(""));

        // Tasks block (2/4 done = 50%)
        assert_eq!(
            lines.get(10).copied(),
            Some("Tasks: 2/4 done (50%); parallel 2; optional 1")
        );
        assert_eq!(
            lines.get(11).copied(),
            Some("| Major | Done | Description |")
        );
        assert_eq!(lines.get(12).copied(), Some("| --- | --- | --- |"));
        assert_eq!(lines.get(13).copied(), Some("| 1 | 2/2 | Major task one |"));
        assert_eq!(lines.get(14).copied(), Some("| 2 | 0/1 | Major task two |"));
        assert_eq!(lines.get(15).copied(), Some(""));

        // Figures block
        assert_eq!(lines.get(16).copied(), Some("Requirements: 1 (2 criteria)"));
        assert_eq!(
            lines.get(17).copied(),
            Some("Design: 1 components, 2 files (0 unassigned)")
        );
        assert_eq!(
            lines.get(18).copied(),
            Some("Findings: 0 errors, 1 warnings, 0 info")
        );
        assert_eq!(lines.len(), 19);
    }
}
