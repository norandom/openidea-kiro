//! Context command execution, ownership gathering and reference generation (tasks 2.2, 2.3).

use crate::args::Request;
use crate::context::CommandContext;
use crate::host::Level;
use crate::nav::comment::{CommentStyle, style_for, wrap};
use crate::nav::{file_name, reject_extra, task_ref_line};
use crate::present::Output;
use kiro_core::join::Owners;
use kiro_core::resolve::{ProjectPath, feature_of_path};
use kiro_core::spec::Doc;

/// Ownership details for a spec that covers the active file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecOwners {
    pub spec: String,
    pub tasks_path: String,
    pub requirements_path: String,
    pub owners: Owners,
    pub criteria_lines: Vec<(String, Option<usize>)>,
}

/// The result of checking a spec against the active file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecCheck {
    Covered(SpecOwners),
    NotCovered(String),
    NoDesign(String),
}

/// Aggregated ownership report across checked specs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextReport {
    pub file: String,
    pub checked: Vec<String>,
    pub results: Vec<SpecCheck>,
}

/// Executes `nav.context` (and `nav.context.insert`).
pub fn run(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let raw_file = match ctx.host.active_file() {
        Some(f) if !f.trim().is_empty() => f,
        _ => {
            return Err(
                "no active file: open a file in the editor and run the command again".to_string(),
            );
        }
    };

    let file = match kiro_core::resolve::to_project_relative(&raw_file, Some(&ctx.root)) {
        ProjectPath::Outside(p) => {
            return Err(format!("active file `{p}` is outside the project"));
        }
        ProjectPath::Relative(p) => p,
    };

    if let Some(spec) = feature_of_path(&file) {
        if req.insert {
            return Err(format!("nothing to insert: `{file}` is a spec document"));
        }
        return Ok(spec_document_output(&file, &spec));
    }

    reject_extra(&crate::nav::COMMANDS[0], req, 0)?;

    let specs = match &req.spec {
        Some(s) => vec![s.clone()],
        None => {
            if ctx.known.is_empty() {
                return Err(ctx
                    .resolve(None)
                    .err()
                    .unwrap_or_else(|| "no specs known".to_string()));
            }
            ctx.known.iter().map(|(name, _)| name.clone()).collect()
        }
    };

    let report = gather(ctx, &file, &specs);

    if req.insert {
        let style = style_for(&file);
        match reference_text(&report, style) {
            Some(text) => Ok(Output {
                title: report_title(&report),
                level: report_level(&report),
                text,
            }),
            None => Err(format!(
                "nothing to insert: `{file}` is not covered by {}",
                specs.join(", ")
            )),
        }
    } else {
        Ok(Output {
            title: report_title(&report),
            level: report_level(&report),
            text: report_text(&report),
        })
    }
}

/// Gathers ownership information by checking `specs` in order against `file`.
pub fn gather(ctx: &mut CommandContext<'_>, file: &str, specs: &[String]) -> ContextReport {
    let mut results = Vec::new();
    for spec in specs {
        let feature = ctx.load(spec);
        if feature.design.is_none() {
            results.push(SpecCheck::NoDesign(spec.clone()));
            continue;
        }

        let owners =
            kiro_core::join::owners_of(feature.design.as_ref(), feature.tasks.as_ref(), file);
        if owners.covered {
            let mut criteria_lines = Vec::new();
            for id in &owners.criteria {
                let line = feature.requirements.as_ref().and_then(|r| {
                    r.criterion(id)
                        .map(|c| c.line)
                        .or_else(|| r.requirement(id).map(|req| req.line))
                });
                criteria_lines.push((id.clone(), line));
            }

            results.push(SpecCheck::Covered(SpecOwners {
                spec: spec.clone(),
                tasks_path: feature.path(Doc::Tasks),
                requirements_path: feature.path(Doc::Requirements),
                owners,
                criteria_lines,
            }));
        } else {
            results.push(SpecCheck::NotCovered(spec.clone()));
        }
    }

    ContextReport {
        file: file.to_string(),
        checked: specs.to_vec(),
        results,
    }
}

