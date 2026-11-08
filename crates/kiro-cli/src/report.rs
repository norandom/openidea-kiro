//! Gate and vocabulary report generation in human-readable text and JSON.

use std::collections::BTreeMap;

use kiro_core::checks::{CheckKind, CheckReport, CheckStatus};
use kiro_core::finding::{Finding, Severity};
use kiro_core::render::text::findings_text;
use kiro_core::vocab::{Loaded, Status};

use crate::exit::{Exit, FailOn, Failure};

/// Fixed schema version of the JSON gate report.
pub const REPORT_SCHEMA_VERSION: &str = "1";

/// Standard project-relative path to the vocabulary file.
pub const VOCABULARY_PATH: &str = ".kiro/vocabulary.toml";

/// Standard project-relative path to the roadmap document.
pub const ROADMAP_PATH: &str = ".kiro/steering/roadmap.md";

/// Result of checking a single specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecResult {
    pub spec: String,
    pub report: CheckReport,
    /// Merged structural and check findings, minus vocabulary findings, sorted.
    pub findings: Vec<Finding>,
}

/// Total counts of findings by severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

impl Counts {
    /// Computes counts from an iterator of findings.
    #[must_use]
    pub fn from_findings<'a>(findings: impl IntoIterator<Item = &'a Finding>) -> Self {
        let mut counts = Self::default();
        for f in findings {
            match f.severity {
                Severity::Error => counts.errors = counts.errors.saturating_add(1),
                Severity::Warning => counts.warnings = counts.warnings.saturating_add(1),
                Severity::Info => counts.info = counts.info.saturating_add(1),
            }
        }
        counts
    }
}

/// Complete gate report across all checked specs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateReport {
    /// Names of checked specs in execution order.
    pub specs: Vec<String>,
    /// Vocabulary load status and findings (printed once).
    pub vocabulary: Loaded,
    /// Roadmap findings (printed once, if non-empty).
    pub roadmap: Vec<Finding>,
    /// Per-spec results.
    pub checked: Vec<SpecResult>,
    /// Severity threshold configured for this run.
    pub fail_on: FailOn,
    /// Aggregated finding counts.
    pub counts: Counts,
    /// Exit verdict (`Clean` or `Findings`).
    pub exit: Exit,
}

impl GateReport {
    /// Iterates over all findings in canonical report order:
    /// vocabulary -> roadmap -> specs in order checked.
    pub fn all_findings(&self) -> impl Iterator<Item = &Finding> {
        self.vocabulary
            .findings
            .iter()
            .chain(self.roadmap.iter())
            .chain(self.checked.iter().flat_map(|r| r.findings.iter()))
    }
}

/// Report summarizing standalone vocabulary validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabReport {
    pub file: String,
    pub status: Status,
    pub topics: usize,
    pub terms: usize,
    pub findings: Vec<Finding>,
}

/// Mirrors the IDE check notification scope line:
/// `Spec: <spec> (topics: a, b | none) — vocabulary: loaded | missing | invalid`
#[must_use]
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

/// Mirrors the IDE check notification status lines for each check suite.
#[must_use]
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
                let finding_word = if *findings == 1 {
                    "finding"
                } else {
                    "findings"
                };
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

/// Formats the final summary line:
/// `Checked <n> specs: <e> errors, <w> warnings, <i> info — fail-on <level>: pass|fail`
#[must_use]
pub fn summary_line(report: &GateReport) -> String {
    let n = report.specs.len();
    let e = report.counts.errors;
    let w = report.counts.warnings;
    let i = report.counts.info;
    let level = match report.fail_on {
        FailOn::Error => "error",
        FailOn::Warning => "warning",
    };
    let verdict = match report.exit {
        Exit::Clean => "pass",
        Exit::Findings | Exit::Failure => "fail",
    };
    format!("Checked {n} specs: {e} errors, {w} warnings, {i} info — fail-on {level}: {verdict}")
}

