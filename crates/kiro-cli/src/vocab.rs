//! Vocabulary validation and import pipeline orchestration.

use std::path::Path;

use kiro_core::finding::Severity;
use kiro_core::render::text::findings_text;
use kiro_core::source::Workspace;
use kiro_core::vocab::model::Kind;
use kiro_core::vocab::{Status, load as vocab_load, parse as vocab_parse};

use crate::cli::{ImportArgs, KindArg};
use crate::exit::{Exit, FailOn, Failure, meets};
use crate::fs::{
    FsSource, ProjectRoot, display_path, output_exists, read_argument_file, write_output,
};
use crate::import as import_mod;
use crate::import::{Defaults, Header, Input};
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
pub fn import(root: &ProjectRoot, args: &ImportArgs, verbose: bool) -> Result<Exit, Failure> {
    // 1. Read all input files
    let mut inputs = Vec::with_capacity(args.files.len());
    for path in &args.files {
        let text = read_argument_file(path)?;
        let display = display_path(path);
        inputs.push(Input { display, text });
    }

    // 2. Set defaults
    let default_kind = match args.kind {
        KindArg::Entity => Kind::Entity,
        KindArg::Bool => Kind::Bool,
        KindArg::Enum => Kind::Enum { values: Vec::new() },
        KindArg::Int => Kind::Int {
            min: None,
            max: None,
        },
    };
    let defaults = Defaults {
        topic: &args.topic,
        kind: &default_kind,
    };

    // 3. Reader dispatch
    let mut imported = match import_mod::read(args.format, &inputs, &defaults) {
        Ok(imp) => imp,
        Err(finding) => {
            let ft = findings_text(std::slice::from_ref(&finding));
            eprint!("{ft}");
            return Err(Failure::new(finding.message));
        }
    };

    // 4. Deduplicate terms (first in input order wins)
    let mut dedupe_findings = Vec::new();
    imported.terms = import_mod::dedupe(imported.terms, &mut dedupe_findings);
    imported.findings.append(&mut dedupe_findings);

    // 5. Output resolution
    let out_is_stdout = args.out.as_deref() == Some(Path::new("-"));
    let (out_path, out_display) = if out_is_stdout {
        (None, "-".to_string())
    } else {
        match &args.out {
            Some(p) => (Some(p.clone()), display_path(p)),
            None => (
                Some(root.join(VOCABULARY_PATH)),
                VOCABULARY_PATH.to_string(),
            ),
        }
    };

    let input_displays: Vec<String> = inputs.iter().map(|i| i.display.clone()).collect();
    let header = Header {
        format: args.format,
        inputs: &input_displays,
        topic: &args.topic,
    };

    // 6. Merge or render fresh
    let text = if let Some(ref path) = out_path {
        if output_exists(path) {
            if !args.merge {
                return Err(Failure::new(format!(
                    "`{out_display}` exists; use --merge to add terms or choose another --out"
                )));
            }
            let existing_text = read_argument_file(path)?;
            let existing_loaded = vocab_parse(&existing_text, &out_display);
            if existing_loaded.status == Status::Invalid {
                let ft = findings_text(&existing_loaded.findings);
                eprint!("{ft}");
                return Err(Failure::new(format!(
                    "existing vocabulary `{out_display}` is invalid"
                )));
            }
            let mut merge_findings = Vec::new();
            let res = import_mod::merge(
                &existing_text,
                &existing_loaded.vocabulary,
                &imported,
                &imported.terms,
                &header,
                &mut merge_findings,
                &out_display,
            );
            imported.findings.append(&mut merge_findings);
            res
        } else {
            import_mod::render(&imported, &imported.terms, &header)
        }
    } else {
        import_mod::render(&imported, &imported.terms, &header)
    };

    // 7. Validate output text
    let mut val_findings = import_mod::validate_output(&text, &out_display);
    imported.findings.append(&mut val_findings);

    // 8. Write or print
    if let Some(ref path) = out_path {
        write_output(path, &text)?;
        if verbose {
            eprintln!("written: {out_display}");
        }
    } else {
        print!("{text}");
    }

    // 9. Print findings to stderr
    if !imported.findings.is_empty() {
        let ft = findings_text(&imported.findings);
        eprint!("{ft}");
    }

    // 10. Exit code (1 when any is error, else 0)
    if imported
        .findings
        .iter()
        .any(|f| f.severity == Severity::Error)
    {
        Ok(Exit::Findings)
    } else {
        Ok(Exit::Clean)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use crate::cli::InputFormat;
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

    #[test]
    fn import_fresh_refusal_and_merge() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical.clone(), ".");

        let src_file = dir.path().join("terms-page.json");
        crate::fs::write_output(
            &src_file,
            include_str!("../fixtures/import/terms-page.json"),
        )
        .unwrap();

        let out_file = dir.path().join("vocab.toml");

        // 1. Fresh import
        let args_fresh = ImportArgs {
            files: vec![src_file.clone()],
            format: InputFormat::AutoOntologyJson,
            topic: "sdd".to_string(),
            kind: KindArg::Entity,
            out: Some(out_file.clone()),
            merge: false,
        };
        let exit_fresh = import(&root, &args_fresh, false).unwrap();
        assert_eq!(exit_fresh, Exit::Clean);
        assert!(output_exists(&out_file));

        let content = read_argument_file(&out_file).unwrap();
        assert!(content.contains("[topics.sdd]\n"));
        assert!(content.contains("[terms.order]\n"));
        assert!(content.contains("[terms.customer-lifetime-value]\n"));

        // 2. Refusal on existing file without --merge
        let args_refuse = ImportArgs {
            files: vec![src_file.clone()],
            format: InputFormat::AutoOntologyJson,
            topic: "sdd".to_string(),
            kind: KindArg::Entity,
            out: Some(out_file.clone()),
            merge: false,
        };
        let err = import(&root, &args_refuse, false).unwrap_err();
        assert!(err.0.contains("exists; use --merge"));

        // 3. Merge into existing fixture
        let merge_target = dir.path().join("existing.toml");
        crate::fs::write_output(
            &merge_target,
            include_str!("../fixtures/import/existing.toml"),
        )
        .unwrap();

        let args_merge = ImportArgs {
            files: vec![src_file],
            format: InputFormat::AutoOntologyJson,
            topic: "sdd".to_string(),
            kind: KindArg::Entity,
            out: Some(merge_target.clone()),
            merge: true,
        };
        let exit_merge = import(&root, &args_merge, false).unwrap();
        assert_eq!(exit_merge, Exit::Clean);

        let merged_content = read_argument_file(&merge_target).unwrap();
        assert!(merged_content.starts_with(include_str!("../fixtures/import/existing.toml")));
        assert!(merged_content.contains("# Merged by kiro-cli vocab import"));
        assert!(merged_content.contains("[terms.customer-lifetime-value]\n"));
    }
}