/// Formats the full human-readable ownership report.
#[must_use]
pub fn report_text(r: &ContextReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("File: {}", r.file));

    let spec_count_label = if r.checked.len() == 1 {
        "1 spec".to_string()
    } else {
        format!("{} specs", r.checked.len())
    };
    lines.push(format!(
        "Checked: {} ({spec_count_label})",
        r.checked.join(", ")
    ));

    let mut covered = Vec::new();
    let mut not_covered = Vec::new();
    let mut no_design = Vec::new();

    for check in &r.results {
        match check {
            SpecCheck::Covered(owners) => covered.push(owners.spec.as_str()),
            SpecCheck::NotCovered(name) => not_covered.push(name.as_str()),
            SpecCheck::NoDesign(name) => no_design.push(name.as_str()),
        }
    }

    if covered.is_empty() {
        lines.push("Covered by: none".to_string());
    } else {
        lines.push(format!("Covered by: {}", covered.join(", ")));
    }

    if !not_covered.is_empty() {
        lines.push(format!("Not covered by: {}", not_covered.join(", ")));
    }

    if !no_design.is_empty() {
        lines.push(format!("No design: {}", no_design.join(", ")));
    }

    for check in &r.results {
        if let SpecCheck::Covered(owners) = check {
            lines.push(String::new());
            lines.push(owners.spec.clone());
            lines.push(format!(
                "  Components: {}",
                owners.owners.components.join(", ")
            ));

            if owners.owners.tasks.is_empty() {
                lines.push("  Tasks: none".to_string());
            } else {
                lines.push("  Tasks:".to_string());
                for t in &owners.owners.tasks {
                    lines.push(format!("    {}", task_ref_line(t, &owners.tasks_path)));
                }
            }

            if owners.criteria_lines.is_empty() {
                lines.push("  Requirements: none".to_string());
            } else {
                lines.push("  Requirements:".to_string());
                for (id, line) in &owners.criteria_lines {
                    match line {
                        Some(l) => {
                            lines.push(format!("    {id} ({}:{l})", owners.requirements_path));
                        }
                        None => {
                            lines.push(format!("    {id} (not found)"));
                        }
                    }
                }
            }
        }
    }

    lines.join("\n")
}

/// Generates the notification title for the ownership report.
#[must_use]
pub fn report_title(r: &ContextReport) -> String {
    let name = file_name(&r.file);
    let mut covered = Vec::new();
    for check in &r.results {
        if let SpecCheck::Covered(owners) = check {
            covered.push(owners.spec.as_str());
        }
    }

    if covered.is_empty() {
        format!("Kiro: {name} — not covered")
    } else {
        format!("Kiro: {name} — {}", covered.join(", "))
    }
}

/// Determines the notification severity level for the ownership report.
#[must_use]
pub fn report_level(r: &ContextReport) -> Level {
    let has_covered = r.results.iter().any(|c| matches!(c, SpecCheck::Covered(_)));
    let has_no_design = r
        .results
        .iter()
        .any(|c| matches!(c, SpecCheck::NoDesign(_)));
    if !has_covered || has_no_design {
        Level::Warning
    } else {
        Level::Info
    }
}

fn document_kind(path: &str) -> &'static str {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name {
        "spec.json" => "spec metadata",
        "requirements.md" => "requirements document",
        "design.md" => "design document",
        "tasks.md" => "tasks document",
        _ => "document",
    }
}

/// Output when the active file is itself a spec document.
#[must_use]
pub fn spec_document_output(path: &str, spec: &str) -> Output {
    let kind = document_kind(path);
    let name = file_name(path);
    Output {
        title: format!("Kiro: {name} — spec document of {spec}"),
        level: Level::Info,
        text: format!(
            "File: {path}\n{path} is the {kind} of spec `{spec}`. Run `nav.files` (Kiro: Files by Component) for its components and files."
        ),
    }
}

