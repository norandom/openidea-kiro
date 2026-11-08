//! The `check` feature module: spec checks and quality verification (task 6.1).
//!
//! Provides the four spec-addressable commands:
//! - `check.spec`: runs all three checks with patterns off and term suggestions on.
//! - `check.ears`: runs EARS shape check only, with pattern reporting enabled.
//! - `check.terms`: runs declared term scope resolution and suggestion checks only.
//! - `check.contradictions`: runs bounded decision-diagram contradiction search only.

use crate::args::Request;
use crate::context::CommandContext;
use crate::host::LogLevel;
use crate::module::FeatureModule;
use crate::present::{self, Output};
use crate::registry::{CommandSpec, Group, Positional};
use kiro_core::checks::ears::{EarsSummary, Pattern};
use kiro_core::checks::{run_checks, CheckConfig, CheckKind, CheckReport, CheckStatus};
use kiro_core::vocab::Status;

pub struct Check;

pub static COMMANDS: [CommandSpec; 4] = [
    CommandSpec {
        group: Group::Check,
        name: "spec",
        title: "Kiro: Check Spec",
        spec_addressable: true,
        insert_capable: true,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Check,
        name: "ears",
        title: "Kiro: Check EARS Shape",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Check,
        name: "terms",
        title: "Kiro: Check Terms",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Check,
        name: "contradictions",
        title: "Kiro: Check Contradictions",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
];

impl FeatureModule for Check {
    fn group(&self) -> Group {
        Group::Check
    }

    fn commands(&self) -> &'static [CommandSpec] {
        &COMMANDS
    }

    fn run(
        &self,
        ctx: &mut CommandContext<'_>,
        cmd: &CommandSpec,
        req: &Request,
    ) -> Result<Output, String> {
        match cmd.name {
            "spec" => spec(ctx, req),
            "ears" => ears(ctx, req),
            "terms" => terms(ctx, req),
            "contradictions" => contradictions(ctx, req),
            other => Err(present::error_line(&format!("unknown check command: {other}"))),
        }
    }
}

fn run_report(
    ctx: &mut CommandContext<'_>,
    req: &Request,
    config: &CheckConfig,
) -> Result<(String, CheckReport), String> {
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let spec_name = resolved.name;
    let report = run_checks(&mut ctx.workspace, &spec_name, config);
    ctx.host.log(
        LogLevel::Debug,
        &format!(
            "check: spec={}, status={:?}, counts={:?}",
            spec_name,
            report.status,
            report.counts()
        ),
    );
    Ok((spec_name, report))
}

fn spec(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let (spec_name, report) = run_report(ctx, req, &CheckConfig::default())?;
    let lines = vec![
        status_line(CheckKind::Ears, &report),
        status_line(CheckKind::Terms, &report),
        status_line(CheckKind::Contradictions, &report),
    ];
    Ok(render(&spec_name, &report, lines))
}

fn ears(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let cfg = CheckConfig {
        ears: true,
        terms: false,
        contradictions: false,
        report_patterns: true,
        ..Default::default()
    };
    let (spec_name, report) = run_report(ctx, req, &cfg)?;
    let mut lines = vec![status_line(CheckKind::Ears, &report)];
    if let Some(ears_sum) = &report.ears {
        lines.push(patterns_line(ears_sum));
    }
    Ok(render(&spec_name, &report, lines))
}

fn terms(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let (spec_name, report) = run_report(ctx, req, &CheckConfig::only(CheckKind::Terms))?;
    let lines = vec![status_line(CheckKind::Terms, &report)];
    Ok(render(&spec_name, &report, lines))
}

fn contradictions(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let (spec_name, report) = run_report(ctx, req, &CheckConfig::only(CheckKind::Contradictions))?;
    let lines = vec![status_line(CheckKind::Contradictions, &report)];
    Ok(render(&spec_name, &report, lines))
}

pub fn title(spec: &str, report: &CheckReport) -> String {
    if report.findings.is_empty() {
        format!("Kiro: {spec} — no findings")
    } else {
        let (errs, warns, infos) = report.counts();
        let err_str = if errs == 1 { "error" } else { "errors" };
        let warn_str = if warns == 1 { "warning" } else { "warnings" };
        format!("Kiro: {spec} — {errs} {err_str}, {warns} {warn_str}, {infos} info")
    }
}

pub fn scope_line(spec: &str, report: &CheckReport) -> String {
    let topics_str = if report.topics.is_empty() {
        "none".to_string()
    } else {
        report.topics.join(", ")
    };
    let vocab_str = match report.vocabulary {
        Status::Loaded => "loaded",
        Status::Missing => "missing",
        Status::Invalid => "invalid",
    };
    format!("Spec: {spec} (topics: {topics_str}) — vocabulary: {vocab_str}")
}

