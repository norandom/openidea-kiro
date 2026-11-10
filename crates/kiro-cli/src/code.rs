//! `code check`: the code gate (spec `code-constraints`).
//!
//! Selection is resolved through the specs' File Structure Plans, the
//! selected sources are read once and measured by `kiro-code`, and the
//! engine judges limits and ownership. This module only connects them.

use kiro_core::code::{
    CodeConfig, CodeReport, EvaluateInput, evaluate, highest, matches_any, report, spec_paths,
    task_paths,
};
use kiro_core::source::{Feature, Workspace};

use crate::cli::CodeCheckArgs;
use crate::exit::Failure;
use crate::fs::{FsSource, ProjectRoot};

/// What `code check` judges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeSelection {
    /// Every file a measurement profile covers.
    All,
    /// The given project-relative paths.
    Files(Vec<String>),
    /// The files a spec's design maps.
    Spec(String),
    /// The files of the components a task's boundary names.
    Task { spec: String, id: String },
}

/// Resolves the selection flags; exactly one of them must be given.
pub fn selection(args: &CodeCheckArgs) -> Result<CodeSelection, Failure> {
    let given = usize::from(args.all)
        + usize::from(!args.files.is_empty())
        + usize::from(args.spec.is_some())
        + usize::from(args.task.is_some());
    if given != 1 {
        return Err(Failure::new(
            "specify exactly one of --all, --files <path>..., --spec <name>, --task <spec>:<id>",
        ));
    }
    if args.all {
        return Ok(CodeSelection::All);
    }
    if let Some(spec) = &args.spec {
        return Ok(CodeSelection::Spec(spec.clone()));
    }
    if let Some(task) = &args.task {
        let (spec, id) = task.split_once(':').ok_or_else(|| {
            Failure::new(format!(
                "invalid task `{task}`, expected <spec>:<id>, for example code-constraints:3.1"
            ))
        })?;
        return Ok(CodeSelection::Task {
            spec: spec.to_string(),
            id: id.to_string(),
        });
    }
    let files = args
        .files
        .iter()
        .map(|path| path.replace('\\', "/"))
        .map(|path| path.trim_start_matches("./").to_string())
        .collect();
    Ok(CodeSelection::Files(files))
}

/// Loads every spec in precedence order: roadmap order first, then the
/// remaining discovered specs by name.
fn load_specs(source: FsSource, discovered: Vec<String>) -> Vec<Feature> {
    let mut workspace = Workspace::new(source, discovered);
    workspace.adopt_roadmap_features();
    let mut order: Vec<String> = workspace
        .load_roadmap()
        .map(|roadmap| {
            roadmap
                .entries
                .iter()
                .map(|entry| entry.name.clone())
                .collect()
        })
        .unwrap_or_default();
    for name in workspace.known_features() {
        if !order.contains(&name) {
            order.push(name);
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    order
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .map(|name| workspace.load(&name).clone())
        .collect()
}

/// The result of a run: the report and, on request, the highest values.
pub struct CodeRun {
    pub report: CodeReport,
    /// One line per scope and metric, when `--show-highest` was given.
    pub highest: Vec<String>,
}

/// Runs the code check.
pub fn run(
    root: &ProjectRoot,
    source: FsSource,
    discovered: Vec<String>,
    selection: &CodeSelection,
    show_highest: bool,
) -> Result<CodeRun, Failure> {
    let features = load_specs(source, discovered);
    let specs: Vec<&Feature> = features.iter().collect();

    // Which design paths the selection names, if it goes through a spec.
    let design_paths: Option<Vec<String>> = match selection {
        CodeSelection::Spec(name) => {
            Some(spec_paths(name, &specs).map_err(|error| Failure::new(error.to_string()))?)
        }
        CodeSelection::Task { spec, id } => {
            Some(task_paths(spec, id, &specs).map_err(|error| Failure::new(error.to_string()))?)
        }
        CodeSelection::All | CodeSelection::Files(_) => None,
    };
    let wanted = |path: &str| match selection {
        CodeSelection::All => kiro_code::profile_for(path).is_some(),
        CodeSelection::Files(files) => files.iter().any(|file| file == path),
        CodeSelection::Spec(_) | CodeSelection::Task { .. } => design_paths
            .as_ref()
            .is_some_and(|paths| matches_any(paths, path)),
    };

    let bundle = kiro_code::read_bundle(root.path(), &wanted)
        .map_err(|error| Failure::new(error.to_string()))?;
    if let CodeSelection::Files(files) = selection {
        for file in files {
            let found = bundle.files.iter().any(|text| &text.path == file)
                || bundle.unreadable.iter().any(|entry| &entry.path == file);
            if !found {
                return Err(Failure::new(format!(
                    "file `{file}` was not found under the project root"
                )));
            }
        }
    }
    let facts = kiro_code::measure(&bundle);
    let problems = facts.problems();
    if let Some(first) = problems.first() {
        return Err(Failure::new(format!("invalid measurement: {first}")));
    }

    let mut selected: Vec<String> = bundle
        .files
        .iter()
        .map(|file| file.path.clone())
        .chain(bundle.unreadable.iter().map(|entry| entry.path.clone()))
        .collect();
    selected.sort();
    selected.dedup();

    let config = CodeConfig::default();
    let report = evaluate(&EvaluateInput {
        facts: &facts,
        config: &config,
        selected: &selected,
        specs: &specs,
    });
    let highest_lines = if show_highest {
        highest(&facts, &|_| true)
            .into_iter()
            .map(|observation| {
                format!(
                    "{} {}: {} at {}:{} ({})",
                    observation.entity.scope.label(),
                    observation.metric.name,
                    report::display_number(observation.value),
                    observation.entity.path,
                    observation.line,
                    observation.entity.qualified_name
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(CodeRun {
        report,
        highest: highest_lines,
    })
}