/// Generates a single reference line for a covering spec.
#[must_use]
pub fn reference_line(o: &SpecOwners) -> String {
    let components = if o.owners.components.is_empty() {
        "none".to_string()
    } else {
        o.owners.components.join(", ")
    };
    let tasks = if o.owners.tasks.is_empty() {
        "none".to_string()
    } else {
        o.owners
            .tasks
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let requirements = if o.owners.criteria.is_empty() {
        "none".to_string()
    } else {
        o.owners.criteria.join(", ")
    };

    format!(
        "kiro-spec: {}; components: {components}; tasks: {tasks}; requirements: {requirements}",
        o.spec
    )
}

/// Generates the full wrapped reference text for editor insertion.
#[must_use]
pub fn reference_text(r: &ContextReport, style: CommentStyle) -> Option<String> {
    let mut out = String::new();
    let mut any_covered = false;

    for check in &r.results {
        if let SpecCheck::Covered(owners) = check {
            any_covered = true;
            let line = reference_line(owners);
            let wrapped = wrap(style, &line);
            out.push_str(&wrapped);
            out.push('\n');
        }
    }

    if any_covered { Some(out) } else { None }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]
mod tests {
    use super::*;
    use crate::host::FakeHost;
    use kiro_core::join::TaskRef;

    fn sample_spec_owners() -> SpecOwners {
        SpecOwners {
            spec: "alpha".to_string(),
            tasks_path: ".kiro/specs/alpha/tasks.md".to_string(),
            requirements_path: ".kiro/specs/alpha/requirements.md".to_string(),
            owners: Owners {
                covered: true,
                components: vec!["A".to_string(), "B".to_string()],
                tasks: vec![TaskRef {
                    id: "2.1".to_string(),
                    description: "Build alpha".to_string(),
                    checked: true,
                    line: 11,
                }],
                criteria: vec!["1.1".to_string(), "9.9".to_string()],
            },
            criteria_lines: vec![("1.1".to_string(), Some(9)), ("9.9".to_string(), None)],
        }
    }

    #[test]
    fn report_text_formatting() {
        let report = ContextReport {
            file: "src/shared.rs".to_string(),
            checked: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
            results: vec![
                SpecCheck::Covered(sample_spec_owners()),
                SpecCheck::NotCovered("beta".to_string()),
                SpecCheck::NoDesign("gamma".to_string()),
            ],
        };

        let text = report_text(&report);
        let expected = "\
File: src/shared.rs
Checked: alpha, beta, gamma (3 specs)
Covered by: alpha
Not covered by: beta
No design: gamma

alpha
  Components: A, B
  Tasks:
    2.1 [x] Build alpha (.kiro/specs/alpha/tasks.md:11)
  Requirements:
    1.1 (.kiro/specs/alpha/requirements.md:9)
    9.9 (not found)";
        assert_eq!(text, expected);
    }

    #[test]
    fn report_text_empty_tasks_and_requirements() {
        let report = ContextReport {
            file: "src/alpha.rs".to_string(),
            checked: vec!["alpha".to_string()],
            results: vec![SpecCheck::Covered(SpecOwners {
                spec: "alpha".to_string(),
                tasks_path: ".kiro/specs/alpha/tasks.md".to_string(),
                requirements_path: ".kiro/specs/alpha/requirements.md".to_string(),
                owners: Owners {
                    covered: true,
                    components: vec!["A".to_string()],
                    tasks: vec![],
                    criteria: vec![],
                },
                criteria_lines: vec![],
            })],
        };

        let text = report_text(&report);
        assert!(text.contains("  Tasks: none"));
        assert!(text.contains("  Requirements: none"));
    }

    #[test]
    fn report_text_not_covered() {
        let report = ContextReport {
            file: "src/unknown.rs".to_string(),
            checked: vec!["alpha".to_string()],
            results: vec![SpecCheck::NotCovered("alpha".to_string())],
        };

        let text = report_text(&report);
        assert!(text.contains("Covered by: none"));
        assert!(text.contains("Not covered by: alpha"));
        assert!(!text.contains("No design:"));
    }

    #[test]
    fn report_title_and_level() {
        let covered = ContextReport {
            file: "src/shared.rs".to_string(),
            checked: vec!["alpha".to_string(), "beta".to_string()],
            results: vec![
                SpecCheck::Covered(sample_spec_owners()),
                SpecCheck::NotCovered("beta".to_string()),
            ],
        };
        assert_eq!(report_title(&covered), "Kiro: shared.rs — alpha");
        assert_eq!(report_level(&covered), Level::Info);

        let uncovered = ContextReport {
            file: "src/shared.rs".to_string(),
            checked: vec!["alpha".to_string()],
            results: vec![SpecCheck::NotCovered("alpha".to_string())],
        };
        assert_eq!(report_title(&uncovered), "Kiro: shared.rs — not covered");
        assert_eq!(report_level(&uncovered), Level::Warning);

        let with_no_design = ContextReport {
            file: "src/shared.rs".to_string(),
            checked: vec!["alpha".to_string(), "gamma".to_string()],
            results: vec![
                SpecCheck::Covered(sample_spec_owners()),
                SpecCheck::NoDesign("gamma".to_string()),
            ],
        };
        assert_eq!(report_level(&with_no_design), Level::Warning);
    }

    #[test]
    fn reference_line_and_text() {
        let owners = sample_spec_owners();
        assert_eq!(
            reference_line(&owners),
            "kiro-spec: alpha; components: A, B; tasks: 2.1; requirements: 1.1, 9.9"
        );

        let empty_owners = SpecOwners {
            spec: "beta".to_string(),
            tasks_path: ".kiro/specs/beta/tasks.md".to_string(),
            requirements_path: ".kiro/specs/beta/requirements.md".to_string(),
            owners: Owners {
                covered: true,
                components: vec![],
                tasks: vec![],
                criteria: vec![],
            },
            criteria_lines: vec![],
        };
        assert_eq!(
            reference_line(&empty_owners),
            "kiro-spec: beta; components: none; tasks: none; requirements: none"
        );

        let report = ContextReport {
            file: "src/shared.rs".to_string(),
            checked: vec!["alpha".to_string()],
            results: vec![SpecCheck::Covered(owners)],
        };
        let wrapped = reference_text(&report, CommentStyle::DoubleSlash).unwrap();
        assert_eq!(
            wrapped,
            "// kiro-spec: alpha; components: A, B; tasks: 2.1; requirements: 1.1, 9.9\n"
        );

        let uncovered_report = ContextReport {
            file: "src/shared.rs".to_string(),
            checked: vec!["alpha".to_string()],
            results: vec![SpecCheck::NotCovered("alpha".to_string())],
        };
        assert!(reference_text(&uncovered_report, CommentStyle::DoubleSlash).is_none());
    }

    #[test]
    fn spec_document_output_variants() {
        let out = spec_document_output(".kiro/specs/alpha/design.md", "alpha");
        assert_eq!(out.title, "Kiro: design.md — spec document of alpha");
        assert_eq!(out.level, Level::Info);
        assert!(out.text.contains("design document of spec `alpha`"));
    }

    #[test]
    fn run_active_file_error_checks() {
        let host = FakeHost::new().with_root("C:\\proj");
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let req = Request {
            spec: None,
            raw: None,
            rest: Vec::new(),
            insert: false,
        };

        let err = run(&mut ctx, &req).unwrap_err();
        assert!(err.contains("no active file"));

        let host_outside = FakeHost::new()
            .with_root("C:\\proj")
            .with_active("D:\\other\\file.rs");
        let mut ctx_outside = CommandContext::new(&host_outside, None).unwrap();
        let err_outside = run(&mut ctx_outside, &req).unwrap_err();
        assert!(err_outside.contains("outside the project"));

        let host_spec = FakeHost::new()
            .with_root("C:\\proj")
            .with_active("C:\\proj\\.kiro\\specs\\alpha\\design.md");
        let mut ctx_spec = CommandContext::new(&host_spec, None).unwrap();
        let mut req_insert = req;
        req_insert.insert = true;
        let err_insert = run(&mut ctx_spec, &req_insert).unwrap_err();
        assert!(err_insert.contains("nothing to insert"));
        assert!(err_insert.contains("is a spec document"));
    }
}
