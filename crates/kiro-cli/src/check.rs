//! Spec check orchestrator and gate evaluation.

use kiro_core::checks::{CheckConfig, run_checks};
use kiro_core::finding::sort_findings;
use kiro_core::source::{Feature, Workspace};
use kiro_core::vocab::load as vocab_load;

use crate::cli::Selection;
use crate::exit::{FailOn, Failure, verdict};
use crate::fs::{FsSource, ProjectRoot};
use crate::report::{Counts, GateReport, SpecResult, VOCABULARY_PATH};

/// Validates the spec selection against discovered features.
///
/// Returns:
/// - `Ok(Vec<String>)`: the ordered list of spec names to check.
/// - `Err(Failure)`: when `--all` was given but no specs were discovered, or
///   when `--feature` named a spec that does not exist in `discovered`.
pub fn select(selection: &Selection, discovered: &[String]) -> Result<Vec<String>, Failure> {
    match selection {
        Selection::All => {
            if discovered.is_empty() {
                return Err(Failure::new("no specs found under .kiro/specs"));
            }
            Ok(discovered.to_vec())
        }
        Selection::Features(features) => {
            for f in features {
                if !discovered.contains(f) {
                    let disc_str = if discovered.is_empty() {
                        "none".to_string()
                    } else {
                        discovered.join(", ")
                    };
                    return Err(Failure::new(format!(
                        "spec `{f}` was not found; discovered: {disc_str}"
                    )));
                }
            }
            Ok(features.clone())
        }
    }
}

/// Executes all spec checks across `specs` and produces the consolidated [`GateReport`].
#[must_use]
pub fn run(
    root: &ProjectRoot,
    source: FsSource,
    discovered: Vec<String>,
    specs: Vec<String>,
    config: &CheckConfig,
    fail_on: FailOn,
) -> GateReport {
    run_with_features(root, source, discovered, specs, config, fail_on).0
}

/// Like [`run`], and also returns the loaded feature of every checked spec
/// in the same order, for consumers that read the documents themselves.
#[must_use]
pub fn run_with_features(
    _root: &ProjectRoot,
    source: FsSource,
    discovered: Vec<String>,
    specs: Vec<String>,
    config: &CheckConfig,
    fail_on: FailOn,
) -> (GateReport, Vec<Feature>) {
    let mut ws = Workspace::new(source, discovered);
    ws.adopt_roadmap_features();
    let roadmap = ws
        .load_roadmap()
        .map(|r| r.findings.clone())
        .unwrap_or_default();
    let vocabulary = vocab_load(&mut ws);

    let mut checked = Vec::with_capacity(specs.len());
    let mut features: Vec<Feature> = Vec::with_capacity(specs.len());
    for spec in &specs {
        let report = run_checks(&mut ws, spec, config);
        let structural = ws
            .feature(spec)
            .map(|f| f.findings.clone())
            .unwrap_or_default();

        let mut findings = structural;
        for f in &report.findings {
            if f.file != VOCABULARY_PATH {
                findings.push(f.clone());
            }
        }
        sort_findings(&mut findings);

        features.push(ws.load(spec).clone());
        checked.push(SpecResult {
            spec: spec.clone(),
            report,
            findings,
        });
    }

    let all_findings: Vec<_> = vocabulary
        .findings
        .iter()
        .chain(roadmap.iter())
        .chain(checked.iter().flat_map(|r| r.findings.iter()))
        .collect();

    let counts = Counts::from_findings(all_findings.iter().copied());
    let exit = verdict(fail_on, all_findings);

    let report = GateReport {
        specs,
        vocabulary,
        roadmap,
        checked,
        fail_on,
        counts,
        exit,
    };
    (report, features)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use crate::exit::Exit;
    use tempfile::tempdir;

    #[test]
    fn select_all_and_features() {
        let discovered = vec!["alpha".to_string(), "beta".to_string()];

        // All with discovered specs
        assert_eq!(
            select(&Selection::All, &discovered).unwrap(),
            vec!["alpha".to_string(), "beta".to_string()]
        );

        // All with empty discovered
        let empty_disc: Vec<String> = vec![];
        let err_empty = select(&Selection::All, &empty_disc).unwrap_err();
        assert!(err_empty.0.contains("no specs found under .kiro/specs"));

        // Explicit features subset
        let sel_subset = Selection::Features(vec!["beta".to_string()]);
        assert_eq!(
            select(&sel_subset, &discovered).unwrap(),
            vec!["beta".to_string()]
        );

        // Unknown feature
        let sel_unknown = Selection::Features(vec!["gamma".to_string()]);
        let err_unknown = select(&sel_unknown, &discovered).unwrap_err();
        assert_eq!(
            err_unknown.0,
            "spec `gamma` was not found; discovered: alpha, beta"
        );

        // Unknown feature with empty discovered
        let err_unknown_empty = select(&sel_unknown, &empty_disc).unwrap_err();
        assert_eq!(
            err_unknown_empty.0,
            "spec `gamma` was not found; discovered: none"
        );
    }

    #[test]
    fn run_check_gate_orchestrator() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, ".");

        // Construct filesystem tree for checking via crate::fs
        let specs_dir = dir.path().join(".kiro").join("specs").join("demo");
        crate::fs::create_dir_all(&specs_dir).unwrap();
        crate::fs::write_output(
            &specs_dir.join("spec.json"),
            r#"{"feature_name": "demo", "phase": "tasks-generated"}"#,
        )
        .unwrap();
        crate::fs::write_output(
            &specs_dir.join("requirements.md"),
            "# Requirements\n\n### Requirement 1: Alpha\n\n#### Acceptance Criteria\n\n1. The system shall work.\n",
        )
        .unwrap();

        let source = FsSource::new(root.clone(), false);
        let discovered = vec!["demo".to_string()];
        let specs = vec!["demo".to_string()];
        let config = CheckConfig::default();

        let report = run(&root, source, discovered, specs, &config, FailOn::Error);
        assert_eq!(report.specs, vec!["demo".to_string()]);
        assert_eq!(report.checked.len(), 1);
        assert_eq!(report.checked[0].spec, "demo");
        // No errors -> exit Clean
        assert_eq!(report.exit, Exit::Clean);
        assert_eq!(report.counts.errors, 0);
    }
}
