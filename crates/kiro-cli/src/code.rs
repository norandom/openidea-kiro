//! `code check`, `code baseline`, `code config`: the code gate (spec
//! `code-constraints`).
//!
//! Selection is resolved through the specs' File Structure Plans or Git,
//! the selected sources are read once and measured by `kiro-code`, and the
//! engine judges limits, the ratchet and ownership. This module only
//! connects them.

use kiro_core::code::{
    Before, CodeConfig, CodeFacts, CodeReport, EvaluateInput, FACTS_SCHEMA_VERSION, evaluate,
    highest, matches_any, report, spec_paths, task_paths,
};
use kiro_core::source::{Feature, Workspace};
use serde::{Deserialize, Serialize};

use crate::cli::CodeCheckArgs;
use crate::exit::Failure;
use crate::fs::{FsSource, ProjectRoot};

/// Project-relative path of the committed baseline.
pub const BASELINE_PATH: &str = ".kiro/code-baseline.json";

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
    /// The tracked files that differ from a revision.
    Changed(String),
}

/// Resolves the selection flags; exactly one of them must be given.
pub fn selection(args: &CodeCheckArgs) -> Result<CodeSelection, Failure> {
    let given = usize::from(args.all)
        + usize::from(!args.files.is_empty())
        + usize::from(args.spec.is_some())
        + usize::from(args.task.is_some())
        + usize::from(args.changed.is_some());
    if given != 1 {
        return Err(Failure::new(
            "specify exactly one of --all, --changed <rev>, --files <path>..., --spec <name>, --task <spec>:<id>",
        ));
    }
    if args.all {
        return Ok(CodeSelection::All);
    }
    if let Some(revision) = &args.changed {
        return Ok(CodeSelection::Changed(revision.clone()));
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

/// Loads `.kiro/code-rules.toml`; the built-in limits apply when it is
/// absent. Every problem is reported at once, before anything is measured.
pub fn load_config(root: &ProjectRoot) -> Result<CodeConfig, Failure> {
    let text = crate::fs::read_optional(&root.join(kiro_core::code::CONFIG_PATH))?;
    kiro_core::code::parse_config(text.as_deref()).map_err(|errors| {
        let problems: Vec<String> = errors.iter().map(ToString::to_string).collect();
        Failure::new(format!(
            "CODE_CONFIG_INVALID {}: {}",
            kiro_core::code::CONFIG_PATH,
            problems.join("; ")
        ))
    })
}

/// The committed baseline: the facts of a whole-tree measurement.
#[derive(Debug, Serialize, Deserialize)]
struct BaselineFile {
    schema_version: u32,
    /// Digest of the configuration in force when the baseline was captured.
    config_digest: String,
    facts: CodeFacts,
}

/// Measures the whole tree and returns the baseline document. The same
/// tree gives the same bytes: sorted facts, no timestamp.
pub fn baseline_document(root: &ProjectRoot) -> Result<String, Failure> {
    let config = load_config(root)?;
    let bundle =
        kiro_code::read_bundle(root.path(), &|path| kiro_code::profile_for(path).is_some())
            .map_err(|error| Failure::new(error.to_string()))?;
    let facts = kiro_code::measure(&bundle);
    let failed: Vec<&str> = facts
        .coverage
        .iter()
        .filter(|cell| cell.state == kiro_core::code::Completeness::Failed)
        .map(|cell| cell.path.as_str())
        .collect();
    if !failed.is_empty() {
        return Err(Failure::new(format!(
            "a baseline needs a complete measurement; these files do not parse: {}",
            failed.join(", ")
        )));
    }
    let document = BaselineFile {
        schema_version: FACTS_SCHEMA_VERSION,
        config_digest: config.digest(),
        facts,
    };
    let mut text = serde_json::to_string_pretty(&document)
        .map_err(|error| Failure::new(format!("failed to serialize the baseline: {error}")))?;
    text.push('\n');
    Ok(text)
}

/// The side a run is compared with.
struct BeforeSide {
    facts: CodeFacts,
    identity: String,
    renames: Vec<(String, String)>,
    /// Paths of the before side inside the accounting scope.
    paths: Vec<String>,
}

/// Reads the committed baseline, if there is one. A baseline of another
/// schema version is not compared; the reason is given instead.
fn read_baseline(root: &ProjectRoot) -> Result<Result<CodeFacts, String>, Failure> {
    let Some(text) = crate::fs::read_optional(&root.join(BASELINE_PATH))? else {
        return Ok(Err(
            "none: no baseline and no revision given, so nothing is ratcheted".to_string(),
        ));
    };
    let document: BaselineFile = serde_json::from_str(&text).map_err(|error| {
        Failure::new(format!(
            "{BASELINE_PATH} is not a baseline document: {error}; capture it again with `code baseline`"
        ))
    })?;
    if document.schema_version != FACTS_SCHEMA_VERSION {
        return Ok(Err(format!(
            "none: {BASELINE_PATH} has schema version {} and this build reads {FACTS_SCHEMA_VERSION}; capture it again",
            document.schema_version
        )));
    }
    Ok(Ok(document.facts))
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
    // The configuration is validated before anything is read or measured.
    let config = load_config(root)?;
    let features = load_specs(source, discovered);
    let specs: Vec<&Feature> = features.iter().collect();
    let measurable = |path: &str| kiro_code::profile_for(path).is_some();

    // Which design paths the selection names, if it goes through a spec.
    let design_paths: Option<Vec<String>> = match selection {
        CodeSelection::Spec(name) => {
            Some(spec_paths(name, &specs).map_err(|error| Failure::new(error.to_string()))?)
        }
        CodeSelection::Task { spec, id } => {
            Some(task_paths(spec, id, &specs).map_err(|error| Failure::new(error.to_string()))?)
        }
        _ => None,
    };
    // A revision names both the selection and the before side.
    let changes = match selection {
        CodeSelection::Changed(revision) => Some(
            kiro_code::read_before(root.path(), revision, &measurable)
                .map_err(|error| Failure::new(error.to_string()))?,
        ),
        _ => None,
    };
    let wanted = |path: &str| match selection {
        CodeSelection::All => measurable(path),
        CodeSelection::Files(files) => files.iter().any(|file| file == path),
        CodeSelection::Spec(_) | CodeSelection::Task { .. } => design_paths
            .as_ref()
            .is_some_and(|paths| matches_any(paths, path)),
        CodeSelection::Changed(_) => changes.as_ref().is_some_and(|set| {
            measurable(path) && set.changed.iter().any(|changed| changed == path)
        }),
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
    if let Some(first) = facts.problems().first() {
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

    // The before side: the revision when one was named, else the baseline.
    let mut no_before = None;
    let before_side = match changes {
        Some(set) => {
            let mut paths: Vec<String> = set
                .before
                .files
                .iter()
                .map(|file| file.path.clone())
                .collect();
            paths.sort();
            Some(BeforeSide {
                facts: kiro_code::measure(&set.before),
                identity: format!("revision {}", set.revision),
                renames: set.renames,
                paths,
            })
        }
        None => match read_baseline(root)? {
            Ok(baseline) => Some(BeforeSide {
                facts: baseline,
                identity: format!("baseline {BASELINE_PATH}"),
                renames: Vec::new(),
                paths: selected.clone(),
            }),
            Err(reason) => {
                no_before = Some(reason);
                None
            }
        },
    };

    let mut report = evaluate(&EvaluateInput {
        facts: &facts,
        config: &config,
        selected: &selected,
        specs: &specs,
        before: before_side.as_ref().map(|side| Before {
            facts: &side.facts,
            identity: &side.identity,
            renames: &side.renames,
        }),
        before_paths: before_side
            .as_ref()
            .map_or(&[][..], |side| side.paths.as_slice()),
    });
    if let Some(reason) = no_before
        && reason != report.coverage.before_side
    {
        // Why there is no before side belongs in the coverage line.
        if let Some(last) = report.findings.last_mut() {
            last.finding.message = last
                .finding
                .message
                .replace(&report.coverage.before_side, &reason);
        }
        report.coverage.before_side = reason;
    }

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
