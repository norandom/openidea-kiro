//! The file, component and task join: path matching, the owners of a
//! project-relative file, the reverse coverage listing including unscoped
//! and unknown-boundary tasks, and the cross-document link findings.
//!
//! Design component `Join`, requirements 9.1-9.10.

use crate::design::{DesignDoc, normalize_design_path};
use crate::finding::{Finding, codes};
use crate::requirements::RequirementsDoc;
use crate::tasks::{Task, TasksDoc, valid_requirement_ref};

/// One task referenced from the join, carrying what navigation needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRef {
    pub id: String,
    pub description: String,
    pub line: usize,
    pub checked: bool,
}

/// The owners of a queried path: matching components in design order, the
/// tasks whose boundaries name them, and the criteria those tasks reference.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Owners {
    pub covered: bool,
    pub components: Vec<String>,
    pub tasks: Vec<TaskRef>,
    pub criteria: Vec<String>,
}

/// One component with its files and the tasks bound to it by boundary name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentCoverage {
    pub name: String,
    pub files: Vec<String>,
    pub tasks: Vec<TaskRef>,
}

/// The reverse direction of the join: everything a feature covers, plus the
/// tasks and files that fall outside the design's component mapping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Coverage {
    pub components: Vec<ComponentCoverage>,
    pub unscoped_tasks: Vec<TaskRef>,
    pub unassigned_files: Vec<String>,
    pub unknown_boundary_tasks: Vec<(TaskRef, Vec<String>)>,
}

/// Equal after normalisation, or one is a suffix of the other starting at a
/// `/` boundary (`b/c.rs` does not match `ab/c.rs`).
pub fn path_matches(design_path: &str, query: &str) -> bool {
    let a = normalize_design_path(design_path);
    let b = normalize_design_path(query);
    if a == b {
        return true;
    }
    let a_segments = segments(&a);
    let b_segments = segments(&b);
    is_suffix(&a_segments, &b_segments) || is_suffix(&b_segments, &a_segments)
}

