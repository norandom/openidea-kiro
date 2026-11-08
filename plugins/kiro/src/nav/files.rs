//! Files by component and by task coverage views (tasks 2.4, 2.5).

use crate::args::Request;
use crate::context::CommandContext;
use crate::host::Level;
use crate::nav::{reject_extra, task_line, task_ref_line};
use crate::present::Output;
use kiro_core::design::DesignDoc;
use kiro_core::join::{coverage, names_equal};
use kiro_core::source::Feature;
use kiro_core::spec::Doc;
use kiro_core::tasks::Task;

/// Executes `nav.files` (files by component).
pub fn by_component(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    reject_extra(&crate::nav::COMMANDS[1], req, 0)?;
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let feature = ctx.load(&resolved.name);
    let (text, level) = component_text(feature);
    Ok(Output {
        title: format!("Kiro: {} files by component", resolved.name),
        level,
        text,
    })
}

/// Executes `nav.task-files` (files by task).
pub fn by_task(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    reject_extra(&crate::nav::COMMANDS[2], req, 1)?;
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let filter = req.rest.first().map(String::as_str);
    let feature = ctx.load(&resolved.name);
    let (text, level) = task_text(feature, filter)?;
    let title = match filter {
        Some(id) => format!("Kiro: {} files by task — task {id}", resolved.name),
        None => format!("Kiro: {} files by task", resolved.name),
    };
    Ok(Output { title, level, text })
}

/// Label for one boundary name: "<Name>" | "<Name> (no files)" | "<name as written> (unknown)";
/// a task with an empty boundary is labelled "unscoped".
#[must_use]
pub fn boundary_label(design: Option<&DesignDoc>, name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "unscoped".to_string();
    }
    match design {
        None => format!("{name} (unknown)"),
        Some(d) => {
            if let Some(c) = d.components.iter().find(|c| names_equal(&c.name, name)) {
                if d.files_of(&c.name).next().is_none() {
                    format!("{} (no files)", c.name)
                } else {
                    c.name.clone()
                }
            } else {
                format!("{name} (unknown)")
            }
        }
    }
}

/// Formats the files-by-component view for `feature`.
#[must_use]
pub fn component_text(feature: &Feature) -> (String, Level) {
    let mut lines = Vec::new();
    let cov = coverage(feature.design.as_ref(), feature.tasks.as_ref());
    let design_path = feature.path(Doc::Design);
    let tasks_path = feature.path(Doc::Tasks);

    lines.push(format!("Spec: {}", feature.name));

    match feature.design {
        Some(_) => lines.push(format!("Design: {design_path}")),
        None => lines.push(format!("Design: {design_path} (missing)")),
    }

    match feature.tasks {
        Some(_) => lines.push(format!("Tasks: {tasks_path}")),
        None => lines.push(format!("Tasks: {tasks_path} (missing)")),
    }

    let comp_part = match &feature.design {
        Some(d) => format!(
            "Components: {} (files: {}, unassigned: {})",
            d.components.len(),
            d.files.len(),
            cov.unassigned_files.len()
        ),
        None => "Components: missing".to_string(),
    };

    let task_part = match &feature.tasks {
        Some(t) => format!(
            "tasks: {} (unscoped: {}, unknown boundary: {})",
            t.counts.total,
            cov.unscoped_tasks.len(),
            cov.unknown_boundary_tasks.len()
        ),
        None => "tasks: missing".to_string(),
    };

    lines.push(format!("{comp_part}; {task_part}"));

    if let Some(design) = &feature.design {
        for comp in &cov.components {
            lines.push(String::new());
            if comp.files.is_empty() {
                lines.push(format!(
                    "{} (no files, {} tasks)",
                    comp.name,
                    comp.tasks.len()
                ));
            } else {
                lines.push(format!(
                    "{} ({} files, {} tasks)",
                    comp.name,
                    comp.files.len(),
                    comp.tasks.len()
                ));
            }

            for file in &comp.files {
                let resp = design
                    .files
                    .iter()
                    .find(|df| df.path == *file)
                    .and_then(|df| df.responsibility.as_deref());
                match resp {
                    Some(r) => lines.push(format!("  {file} — {r}")),
                    None => lines.push(format!("  {file}")),
                }
            }

            for t in &comp.tasks {
                lines.push(format!("  task {}", task_ref_line(t, &tasks_path)));
            }
        }

        if !cov.unassigned_files.is_empty() {
            lines.push(String::new());
            lines.push(format!("Unassigned files ({})", cov.unassigned_files.len()));
            for file in &cov.unassigned_files {
                lines.push(format!("  {file}"));
            }
        }
    }

    if feature.tasks.is_some() {
        if !cov.unscoped_tasks.is_empty() {
            lines.push(String::new());
            lines.push(format!("Unscoped tasks ({})", cov.unscoped_tasks.len()));
            for t in &cov.unscoped_tasks {
                lines.push(format!("  task {}", task_ref_line(t, &tasks_path)));
            }
        }

        if !cov.unknown_boundary_tasks.is_empty() {
            lines.push(String::new());
            lines.push(format!(
                "Unknown boundary ({})",
                cov.unknown_boundary_tasks.len()
            ));
            for (t, names) in &cov.unknown_boundary_tasks {
                lines.push(format!(
                    "  task {} — {}",
                    task_ref_line(t, &tasks_path),
                    names.join(", ")
                ));
            }
        }
    }

    let level = if feature.design.is_none() || feature.tasks.is_none() {
        Level::Warning
    } else {
        Level::Info
    };

    (lines.join("\n"), level)
}