pub fn patterns_line(ears_sum: &EarsSummary) -> String {
    let get_pat = |p: Pattern| ears_sum.patterns.get(&p).copied().unwrap_or(0);
    format!(
        "Patterns: ubiquitous {}, event-driven {}, state-driven {}, unwanted-behaviour {}, optional-feature {}, complex {}, unclassified {}",
        get_pat(Pattern::Ubiquitous),
        get_pat(Pattern::EventDriven),
        get_pat(Pattern::StateDriven),
        get_pat(Pattern::UnwantedBehaviour),
        get_pat(Pattern::OptionalFeature),
        get_pat(Pattern::Complex),
        ears_sum.unclassified,
    )
}

pub fn status_line(kind: CheckKind, report: &CheckReport) -> String {
    match report.status_of(kind) {
        CheckStatus::Skipped { reason } => {
            let name = match kind {
                CheckKind::Ears => "EARS",
                CheckKind::Terms => "Terms",
                CheckKind::Contradictions => "Contradictions",
            };
            format!("{name}: skipped ({reason})")
        }
        CheckStatus::Ran { findings } => match kind {
            CheckKind::Ears => {
                let criteria = report.ears.as_ref().map_or(0, |s| s.criteria);
                let finding_word = if *findings == 1 { "finding" } else { "findings" };
                format!("EARS: {criteria} criteria checked, {findings} {finding_word}")
            }
            CheckKind::Terms => {
                if let Some(s) = &report.terms {
                    format!(
                        "Terms: {} declared — {} resolved, {} deprecated, {} mismatched, {} undeclared, {} ambiguous; {} suggested",
                        s.declared,
                        s.resolved,
                        s.deprecated,
                        s.mismatched,
                        s.undeclared,
                        s.ambiguous,
                        s.suggested,
                    )
                } else {
                    format!("Terms: ran ({findings} findings)")
                }
            }
            CheckKind::Contradictions => {
                if let Some(s) = &report.contradictions {
                    format!(
                        "Contradictions: {} guarded criteria, {} pairs considered, {} found, {} refused (caps: {} variables, {} nodes)",
                        s.guarded,
                        s.pairs_considered,
                        s.contradictions,
                        s.refused_pairs,
                        s.max_variables,
                        s.max_nodes,
                    )
                } else {
                    format!("Contradictions: ran ({findings} findings)")
                }
            }
        },
    }
}

pub fn vocab_pointer(report: &CheckReport) -> Option<String> {
    let terms_skipped = report.status_of(CheckKind::Terms);
    let cont_skipped = report.status_of(CheckKind::Contradictions);
    let needs_pointer = match (terms_skipped, cont_skipped) {
        (CheckStatus::Skipped { reason: r1 }, _)
            if r1.contains("vocabulary") || r1.contains("topics") =>
        {
            true
        }
        (_, CheckStatus::Skipped { reason: r2 })
            if r2.contains("vocabulary") || r2.contains("topics") =>
        {
            true
        }
        _ => false,
    };
    if needs_pointer {
        Some("See docs/vocabulary.md or run Kiro: Vocabulary Topics.".to_string())
    } else {
        None
    }
}

