//! Spec ownership of code files and task selection (code-constraints 6).
//!
//! Ownership is the engine's existing join over the design's File Structure
//! Plan and the tasks' boundaries. Nothing is derived a second time here.

use crate::join::{owners_of, path_matches};
use crate::source::Feature;

/// Who owns a file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attribution {
    /// The first owning spec in the given order (roadmap order).
    pub feature: Option<String>,
    pub components: Vec<String>,
    /// Ids of the tasks whose boundary names one of the components.
    pub tasks: Vec<String>,
    /// Further specs that map the same file.
    pub others: Vec<String>,
}

/// Attributes `path` to the specs that map it. `specs` is in precedence
/// order; the first owner is primary and the rest are listed.
#[must_use]
pub fn attribute(path: &str, specs: &[&Feature]) -> Attribution {
    let mut out = Attribution::default();
    for feature in specs {
        let owners = owners_of(feature.design.as_ref(), feature.tasks.as_ref(), path);
        if !owners.covered {
            continue;
        }
        if out.feature.is_none() {
            out.feature = Some(feature.name.clone());
            out.components = owners.components;
            out.tasks = owners.tasks.into_iter().map(|task| task.id).collect();
        } else {
            out.others.push(feature.name.clone());
        }
    }
    out
}

/// Why a selection cannot be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    UnknownSpec {
        name: String,
        available: Vec<String>,
    },
    NoDesign {
        spec: String,
    },
    UnknownTask {
        spec: String,
        task: String,
        available: Vec<String>,
    },
    NoBoundary {
        spec: String,
        task: String,
    },
    UnmappedBoundary {
        spec: String,
        task: String,
        components: Vec<String>,
    },
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSpec { name, available } => write!(
                f,
                "spec `{name}` was not found; available: {}",
                list(available)
            ),
            Self::NoDesign { spec } => {
                write!(f, "spec `{spec}` has no design, so it maps no files")
            }
            Self::UnknownTask {
                spec,
                task,
                available,
            } => write!(
                f,
                "task `{task}` was not found in spec `{spec}`; available: {}",
                list(available)
            ),
            Self::NoBoundary { spec, task } => write!(
                f,
                "task `{spec}:{task}` names no boundary, so it selects no files"
            ),
            Self::UnmappedBoundary {
                spec,
                task,
                components,
            } => write!(
                f,
                "the boundary of task `{spec}:{task}` maps no files; components: {}",
                list(components)
            ),
        }
    }
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

fn find<'a>(name: &str, specs: &[&'a Feature]) -> Result<&'a Feature, SelectionError> {
    specs
        .iter()
        .copied()
        .find(|feature| feature.name == name)
        .ok_or_else(|| SelectionError::UnknownSpec {
            name: name.to_string(),
            available: specs.iter().map(|feature| feature.name.clone()).collect(),
        })
}

/// The design paths a spec maps, in document order.
pub fn spec_paths(name: &str, specs: &[&Feature]) -> Result<Vec<String>, SelectionError> {
    let feature = find(name, specs)?;
    let design = feature
        .design
        .as_ref()
        .ok_or_else(|| SelectionError::NoDesign {
            spec: name.to_string(),
        })?;
    Ok(design.files.iter().map(|file| file.path.clone()).collect())
}

/// The design paths of the components a task's boundary names.
pub fn task_paths(
    spec: &str,
    task: &str,
    specs: &[&Feature],
) -> Result<Vec<String>, SelectionError> {
    let feature = find(spec, specs)?;
    let design = feature
        .design
        .as_ref()
        .ok_or_else(|| SelectionError::NoDesign {
            spec: spec.to_string(),
        })?;
    let tasks = feature.tasks.as_ref();
    let found =
        tasks
            .and_then(|doc| doc.task(task))
            .ok_or_else(|| SelectionError::UnknownTask {
                spec: spec.to_string(),
                task: task.to_string(),
                available: tasks
                    .map(|doc| doc.all().map(|entry| entry.id.clone()).collect())
                    .unwrap_or_default(),
            })?;
    if found.boundary.is_empty() {
        return Err(SelectionError::NoBoundary {
            spec: spec.to_string(),
            task: task.to_string(),
        });
    }
    let paths: Vec<String> = design
        .files
        .iter()
        .filter(|file| {
            file.component
                .as_ref()
                .is_some_and(|component| found.boundary.contains(component))
        })
        .map(|file| file.path.clone())
        .collect();
    if paths.is_empty() {
        return Err(SelectionError::UnmappedBoundary {
            spec: spec.to_string(),
            task: task.to_string(),
            components: found.boundary.clone(),
        });
    }
    Ok(paths)
}

/// Whether a project file is one of the given design paths.
#[must_use]
pub fn matches_any(design_paths: &[String], file: &str) -> bool {
    design_paths
        .iter()
        .any(|design_path| path_matches(design_path, file))
}