/// Renders the complete gate report as formatted text.
#[must_use]
pub fn text(report: &GateReport) -> String {
    let mut out = String::new();

    // 1. Vocabulary section
    let vocab_header = match report.vocabulary.status {
        Status::Loaded => format!(
            "Vocabulary: {VOCABULARY_PATH} — loaded ({} topics, {} terms)",
            report.vocabulary.vocabulary.topics().len(),
            report.vocabulary.vocabulary.len(),
        ),
        Status::Missing => format!("Vocabulary: {VOCABULARY_PATH} — missing"),
        Status::Invalid => format!("Vocabulary: {VOCABULARY_PATH} — invalid"),
    };
    out.push_str(&vocab_header);
    out.push('\n');
    if !report.vocabulary.findings.is_empty() {
        out.push_str(&findings_text(&report.vocabulary.findings));
    }
    out.push('\n');

    // 2. Roadmap section (only when findings exist)
    if !report.roadmap.is_empty() {
        let n = report.roadmap.len();
        let word = if n == 1 { "finding" } else { "findings" };
        out.push_str(&format!("Roadmap: {ROADMAP_PATH} — {n} {word}\n"));
        out.push_str(&findings_text(&report.roadmap));
        out.push('\n');
    }

    // 3. Per-spec sections
    for spec_res in &report.checked {
        out.push_str(&scope_line(&spec_res.spec, &spec_res.report));
        out.push('\n');
        for &kind in &CheckKind::ALL {
            out.push_str(&status_line(kind, &spec_res.report));
            out.push('\n');
        }
        if spec_res.findings.is_empty() {
            out.push_str("No findings.\n");
        } else {
            out.push_str(&findings_text(&spec_res.findings));
        }
        out.push('\n');
    }

    // 4. Summary line
    out.push_str(&summary_line(report));
    out.push('\n');

    out
}

#[derive(serde::Serialize)]
struct JsonReport<'a> {
    schema_version: &'static str,
    specs: &'a [String],
    checked: Vec<JsonSpec<'a>>,
    summary: JsonCounts,
    fail_on: FailOn,
    verdict: &'static str,
    findings: Vec<&'a Finding>,
}

#[derive(serde::Serialize)]
struct JsonSpec<'a> {
    spec: &'a str,
    topics: &'a [String],
    vocabulary: &'static str,
    checks: BTreeMap<&'static str, JsonStatus<'a>>,
}

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum JsonStatus<'a> {
    Ran { findings: usize },
    Skipped { reason: &'a str },
}

#[derive(serde::Serialize)]
struct JsonCounts {
    errors: usize,
    warnings: usize,
    info: usize,
}