/// Non-empty segments of a normalised path (empty segments are collapsed
/// away by the normaliser or dropped here for trailing slashes).
fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Whether `long` ends with the whole of `short` (equal counts as a match;
/// the strict-subset case is covered by the caller's equality fast path).
fn is_suffix(short: &[&str], long: &[&str]) -> bool {
    !short.is_empty() && long.ends_with(short)
}

/// Trimmed, ASCII case-insensitive name comparison.
pub fn names_equal(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Components whose design files match `query`, the tasks bound to them and
/// the criteria those tasks reference; `covered` is false with empty lists
/// when nothing matches (no finding is recorded).
pub fn owners_of(design: Option<&DesignDoc>, tasks: Option<&TasksDoc>, query: &str) -> Owners {
    let mut components: Vec<String> = Vec::new();
    if let Some(design) = design {
        for component in &design.components {
            let matched = design
                .files_of(&component.name)
                .any(|file| path_matches(&file.path, query));
            if matched && !components.contains(&component.name) {
                components.push(component.name.clone());
            }
        }
    }
    let mut bound_tasks = Vec::new();
    let mut criteria: Vec<String> = Vec::new();
    if let Some(tasks) = tasks {
        for task in tasks.all() {
            let bound = task
                .boundary
                .iter()
                .any(|name| components.iter().any(|c| names_equal(name, c)));
            if bound {
                criteria.extend(task.requirements.iter().cloned());
                bound_tasks.push(task_ref(task));
            }
        }
    }
    criteria.sort();
    criteria.dedup();
    Owners {
        covered: !components.is_empty(),
        components,
        tasks: bound_tasks,
        criteria,
    }
}

/// Every component with its files and bound tasks, the tasks that declare
/// no boundary, the plan files without a component, and the tasks whose
/// boundary contains names no component matches (all of them when the
/// design document is absent), each paired with the unmatched names.
pub fn coverage(design: Option<&DesignDoc>, tasks: Option<&TasksDoc>) -> Coverage {
    let mut component_coverage = Vec::new();
    if let Some(design) = design {
        for component in &design.components {
            let files: Vec<String> = design
                .files_of(&component.name)
                .map(|file| file.path.clone())
                .collect();
            let bound = bound_tasks(tasks, &component.name);
            component_coverage.push(ComponentCoverage {
                name: component.name.clone(),
                files,
                tasks: bound,
            });
        }
    }
    let mut unscoped_tasks = Vec::new();
    let mut unknown_boundary_tasks = Vec::new();
    if let Some(tasks) = tasks {
        for task in tasks.all() {
            if task.boundary.is_empty() {
                unscoped_tasks.push(task_ref(task));
            } else {
                let unmatched: Vec<String> = match design {
                    Some(design) => task
                        .boundary
                        .iter()
                        .filter(|name| design.component(name).is_none())
                        .cloned()
                        .collect(),
                    None => task.boundary.clone(),
                };
                if !unmatched.is_empty() {
                    unknown_boundary_tasks.push((task_ref(task), unmatched));
                }
            }
        }
    }
    let unassigned_files = design.map_or(Vec::new(), |design| {
        design
            .unassigned_files()
            .map(|file| file.path.clone())
            .collect()
    });
    Coverage {
        components: component_coverage,
        unscoped_tasks,
        unassigned_files,
        unknown_boundary_tasks,
    }
}

/// Cross-document link findings for one feature's `tasks.md`:
/// `TASK_BOUNDARY_UNKNOWN` at the task line (needs the design document),
/// `TASK_REQ_UNKNOWN` at the requirements annotation line for valid-shaped
/// references that no criterion or requirement defines (needs the
/// requirements document), and `TASK_DEPENDS_UNKNOWN` at the depends
/// annotation line. Absent documents are tolerated and simply skip their
/// checks.
pub fn link(
    feature: &str,
    requirements: Option<&RequirementsDoc>,
    design: Option<&DesignDoc>,
    tasks: &TasksDoc,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for task in tasks.all() {
        if let Some(design) = design {
            for name in &task.boundary {
                if design.component(name).is_none() {
                    findings.push(
                        Finding::new(
                            codes::TASK_BOUNDARY_UNKNOWN,
                            &tasks.file,
                            format!("boundary '{name}' names no design component"),
                        )
                        .at(task.line)
                        .feature(feature),
                    );
                }
            }
        }
        if let (Some(requirements), Some(line)) = (requirements, task.requirements_line) {
            for reference in &task.requirements {
                if valid_requirement_ref(reference) && !known_reference(requirements, reference) {
                    findings.push(
                        Finding::new(
                            codes::TASK_REQ_UNKNOWN,
                            &tasks.file,
                            format!("requirement reference '{reference}' is not defined"),
                        )
                        .at(line)
                        .feature(feature),
                    );
                }
            }
        }
        if let Some(line) = task.depends_line {
            for dependency in &task.depends {
                if tasks.task(dependency).is_none() {
                    findings.push(
                        Finding::new(
                            codes::TASK_DEPENDS_UNKNOWN,
                            &tasks.file,
                            format!("dependency '{dependency}' names no task"),
                        )
                        .at(line)
                        .feature(feature),
                    );
                }
            }
        }
    }
    findings
}

/// Whether a requirement reference resolves to a criterion or a whole
/// requirement (`N` is known when requirement `N` exists).
fn known_reference(requirements: &RequirementsDoc, reference: &str) -> bool {
    requirements.criterion(reference).is_some() || requirements.requirement(reference).is_some()
}

/// The tasks whose boundary names `component` (trimmed, case-insensitive),
/// in document order.
fn bound_tasks(tasks: Option<&TasksDoc>, component: &str) -> Vec<TaskRef> {
    let mut bound = Vec::new();
    if let Some(tasks) = tasks {
        for task in tasks.all() {
            if task
                .boundary
                .iter()
                .any(|name| names_equal(name, component))
            {
                bound.push(task_ref(task));
            }
        }
    }
    bound
}

/// The navigation-friendly projection of one task.
fn task_ref(task: &Task) -> TaskRef {
    TaskRef {
        id: task.id.clone(),
        description: task.description.clone(),
        line: task.line,
        checked: task.checked,
    }
}

#[cfg(test)]
mod tests {
    use crate::design::parse as parse_design;
    use crate::finding::codes;
    use crate::join::{coverage, link, names_equal, owners_of, path_matches};
    use crate::requirements::parse as parse_requirements;
    use crate::tasks::parse as parse_tasks;

    const REQUIREMENTS: &str = "### Requirement 1: Owner docs\n\n#### Acceptance Criteria\n1. First criterion.\n2. Second criterion.\n";

    const DESIGN: &str = "# D\n\n## File Structure Plan\n\n| Component | File |\n|---|---|\n| Alpha | `mod.rs` |\n| Beta | `src/render/mod.rs` |\n|  | `src/free.rs` |\n";

    const TASKS: &str = concat!(
        "# Tasks\n",
        "\n",
        "- [x] 1.1 Scanner task\n",
        "  - _Requirements: 1.1, 1_\n",
        "  - _Boundary: Alpha_\n",
        "- [ ] 1.2 Render task\n",
        "  - _Depends: 1.1_\n",
        "  - _Boundary: BETA_\n",
        "- [ ] 1.3 Loose task\n",
    );

    #[test]
    fn path_matching_requires_a_segment_boundary() {
        assert!(path_matches("src/b/c.rs", "b/c.rs"));
        assert!(path_matches("b/c.rs", "src/b/c.rs"));
        assert!(path_matches("a/b/c.rs", "a/b/c.rs"));
        assert!(
            !path_matches("ab/c.rs", "b/c.rs"),
            "no match inside a segment"
        );
        assert!(!path_matches("src/other.rs", "other/different.rs"));
        assert!(
            path_matches("src\\demo.rs", "src/demo.rs"),
            "separators normalise"
        );
    }

    #[test]
    fn names_equal_trims_and_ignores_ascii_case() {
        assert!(names_equal(" Alpha ", "alpha"));
        assert!(names_equal("RENDER", "render"));
        assert!(!names_equal("a-b", "ab"));
    }

    #[test]
    fn owners_multi_match_keeps_design_order_and_collects_criteria() {
        let design = parse_design(DESIGN, "design.md");
        let tasks = parse_tasks(TASKS, "tasks.md");
        let owners = owners_of(Some(&design), Some(&tasks), "mod.rs");
        assert!(owners.covered);
        assert_eq!(owners.components, ["Alpha", "Beta"]);
        let ids: Vec<&str> = owners.tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["1.1", "1.2"], "tasks in document order");
        assert_eq!(
            owners.criteria,
            ["1", "1.1"],
            "criteria sorted and deduplicated"
        );
        assert!(owners.tasks.first().is_some_and(|t| t.checked));
    }

    #[test]
    fn owners_not_covered_is_empty_without_a_finding() {
        let design = parse_design(DESIGN, "design.md");
        let owners = owners_of(Some(&design), None, "nowhere/x.rs");
        assert!(!owners.covered);
        assert!(owners.components.is_empty());
        assert!(owners.tasks.is_empty());
        assert!(owners.criteria.is_empty());
    }

    #[test]
    fn coverage_lists_components_unscoped_and_unassigned() {
        let design = parse_design(DESIGN, "design.md");
        let tasks = parse_tasks(TASKS, "tasks.md");
        let cov = coverage(Some(&design), Some(&tasks));
        let alpha = cov.components.iter().find(|c| c.name == "Alpha");
        assert!(
            alpha.is_some_and(
                |c| c.files == ["mod.rs"] && c.tasks.iter().map(|t| t.id.as_str()).eq(["1.1"])
            ),
            "alpha first in design order; 1.2 binds via case-insensitive boundary"
        );
        let beta = cov.components.iter().find(|c| c.name == "Beta");
        assert!(
            beta.is_some_and(|c| c.files == ["src/render/mod.rs"]
                && c.tasks.iter().map(|t| t.id.as_str()).eq(["1.2"])),
            "beta bound via the case-insensitive BETA boundary"
        );
        assert_eq!(
            cov.unscoped_tasks
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<&str>>(),
            ["1.3"]
        );
        assert_eq!(cov.unassigned_files, ["src/free.rs"]);
        assert!(
            cov.unknown_boundary_tasks.is_empty(),
            "all boundary names resolve"
        );
    }

    #[test]
    fn coverage_paired_unknown_boundary_names_and_design_absent_rules() {
        let design = parse_design(DESIGN, "design.md");
        let tasks = parse_tasks(
            concat!(
                "# Tasks\n",
                "\n",
                "- [ ] 1.1 Mixed boundaries\n",
                "  - _Boundary: Alpha, Ghost, Spectre_\n",
                "- [ ] 1.2 No boundary at all\n",
            ),
            "tasks.md",
        );
        let cov = coverage(Some(&design), Some(&tasks));
        assert_eq!(
            cov.unscoped_tasks
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<&str>>(),
            ["1.2"]
        );
        let expected = vec!["Ghost".to_owned(), "Spectre".to_owned()];
        assert_eq!(
            cov.unknown_boundary_tasks
                .first()
                .map(|(t, names)| (t.id.as_str(), names.clone())),
            Some(("1.1", expected)),
            "only unmatched names, as written"
        );

        let absent = coverage(None, Some(&tasks));
        assert!(absent.components.is_empty());
        assert_eq!(absent.unscoped_tasks.len(), 1);
        let expected = vec!["Alpha".to_owned(), "Ghost".to_owned(), "Spectre".to_owned()];
        assert_eq!(
            absent
                .unknown_boundary_tasks
                .first()
                .map(|(t, names)| (t.id.as_str(), names.clone())),
            Some(("1.1", expected)),
            "all boundary names when design absent"
        );
    }

    #[test]
    fn link_emits_each_finding_at_its_specified_line() {
        let requirements = parse_requirements(REQUIREMENTS, "requirements.md");
        let design = parse_design(DESIGN, "design.md");
        let tasks = parse_tasks(
            concat!(
                "# Tasks\n",
                "\n",
                "- [ ] 1.1 Ghost boundary\n",        // line 3
                "  - _Requirements: 1.9, 1_\n",      // line 4
                "  - _Depends: 9.9_\n",              // line 5
                "  - _Boundary: Ghost_\n",           // line 6
                "- [ ] 1.2 Invalid shape ignored\n", // line 7
                "  - _Requirements: nonsense_\n",    // line 8
                "  - _Depends: 1.1_\n",              // line 9
            ),
            "tasks.md",
        );
        let findings = link("demo", Some(&requirements), Some(&design), &tasks);
        let by_code = |code: &str| -> Vec<(Option<usize>, String)> {
            findings
                .iter()
                .filter(|f| f.code == code)
                .map(|f| (f.line, f.message.clone()))
                .collect()
        };
        assert_eq!(
            by_code(codes::TASK_BOUNDARY_UNKNOWN),
            [(
                Some(3),
                "boundary 'Ghost' names no design component".to_owned()
            )],
            "unknown boundary at the task line"
        );
        assert_eq!(
            by_code(codes::TASK_REQ_UNKNOWN),
            [(
                Some(4),
                "requirement reference '1.9' is not defined".to_owned()
            )],
            "unknown criterion at the requirements line; whole-requirement '1' is known"
        );
        assert_eq!(
            by_code(codes::TASK_DEPENDS_UNKNOWN),
            [(Some(5), "dependency '9.9' names no task".to_owned())],
            "unknown dependency at the depends line"
        );
        assert!(
            findings
                .iter()
                .all(|f| f.feature.as_deref() == Some("demo"))
        );
        assert!(
            !findings
                .iter()
                .any(|f| f.code == codes::TASK_REQ_REF_INVALID),
            "invalid shapes belong to the tasks parser, not the join"
        );
    }

    #[test]
    fn link_tolerates_absent_requirements_and_design() {
        let tasks = parse_tasks(TASKS, "tasks.md");
        let findings = link("demo", None, None, &tasks);
        assert!(
            findings.is_empty(),
            "no boundary/requirement findings without the documents; depends resolve"
        );
    }
}