pub fn render(spec: &str, report: &CheckReport, lines: Vec<String>) -> Output {
    let mut intro_parts = Vec::new();
    intro_parts.push(scope_line(spec, report));
    for line in lines {
        intro_parts.push(line);
    }
    if let Some(ptr) = vocab_pointer(report) {
        intro_parts.push(ptr);
    }
    let intro = intro_parts.join("\n");
    let t = title(spec, report);

    if report.findings.is_empty() {
        Output::info(t, format!("{intro}\n\nNo findings."))
    } else {
        Output::with_findings(t, &intro, &report.findings)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use kiro_core::checks::contradiction::SearchSummary;
    use kiro_core::checks::ears::EarsSummary;
    use kiro_core::checks::terms::TermsSummary;
    use kiro_core::finding::{codes, Finding, Severity};

    fn make_dummy_report(findings: Vec<Finding>, topics: Vec<String>, vocab: Status) -> CheckReport {
        let mut status = BTreeMap::new();
        status.insert(CheckKind::Ears, CheckStatus::Ran { findings: 0 });
        status.insert(CheckKind::Terms, CheckStatus::Ran { findings: 0 });
        status.insert(
            CheckKind::Contradictions,
            CheckStatus::Ran { findings: 0 },
        );

        CheckReport {
            feature: "test_feat".to_string(),
            requirements_path: ".kiro/specs/test_feat/requirements.md".to_string(),
            topics,
            vocabulary: vocab,
            status,
            ears: Some(EarsSummary {
                criteria: 5,
                patterns: BTreeMap::new(),
                unclassified: 1,
            }),
            terms: Some(TermsSummary {
                declared: 3,
                resolved: 3,
                deprecated: 0,
                mismatched: 0,
                undeclared: 0,
                ambiguous: 0,
                suggested: 1,
            }),
            contradictions: Some(SearchSummary {
                guarded: 2,
                pairs_considered: 1,
                contradictions: 0,
                refused_pairs: 0,
                variables: 8,
                max_variables: 512,
                max_nodes: 100_000,
            }),
            findings,
        }
    }

    #[test]
    fn title_formatting_with_and_without_findings() {
        let rep_empty = make_dummy_report(vec![], vec![], Status::Loaded);
        assert_eq!(title("demo", &rep_empty), "Kiro: demo — no findings");

        let rep_findings = make_dummy_report(
            vec![
                Finding::new(codes::EARS_NO_SHALL, "f.md", "err").severity(Severity::Error),
                Finding::new(codes::EARS_MODAL, "f.md", "warn1").severity(Severity::Warning),
                Finding::new(codes::EARS_MODAL, "f.md", "warn2").severity(Severity::Warning),
                Finding::new(codes::CHECK_COVERAGE, "f.md", "inf").severity(Severity::Info),
            ],
            vec![],
            Status::Loaded,
        );
        assert_eq!(
            title("demo", &rep_findings),
            "Kiro: demo — 1 error, 2 warnings, 1 info"
        );
    }

    #[test]
    fn scope_line_formatting() {
        let rep_none = make_dummy_report(vec![], vec![], Status::Missing);
        assert_eq!(
            scope_line("feat", &rep_none),
            "Spec: feat (topics: none) — vocabulary: missing"
        );

        let rep_topics = make_dummy_report(
            vec![],
            vec!["ops".to_string(), "pm".to_string()],
            Status::Loaded,
        );
        assert_eq!(
            scope_line("feat", &rep_topics),
            "Spec: feat (topics: ops, pm) — vocabulary: loaded"
        );
    }

    #[test]
    fn status_lines_and_patterns_line() {
        let rep = make_dummy_report(vec![], vec![], Status::Loaded);
        assert_eq!(
            status_line(CheckKind::Ears, &rep),
            "EARS: 5 criteria checked, 0 findings"
        );
        assert_eq!(
            status_line(CheckKind::Terms, &rep),
            "Terms: 3 declared — 3 resolved, 0 deprecated, 0 mismatched, 0 undeclared, 0 ambiguous; 1 suggested"
        );
        assert_eq!(
            status_line(CheckKind::Contradictions, &rep),
            "Contradictions: 2 guarded criteria, 1 pairs considered, 0 found, 0 refused (caps: 512 variables, 100000 nodes)"
        );

        let mut rep_skipped = rep.clone();
        rep_skipped.status.insert(
            CheckKind::Terms,
            CheckStatus::Skipped {
                reason: "spec declares no topics".to_string(),
            },
        );
        assert_eq!(
            status_line(CheckKind::Terms, &rep_skipped),
            "Terms: skipped (spec declares no topics)"
        );
    }

    #[test]
    fn vocab_pointer_detection() {
        let rep_ok = make_dummy_report(vec![], vec![], Status::Loaded);
        assert_eq!(vocab_pointer(&rep_ok), None);

        let mut rep_skip = rep_ok;
        rep_skip.status.insert(
            CheckKind::Terms,
            CheckStatus::Skipped {
                reason: "spec declares no topics".to_string(),
            },
        );
        assert_eq!(
            vocab_pointer(&rep_skip),
            Some("See docs/vocabulary.md or run Kiro: Vocabulary Topics.".to_string())
        );
    }

    #[test]
    fn render_with_and_without_findings() {
        let rep_empty = make_dummy_report(vec![], vec![], Status::Loaded);
        let out_empty = render("demo", &rep_empty, vec!["Line1".to_string()]);
        assert_eq!(out_empty.title, "Kiro: demo — no findings");
        assert!(out_empty.text.contains("No findings."));

        let rep_findings = make_dummy_report(
            vec![Finding::new(codes::CHECK_COVERAGE, "f.md", "info message")
                .severity(Severity::Info)],
            vec![],
            Status::Loaded,
        );
        let out_findings = render("demo", &rep_findings, vec!["Line1".to_string()]);
        assert_eq!(out_findings.title, "Kiro: demo — 0 errors, 0 warnings, 1 info");
        assert!(out_findings.text.contains("CHECK_COVERAGE"));
    }
}
