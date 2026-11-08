//! Spec visualization views: status, spec graph, and task graph.
//!
//! Fronts the engine's viz module with the same layout and wording as the plugin.

use kiro_core::source::Workspace;
use kiro_core::spec::Phase;
use kiro_core::viz::graph::{graph_summary, graph_view};
use kiro_core::viz::status::{status_text, status_view};
use kiro_core::viz::tasks::{tasks_summary, tasks_view};
use kiro_core::viz::{CAPS, Format, RawLabeler, compose, render_body};

use crate::exit::Failure;
use crate::fs::{FsSource, ProjectRoot};

fn check_discovered(discovered: &[String], spec: &str) -> Result<(), Failure> {
    if !discovered.iter().any(|s| s == spec) {
        let disc_str = if discovered.is_empty() {
            "none".to_string()
        } else {
            discovered.join(", ")
        };
        return Err(Failure::new(format!(
            "spec `{spec}` was not found; discovered: {disc_str}"
        )));
    }
    Ok(())
}

/// Renders the spec status view for `spec`.
pub fn status(
    _root: &ProjectRoot,
    source: FsSource,
    discovered: &[String],
    spec: &str,
) -> Result<String, Failure> {
    check_discovered(discovered, spec)?;
    let mut ws = Workspace::new(source, discovered.to_vec());
    ws.adopt_roadmap_features();
    let superseded = ws.load(spec).phase() == Phase::Superseded;
    let chain = if superseded {
        Some(ws.successor_chain(spec))
    } else {
        None
    };
    let feature = ws.load(spec);
    let view = status_view(feature, chain.as_ref());
    let mut text = status_text(&view);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// Renders the spec graph view for `spec`.
pub fn graph(
    _root: &ProjectRoot,
    source: FsSource,
    discovered: &[String],
    spec: &str,
    table: bool,
) -> Result<String, Failure> {
    check_discovered(discovered, spec)?;
    let mut ws = Workspace::new(source, discovered.to_vec());
    ws.adopt_roadmap_features();
    let feature = ws.load(spec);
    let lab = RawLabeler;
    let view = graph_view(feature, &lab).ok_or_else(|| {
        Failure::new(format!(
            "spec `{spec}` has no readable requirements.md, design.md or tasks.md"
        ))
    })?;
    let format = if table {
        Format::Table
    } else {
        Format::Mermaid
    };
    let body = render_body(&view.graph, format, &CAPS);
    let summary = graph_summary(&view, &body);
    let mut text = compose(&summary, &body.text);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// Renders the task graph view for `spec`.
pub fn tasks(
    _root: &ProjectRoot,
    source: FsSource,
    discovered: &[String],
    spec: &str,
    table: bool,
) -> Result<String, Failure> {
    check_discovered(discovered, spec)?;
    let mut ws = Workspace::new(source, discovered.to_vec());
    ws.adopt_roadmap_features();
    let feature = ws.load(spec);
    let tasks_doc = feature.tasks.as_ref().ok_or_else(|| {
        Failure::new(format!(
            "spec `{spec}` has no readable .kiro/specs/{spec}/tasks.md"
        ))
    })?;
    let view = tasks_view(spec, tasks_doc);
    let format = if table {
        Format::Table
    } else {
        Format::Mermaid
    };
    let body = render_body(&view.graph, format, &CAPS);
    let summary = tasks_summary(&view, &body);
    let mut text = compose(&summary, &body.text);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn undiscovered_spec_error() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, ".");
        let source = FsSource::new(root.clone(), false);
        let discovered = vec!["clean".to_string()];

        let err = status(&root, source.clone(), &discovered, "nope").unwrap_err();
        assert_eq!(err.0, "spec `nope` was not found; discovered: clean");

        let err_empty = status(&root, source, &[], "nope").unwrap_err();
        assert_eq!(err_empty.0, "spec `nope` was not found; discovered: none");
    }

    #[test]
    fn unreadable_documents_error() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, ".");

        let specs_dir = dir.path().join(".kiro").join("specs").join("empty-spec");
        crate::fs::create_dir_all(&specs_dir).unwrap();
        crate::fs::write_output(
            &specs_dir.join("spec.json"),
            r#"{"feature_name": "empty-spec", "phase": "initialized"}"#,
        )
        .unwrap();

        let source = FsSource::new(root.clone(), false);
        let discovered = vec!["empty-spec".to_string()];

        let graph_err = graph(&root, source.clone(), &discovered, "empty-spec", false).unwrap_err();
        assert_eq!(
            graph_err.0,
            "spec `empty-spec` has no readable requirements.md, design.md or tasks.md"
        );

        let tasks_err = tasks(&root, source, &discovered, "empty-spec", false).unwrap_err();
        assert_eq!(
            tasks_err.0,
            "spec `empty-spec` has no readable .kiro/specs/empty-spec/tasks.md"
        );
    }
}
