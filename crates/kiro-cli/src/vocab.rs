//! Vocabulary validation and import pipeline orchestration.

use std::path::Path;

use kiro_core::source::Workspace;
use kiro_core::vocab::{Status, load as vocab_load, parse as vocab_parse};

use crate::cli::ImportArgs;
use crate::exit::{Exit, FailOn, Failure, meets};
use crate::fs::{FsSource, ProjectRoot, display_path, read_argument_file};
use crate::report::{VOCABULARY_PATH, VocabReport};

/// Validates a vocabulary file using the engine's loader.
///
/// When `file` is `None`, loads `.kiro/vocabulary.toml` under the project root.
/// When `file` is `Some(path)`, reads and validates that file as given.
///
/// Returns:
/// - `Exit::Findings` when status is `Missing` or any finding meets `fail_on`.
/// - `Exit::Clean` when valid without qualifying findings.
/// - `Err(Failure)` when an explicit argument file cannot be read.
pub fn validate(
    _root: &ProjectRoot,
    source: FsSource,
    file: Option<&Path>,
    fail_on: FailOn,
) -> Result<(VocabReport, Exit), Failure> {
    let (file_display, loaded) = match file {
        None => {
            let mut ws = Workspace::new(source, Vec::new());
            let loaded = vocab_load(&mut ws);
            (VOCABULARY_PATH.to_string(), loaded)
        }
        Some(path) => {
            let text = read_argument_file(path)?;
            let disp = display_path(path);
            let loaded = vocab_parse(&text, &disp);
            (disp, loaded)
        }
    };

    let report = VocabReport {
        file: file_display,
        status: loaded.status,
        topics: loaded.vocabulary.topics().len(),
        terms: loaded.vocabulary.len(),
        findings: loaded.findings,
    };

    let exit = if report.status == Status::Missing
        || report.findings.iter().any(|f| meets(fail_on, f.severity))
    {
        Exit::Findings
    } else {
        Exit::Clean
    };

    Ok((report, exit))
}

/// Orchestrates vocabulary import from external sources.
///
/// Full pipeline implemented in tasks 5 and 6.
pub fn import(_root: &ProjectRoot, _args: &ImportArgs) -> Result<Exit, Failure> {
    Ok(Exit::Clean)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use kiro_core::finding::codes;
    use tempfile::tempdir;

    #[test]
    fn validate_missing_default_vocabulary() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, ".");
        let source = FsSource::new(root.clone(), false);

        let (report, exit) = validate(&root, source, None, FailOn::Error).unwrap();
        assert_eq!(report.status, Status::Missing);
        assert_eq!(exit, Exit::Findings);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code, codes::VOCAB_MISSING);
    }

    #[test]
    fn validate_explicit_file_valid_and_invalid() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, ".");
        let source = FsSource::new(root.clone(), false);

        // Valid vocabulary TOML
        let valid_file = dir.path().join("valid.toml");
        crate::fs::write_output(
            &valid_file,
            r#"schema_version = "2"
[topics.ops]
name = "Operations"

[terms.deployment]
topic = "ops"
label = "Deployment"
kind = "entity"
definition = "Release of code to a target environment."
"#,
        )
        .unwrap();

        let (rep_valid, exit_valid) =
            validate(&root, source.clone(), Some(&valid_file), FailOn::Error).unwrap();
        assert_eq!(rep_valid.status, Status::Loaded);
        assert_eq!(exit_valid, Exit::Clean);
        assert_eq!(rep_valid.topics, 1);
        assert_eq!(rep_valid.terms, 1);

        // Invalid vocabulary TOML
        let invalid_file = dir.path().join("invalid.toml");
        crate::fs::write_output(&invalid_file, "not a valid toml = [[[").unwrap();
        let (rep_invalid, exit_invalid) =
            validate(&root, source.clone(), Some(&invalid_file), FailOn::Error).unwrap();
        assert_eq!(rep_invalid.status, Status::Invalid);
        assert_eq!(exit_invalid, Exit::Findings);
        assert_eq!(rep_invalid.findings[0].code, codes::VOCAB_INVALID);

        // Unreadable explicit file
        let missing_path = dir.path().join("missing.toml");
        let err = validate(&root, source, Some(&missing_path), FailOn::Error).unwrap_err();
        assert!(err.0.contains("failed to read file"));
    }
}