/// Formats the files-by-task view for `feature`, optionally filtered by `filter`.
pub fn task_text(feature: &Feature, filter: Option<&str>) -> Result<(String, Level), String> {
    let mut lines = Vec::new();
    let tasks_path = feature.path(Doc::Tasks);
    let design_path = feature.path(Doc::Design);

    lines.push(format!("Spec: {}", feature.name));

    match feature.tasks {
        Some(_) => lines.push(format!("Tasks: {tasks_path}")),
        None => lines.push(format!("Tasks: {tasks_path} (missing)")),
    }

    match feature.design {
        Some(_) => lines.push(format!("Design: {design_path}")),
        None => lines.push(format!("Design: {design_path} (missing)")),
    }

    let tasks = match &feature.tasks {
        Some(t) => t,
        None => {
            lines.push("Count: missing".to_string());
            return Ok((lines.join("\n"), Level::Warning));
        }
    };

    let cov = coverage(feature.design.as_ref(), Some(tasks));

    let count_line = match filter {
        Some(f) => format!(
            "Count: {} tasks (completed {}, unscoped {}); filter: task {f}",
            tasks.counts.total,
            tasks.counts.checked,
            cov.unscoped_tasks.len()
        ),
        None => format!(
            "Count: {} tasks (completed {}, unscoped {})",
            tasks.counts.total,
            tasks.counts.checked,
            cov.unscoped_tasks.len()
        ),
    };
    lines.push(count_line);

    let format_task_entry = |t: &Task, is_sub: bool| -> Vec<String> {
        let mut entry_lines = Vec::new();
        let indent = if is_sub { "  " } else { "" };
        let file_indent = if is_sub { "      " } else { "    " };

        let boundary_str = if t.boundary.is_empty() {
            "unscoped".to_string()
        } else {
            let labels: Vec<String> = t
                .boundary
                .iter()
                .map(|b| boundary_label(feature.design.as_ref(), b))
                .collect();
            labels.join(", ")
        };

        entry_lines.push(format!(
            "{indent}{} — {boundary_str}",
            task_line(t, &tasks_path)
        ));

        if let Some(design) = &feature.design {
            let mut files: Vec<String> = Vec::new();
            for b in &t.boundary {
                if design.component(b).is_some() {
                    for f in design.files_of(b) {
                        if !files.contains(&f.path) {
                            files.push(f.path.clone());
                        }
                    }
                }
            }
            for file in files {
                entry_lines.push(format!("{file_indent}{file}"));
            }
        }

        entry_lines
    };

    match filter {
        Some(id) => {
            let found = tasks
                .task(id)
                .ok_or_else(|| format!("task `{id}` not found in {tasks_path}"))?;

            lines.push(String::new());
            if found.sub.is_none() {
                lines.extend(format_task_entry(found, false));
                for sub in &found.subtasks {
                    lines.extend(format_task_entry(sub, true));
                }
            } else {
                lines.extend(format_task_entry(found, true));
            }
        }
        None => {
            lines.push(String::new());
            for major in &tasks.tasks {
                lines.extend(format_task_entry(major, false));
                for sub in &major.subtasks {
                    lines.extend(format_task_entry(sub, true));
                }
            }
        }
    }

    let level = if feature.design.is_none() {
        Level::Warning
    } else {
        Level::Info
    };

    Ok((lines.join("\n"), level))
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
    use crate::context::CommandContext;
    use crate::host::FakeHost;

    fn fixture_host() -> FakeHost {
        let host = FakeHost::new().with_root("C:\\proj");
        host.with_file(
            ".kiro/steering/roadmap.md",
            include_str!("../../tests/fixtures/nav/.kiro/steering/roadmap.md"),
        )
        .with_file(
            ".kiro/specs/alpha/spec.json",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/spec.json"),
        )
        .with_file(
            ".kiro/specs/alpha/requirements.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/requirements.md"),
        )
        .with_file(
            ".kiro/specs/alpha/design.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/design.md"),
        )
        .with_file(
            ".kiro/specs/alpha/tasks.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/tasks.md"),
        )
        .with_file(
            ".kiro/specs/gamma/spec.json",
            include_str!("../../tests/fixtures/nav/.kiro/specs/gamma/spec.json"),
        )
        .with_file(
            ".kiro/specs/gamma/requirements.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/gamma/requirements.md"),
        )
        .with_file(
            ".kiro/specs/gamma/tasks.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/gamma/tasks.md"),
        )
    }

    #[test]
    fn boundary_labels() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let alpha = ctx.load("alpha");
        let design = alpha.design.as_ref();

        assert_eq!(boundary_label(design, "Alpha"), "Alpha");
        assert_eq!(boundary_label(design, "alpha"), "Alpha");
        assert_eq!(boundary_label(design, "Empty"), "Empty (no files)");
        assert_eq!(boundary_label(design, "Glue"), "Glue (unknown)");
        assert_eq!(boundary_label(design, ""), "unscoped");
        assert_eq!(boundary_label(None, "Alpha"), "Alpha (unknown)");
    }

    #[test]
    fn component_text_alpha_full() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let alpha = ctx.load("alpha");

        let (text, level) = component_text(alpha);
        assert_eq!(level, Level::Info);

        assert!(text.contains("Spec: alpha"));
        assert!(text.contains("Design: .kiro/specs/alpha/design.md"));
        assert!(text.contains("Tasks: .kiro/specs/alpha/tasks.md"));
        assert!(text.contains(
            "Components: 2 (files: 3, unassigned: 1); tasks: 7 (unscoped: 4, unknown boundary: 1)"
        ));

        assert!(text.contains("Alpha (2 files, 1 tasks)"));
        assert!(text.contains("  src/alpha.rs — Alpha logic"));
        assert!(text.contains("  src/shared.rs"));
        assert!(text.contains("  task 2.1 [x] Build alpha (.kiro/specs/alpha/tasks.md:6)"));

        assert!(text.contains("Empty (no files, 1 tasks)"));
        assert!(text.contains("  task 2.2 [ ] Empty work (.kiro/specs/alpha/tasks.md:9)"));

        assert!(text.contains("Unassigned files (1)"));
        assert!(text.contains("  src/util.rs"));

        assert!(text.contains("Unscoped tasks (4)"));
        assert!(text.contains("  task 1 [ ] Foundation (.kiro/specs/alpha/tasks.md:3)"));
        assert!(text.contains("  task 1.1 [x] Unscoped task (.kiro/specs/alpha/tasks.md:4)"));

        assert!(text.contains("Unknown boundary (1)"));
        assert!(text.contains("  task 3.1 [ ] Glue work (.kiro/specs/alpha/tasks.md:12) — Glue"));
    }

    #[test]
    fn component_text_missing_design() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let gamma = ctx.load("gamma");

        let (text, level) = component_text(gamma);
        assert_eq!(level, Level::Warning);
        assert!(text.contains("Design: .kiro/specs/gamma/design.md (missing)"));
        assert!(text.contains("Components: missing; tasks: 2 (unscoped: 1, unknown boundary: 1)"));
        assert!(text.contains("Unknown boundary (1)"));
        assert!(text.contains("  task 1.1 [ ] Gamma work (.kiro/specs/gamma/tasks.md:4) — Gamma"));
    }

    #[test]
    fn task_text_full_and_filter() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let alpha = ctx.load("alpha");

        let (text, level) = task_text(alpha, None).unwrap();
        assert_eq!(level, Level::Info);
        assert!(text.contains("Count: 7 tasks (completed 2, unscoped 4)"));
        assert!(text.contains("1 [ ] Foundation (.kiro/specs/alpha/tasks.md:3) — unscoped"));
        assert!(text.contains("  1.1 [x] Unscoped task (.kiro/specs/alpha/tasks.md:4) — unscoped"));
        assert!(text.contains("  2.1 [x] Build alpha (.kiro/specs/alpha/tasks.md:6) — Alpha"));
        assert!(text.contains("      src/alpha.rs"));
        assert!(text.contains("      src/shared.rs"));
        assert!(
            text.contains("  2.2 [ ] Empty work (.kiro/specs/alpha/tasks.md:9) — Empty (no files)")
        );
        assert!(
            text.contains("  3.1 [ ] Glue work (.kiro/specs/alpha/tasks.md:12) — Glue (unknown)")
        );

        // Major filter
        let (major_text, _) = task_text(alpha, Some("2")).unwrap();
        assert!(major_text.contains("filter: task 2"));
        assert!(major_text.contains("2 [ ] Core"));
        assert!(major_text.contains("  2.1 [x] Build alpha"));
        assert!(major_text.contains("  2.2 [ ] Empty work"));
        assert!(!major_text.contains("1 [ ] Foundation"));

        // Subtask filter
        let (sub_text, _) = task_text(alpha, Some("2.1")).unwrap();
        assert!(sub_text.contains("filter: task 2.1"));
        assert!(sub_text.contains("  2.1 [x] Build alpha"));
        assert!(sub_text.contains("      src/alpha.rs"));
        assert!(!sub_text.contains("2 [ ] Core"));
        assert!(!sub_text.contains("2.2 [ ] Empty work"));

        // Unknown id error
        let err = task_text(alpha, Some("99.9")).unwrap_err();
        assert_eq!(err, "task `99.9` not found in .kiro/specs/alpha/tasks.md");
    }

    #[test]
    fn task_text_missing_design() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let gamma = ctx.load("gamma");

        let (text, level) = task_text(gamma, None).unwrap();
        assert_eq!(level, Level::Warning);
        assert!(text.contains("Design: .kiro/specs/gamma/design.md (missing)"));
        assert!(
            text.contains("1.1 [ ] Gamma work (.kiro/specs/gamma/tasks.md:4) — Gamma (unknown)")
        );
    }
}