/// Renders the complete gate report as pretty-printed JSON.
pub fn json(report: &GateReport) -> Result<String, Failure> {
    let mut checked_json = Vec::with_capacity(report.checked.len());
    for res in &report.checked {
        let vocab_str = match res.report.vocabulary {
            Status::Loaded => "loaded",
            Status::Missing => "missing",
            Status::Invalid => "invalid",
        };

        let mut checks = BTreeMap::new();
        for &kind in &CheckKind::ALL {
            let key = match kind {
                CheckKind::Ears => "ears",
                CheckKind::Terms => "terms",
                CheckKind::Contradictions => "contradictions",
            };
            let status = match res.report.status_of(kind) {
                CheckStatus::Ran { findings } => JsonStatus::Ran {
                    findings: *findings,
                },
                CheckStatus::Skipped { reason } => JsonStatus::Skipped {
                    reason: reason.as_str(),
                },
            };
            checks.insert(key, status);
        }

        checked_json.push(JsonSpec {
            spec: &res.spec,
            topics: &res.report.topics,
            vocabulary: vocab_str,
            checks,
        });
    }

    let verdict = match report.exit {
        Exit::Clean => "pass",
        Exit::Findings | Exit::Failure => "fail",
    };

    let all_findings: Vec<&Finding> = report.all_findings().collect();

    let json_rep = JsonReport {
        schema_version: REPORT_SCHEMA_VERSION,
        specs: &report.specs,
        checked: checked_json,
        summary: JsonCounts {
            errors: report.counts.errors,
            warnings: report.counts.warnings,
            info: report.counts.info,
        },
        fail_on: report.fail_on,
        verdict,
        findings: all_findings,
    };

    let mut text =
        serde_json::to_string_pretty(&json_rep).map_err(|e| Failure::new(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Renders standalone vocabulary validation as text.
#[must_use]
pub fn vocab_text(report: &VocabReport) -> String {
    let mut out = String::new();
    let header = match report.status {
        Status::Loaded => format!(
            "Vocabulary: {} — loaded ({} topics, {} terms)",
            report.file, report.topics, report.terms
        ),
        Status::Missing => format!("Vocabulary: {} — missing", report.file),
        Status::Invalid => format!("Vocabulary: {} — invalid", report.file),
    };
    out.push_str(&header);
    out.push('\n');
    out.push('\n');
    if report.findings.is_empty() {
        out.push_str("No findings.\n");
    } else {
        out.push_str(&findings_text(&report.findings));
    }
    out
}

#[derive(serde::Serialize)]
struct JsonVocabReport<'a> {
    schema_version: &'static str,
    file: &'a str,
    status: &'static str,
    topics: usize,
    terms: usize,
    findings: &'a [Finding],
}

/// Renders standalone vocabulary validation as JSON.
pub fn vocab_json(report: &VocabReport) -> Result<String, Failure> {
    let status_str = match report.status {
        Status::Loaded => "loaded",
        Status::Missing => "missing",
        Status::Invalid => "invalid",
    };
    let json_val = JsonVocabReport {
        schema_version: REPORT_SCHEMA_VERSION,
        file: &report.file,
        status: status_str,
        topics: report.topics,
        terms: report.terms,
        findings: &report.findings,
    };
    let mut text =
        serde_json::to_string_pretty(&json_val).map_err(|e| Failure::new(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use kiro_core::checks::{EarsSummary, SearchSummary, TermsSummary};
    use kiro_core::finding::codes;
    use kiro_core::vocab::Vocabulary;

    fn make_test_report(status: Status, topics: Vec<String>) -> CheckReport {
        let mut check_status = BTreeMap::new();
        check_status.insert(CheckKind::Ears, CheckStatus::Ran { findings: 0 });
        check_status.insert(CheckKind::Terms, CheckStatus::Ran { findings: 0 });
        check_status.insert(CheckKind::Contradictions, CheckStatus::Ran { findings: 0 });

        CheckReport {
            feature: "test-feature".to_string(),
            requirements_path: ".kiro/specs/test-feature/requirements.md".to_string(),
            topics,
            vocabulary: status,
            status: check_status,
            ears: Some(EarsSummary {
                criteria: 5,
                patterns: BTreeMap::new(),
                unclassified: 0,
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
                variables: 2,
                max_variables: 512,
                max_nodes: 100_000,
            }),
            findings: vec![],
        }
    }

    #[test]
    fn scope_line_literal_wording() {
        let rep_none = make_test_report(Status::Missing, vec![]);
        assert_eq!(
            scope_line("demo", &rep_none),
            "Spec: demo (topics: none) — vocabulary: missing"
        );

        let rep_topics =
            make_test_report(Status::Loaded, vec!["ops".to_string(), "pm".to_string()]);
        assert_eq!(
            scope_line("demo", &rep_topics),
            "Spec: demo (topics: ops, pm) — vocabulary: loaded"
        );

        let rep_invalid = make_test_report(Status::Invalid, vec!["audit".to_string()]);
        assert_eq!(
            scope_line("demo", &rep_invalid),
            "Spec: demo (topics: audit) — vocabulary: invalid"
        );
    }

    #[test]
    fn status_line_literal_wording() {
        let rep = make_test_report(Status::Loaded, vec![]);
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

        let mut skipped_rep = rep.clone();
        skipped_rep.status.insert(
            CheckKind::Terms,
            CheckStatus::Skipped {
                reason: "disabled by configuration".to_string(),
            },
        );
        assert_eq!(
            status_line(CheckKind::Terms, &skipped_rep),
            "Terms: skipped (disabled by configuration)"
        );
    }

    #[test]
    fn summary_line_literal_wording() {
        let report_clean = GateReport {
            specs: vec!["alpha".to_string(), "beta".to_string()],
            vocabulary: Loaded {
                status: Status::Loaded,
                vocabulary: Vocabulary::default(),
                findings: vec![],
            },
            roadmap: vec![],
            checked: vec![],
            fail_on: FailOn::Error,
            counts: Counts {
                errors: 0,
                warnings: 2,
                info: 1,
            },
            exit: Exit::Clean,
        };
        assert_eq!(
            summary_line(&report_clean),
            "Checked 2 specs: 0 errors, 2 warnings, 1 info — fail-on error: pass"
        );

        let report_fail = GateReport {
            fail_on: FailOn::Warning,
            exit: Exit::Findings,
            ..report_clean
        };
        assert_eq!(
            summary_line(&report_fail),
            "Checked 2 specs: 0 errors, 2 warnings, 1 info — fail-on warning: fail"
        );
    }

    #[test]
    fn text_and_json_determinism_and_layout() {
        let rep = make_test_report(Status::Loaded, vec!["ops".to_string()]);
        let gate_rep = GateReport {
            specs: vec!["demo".to_string()],
            vocabulary: Loaded {
                status: Status::Loaded,
                vocabulary: Vocabulary::default(),
                findings: vec![Finding::new(
                    codes::VOCAB_MISSING,
                    VOCABULARY_PATH,
                    "file missing",
                )],
            },
            roadmap: vec![Finding::new(
                codes::ROADMAP_DEP_UNKNOWN,
                ROADMAP_PATH,
                "unknown dependency",
            )],
            checked: vec![SpecResult {
                spec: "demo".to_string(),
                report: rep,
                findings: vec![Finding::new(
                    codes::EARS_NO_SHALL,
                    ".kiro/specs/demo/requirements.md",
                    "criterion missing shall",
                )],
            }],
            fail_on: FailOn::Error,
            counts: Counts {
                errors: 1,
                warnings: 2,
                info: 0,
            },
            exit: Exit::Findings,
        };

        // Text formatting
        let text1 = text(&gate_rep);
        let text2 = text(&gate_rep);
        assert_eq!(text1, text2);
        assert!(text1.contains("Vocabulary: .kiro/vocabulary.toml — loaded (0 topics, 0 terms)"));
        assert!(text1.contains("Roadmap: .kiro/steering/roadmap.md — 1 finding"));
        assert!(text1.contains("Spec: demo (topics: ops) — vocabulary: loaded"));
        assert!(
            text1.contains("Checked 1 specs: 1 errors, 2 warnings, 0 info — fail-on error: fail")
        );

        // JSON formatting
        let json1 = json(&gate_rep).unwrap();
        let json2 = json(&gate_rep).unwrap();
        assert_eq!(json1, json2);
        assert!(json1.contains(r#""schema_version": "1""#));
        assert!(json1.contains(r#""verdict": "fail""#));
        assert_eq!(gate_rep.all_findings().count(), 3);
    }

    #[test]
    fn vocab_text_and_json_reports() {
        let vocab_rep = VocabReport {
            file: "custom/vocab.toml".to_string(),
            status: Status::Loaded,
            topics: 4,
            terms: 12,
            findings: vec![],
        };
        let txt = vocab_text(&vocab_rep);
        assert!(txt.contains("Vocabulary: custom/vocab.toml — loaded (4 topics, 12 terms)"));
        assert!(txt.contains("No findings."));

        let js = vocab_json(&vocab_rep).unwrap();
        assert!(js.contains(r#""schema_version": "1""#));
        assert!(js.contains(r#""file": "custom/vocab.toml""#));
        assert!(js.contains(r#""status": "loaded""#));
    }
}
