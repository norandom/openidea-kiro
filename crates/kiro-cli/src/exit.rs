//! Exit model, process exit codes, and failure representation.
//!
//! The definitions live in the shared `kiro-cli-common` library (spec
//! `cli-contracts`); this module re-exports them so existing paths keep
//! working.

pub use kiro_cli_common::{Exit, FailOn, Failure, meets, parse_severity, verdict};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use kiro_core::finding::{Finding, Severity};
    use std::process::ExitCode;

    #[test]
    fn meets_truth_table() {
        assert!(meets(FailOn::Error, Severity::Error));
        assert!(!meets(FailOn::Error, Severity::Warning));
        assert!(!meets(FailOn::Error, Severity::Info));

        assert!(meets(FailOn::Warning, Severity::Error));
        assert!(meets(FailOn::Warning, Severity::Warning));
        assert!(!meets(FailOn::Warning, Severity::Info));
    }

    #[test]
    fn verdict_evaluation() {
        let f_info = Finding::new(
            kiro_core::finding::codes::CHECK_COVERAGE,
            "file.md",
            "info message",
        )
        .severity(Severity::Info);
        let f_warn = Finding::new(
            kiro_core::finding::codes::SPEC_DOC_MISSING,
            "file.md",
            "warn message",
        )
        .severity(Severity::Warning);
        let f_err = Finding::new(
            kiro_core::finding::codes::CONTRADICTION,
            "file.md",
            "err message",
        )
        .severity(Severity::Error);

        // Empty findings
        assert_eq!(verdict(FailOn::Error, &[]), Exit::Clean);
        assert_eq!(verdict(FailOn::Warning, &[]), Exit::Clean);

        // Info only
        assert_eq!(
            verdict(FailOn::Error, std::slice::from_ref(&f_info)),
            Exit::Clean
        );
        assert_eq!(
            verdict(FailOn::Warning, std::slice::from_ref(&f_info)),
            Exit::Clean
        );

        // Warning
        assert_eq!(
            verdict(FailOn::Error, std::slice::from_ref(&f_warn)),
            Exit::Clean
        );
        assert_eq!(
            verdict(FailOn::Warning, std::slice::from_ref(&f_warn)),
            Exit::Findings
        );

        // Error
        assert_eq!(
            verdict(FailOn::Error, std::slice::from_ref(&f_err)),
            Exit::Findings
        );
        assert_eq!(
            verdict(FailOn::Warning, std::slice::from_ref(&f_err)),
            Exit::Findings
        );

        // Mixed
        assert_eq!(
            verdict(FailOn::Error, &[f_info.clone(), f_warn.clone()]),
            Exit::Clean
        );
        assert_eq!(
            verdict(FailOn::Warning, &[f_info, f_warn, f_err]),
            Exit::Findings
        );
    }

    #[test]
    fn failure_line_flattening() {
        let failure = Failure::new("line 1\n  line 2\r\n\nline 3  ");
        assert_eq!(failure.0, "line 1; line 2; line 3");

        let single = Failure::new("simple failure");
        assert_eq!(single.0, "simple failure");
    }

    #[test]
    fn parse_severity_cases() {
        assert_eq!(parse_severity("error"), Some(Severity::Error));
        assert_eq!(parse_severity("ERROR"), Some(Severity::Error));
        assert_eq!(parse_severity("warning"), Some(Severity::Warning));
        assert_eq!(parse_severity("Warning"), Some(Severity::Warning));
        assert_eq!(parse_severity("info"), Some(Severity::Info));
        assert_eq!(parse_severity("INFO"), Some(Severity::Info));
        assert_eq!(parse_severity("other"), None);
        assert_eq!(parse_severity(""), None);
    }

    #[test]
    fn exit_to_exit_code() {
        assert_eq!(Exit::Clean.code(), 0);
        assert_eq!(Exit::Findings.code(), 1);
        assert_eq!(Exit::Failure.code(), 2);

        let code_clean: ExitCode = Exit::Clean.into();
        let code_findings: ExitCode = Exit::Findings.into();
        let code_failure: ExitCode = Exit::Failure.into();

        let clean_str = format!("{code_clean:?}");
        let findings_str = format!("{code_findings:?}");
        let failure_str = format!("{code_failure:?}");

        assert!(
            clean_str == "ExitCode(ExitCode(0))"
                || clean_str == "ExitCode(unix_exit_status(0))"
                || clean_str.contains('0'),
            "unexpected clean exit code debug representation: {clean_str}"
        );
        assert!(
            findings_str == "ExitCode(ExitCode(1))"
                || findings_str == "ExitCode(unix_exit_status(1))"
                || findings_str.contains('1'),
            "unexpected findings exit code debug representation: {findings_str}"
        );
        assert!(
            failure_str == "ExitCode(ExitCode(2))"
                || failure_str == "ExitCode(unix_exit_status(2))"
                || failure_str.contains('2'),
            "unexpected failure exit code debug representation: {failure_str}"
        );
    }

    #[test]
    fn fixtures_observable_contract() {
        use kiro_core::checks::{CheckConfig, run_checks};
        use kiro_core::finding::codes;
        use kiro_core::source::{MemorySource, Workspace};
        use kiro_core::vocab::{Status, parse};

        // 1. Merge target existing.toml loads cleanly
        let existing_toml = include_str!("../fixtures/import/existing.toml");
        let loaded = parse(existing_toml, "existing.toml");
        assert_eq!(loaded.status, Status::Loaded);
        assert!(
            loaded.findings.is_empty(),
            "existing.toml must produce zero warnings or errors: {:?}",
            loaded.findings
        );

        // 2. Warnings tree yields exactly one SPEC_DOC_MISSING warning
        let warnings_vocab = include_str!("../fixtures/trees/warnings/.kiro/vocabulary.toml");
        let warnings_roadmap = include_str!("../fixtures/trees/warnings/.kiro/steering/roadmap.md");
        let late_spec = include_str!("../fixtures/trees/warnings/.kiro/specs/late/spec.json");
        let late_req = include_str!("../fixtures/trees/warnings/.kiro/specs/late/requirements.md");
        let late_tasks = include_str!("../fixtures/trees/warnings/.kiro/specs/late/tasks.md");

        let src_warnings = MemorySource::new()
            .with(".kiro/vocabulary.toml", warnings_vocab)
            .with(".kiro/steering/roadmap.md", warnings_roadmap)
            .with(".kiro/specs/late/spec.json", late_spec)
            .with(".kiro/specs/late/requirements.md", late_req)
            .with(".kiro/specs/late/tasks.md", late_tasks);

        let mut ws_warnings = Workspace::new(src_warnings, vec!["late".to_string()]);
        let late_feature = ws_warnings.load("late");
        assert_eq!(late_feature.findings.len(), 1);
        assert_eq!(
            late_feature.findings.first().map(|f| f.code),
            Some(codes::SPEC_DOC_MISSING)
        );

        let check_rep = run_checks(&mut ws_warnings, "late", &CheckConfig::default());
        let (errs, warns, _infos) = check_rep.counts();
        assert_eq!(errs, 0);
        assert_eq!(warns, 0);

        // 3. Vocab findings tree vocabulary has exactly one TERM_NO_DEFINITION warning
        let vocab_findings_toml =
            include_str!("../fixtures/trees/vocab-findings/.kiro/vocabulary.toml");
        let loaded_vf = parse(vocab_findings_toml, "vocabulary.toml");
        assert_eq!(loaded_vf.status, Status::Loaded);
        assert_eq!(loaded_vf.findings.len(), 1);
        assert_eq!(
            loaded_vf.findings.first().map(|f| f.code),
            Some(codes::TERM_NO_DEFINITION)
        );
    }
}
