//! The one finding shape rendered by both frontends (design component
//! `Finding`): severity levels, the finding value, builder helpers whose
//! default severity comes from the catalogue, the complete code catalogue of
//! this crate, and the stable ordering shared by the plugin and the CLI.
//!
//! Invariants: `code` is always one of `CODES`; `file` is project-relative
//! with `/` separators (the builder normalises `\` so Windows frontends
//! cannot smuggle a backslash in).

use serde::Serialize;

/// Severity of a finding. `Ord` is part of the contract: frontends derive
/// their worst-level mapping from it (`Error < Warning < Info`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// One finding, in the fixed field order used by the CLI's JSON report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub code: &'static str,
    pub feature: Option<String>,
    /// Project-relative path, always with `/` separators.
    pub file: String,
    /// 1-based line, when the finding points at a place in the file.
    pub line: Option<usize>,
    pub message: String,
}

impl Finding {
    /// Creates a finding whose severity defaults from the catalogue entry for
    /// `code` (an unknown code — a programming error — falls back to
    /// `Warning`, never a panic) and whose `file` is separator-normalised.
    pub fn new(code: &'static str, file: impl Into<String>, message: impl Into<String>) -> Self {
        let severity = CODES
            .iter()
            .find(|entry| entry.code == code)
            .map_or(Severity::Warning, |entry| entry.severity);
        Finding {
            severity,
            code,
            feature: None,
            file: file.into().replace('\\', "/"),
            line: None,
            message: message.into(),
        }
    }

    /// Attaches the 1-based line the finding points at.
    pub fn at(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Attaches the feature (spec) name the finding belongs to.
    pub fn feature(mut self, name: impl Into<String>) -> Self {
        self.feature = Some(name.into());
        self
    }

    /// Overrides the catalogue's default severity.
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }
}

/// One catalogue entry: a stable code, its default severity, one line of
/// description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeInfo {
    pub code: &'static str,
    pub severity: Severity,
    pub description: &'static str,
}

/// Every finding code this crate emits (Supporting References table).
pub const CODES: &[CodeInfo] = &[
    CodeInfo {
        code: "SPEC_DOC_MISSING",
        severity: Severity::Info,
        description: "A feature document could not be read",
    },
    CodeInfo {
        code: "SPEC_JSON_INVALID",
        severity: Severity::Error,
        description: "spec.json is not a JSON object",
    },
    CodeInfo {
        code: "SPEC_PHASE_UNKNOWN",
        severity: Severity::Warning,
        description: "phase is not a recognised value",
    },
    CodeInfo {
        code: "SPEC_PHASE_MISSING",
        severity: Severity::Warning,
        description: "phase absent or empty",
    },
    CodeInfo {
        code: "SPEC_SUPERSEDED_DANGLING",
        severity: Severity::Warning,
        description: "Superseded feature names no known successor",
    },
    CodeInfo {
        code: "SPEC_SUPERSEDED_CYCLE",
        severity: Severity::Warning,
        description: "Successor chain revisits a feature",
    },
    CodeInfo {
        code: "MD_FENCE_UNCLOSED",
        severity: Severity::Info,
        description: "A fenced block runs to the end of the document",
    },
    CodeInfo {
        code: "REQ_NO_CRITERIA",
        severity: Severity::Warning,
        description: "Baseline requirement without criteria",
    },
    CodeInfo {
        code: "REQ_DUPLICATE_ID",
        severity: Severity::Warning,
        description: "Requirement or criterion id defined twice",
    },
    CodeInfo {
        code: "REQ_NON_NUMERIC_ID",
        severity: Severity::Warning,
        description: "### Requirement heading without a number",
    },
    CodeInfo {
        code: "TASK_HIERARCHY",
        severity: Severity::Warning,
        description: "Depth > 2 or sub-task without its major",
    },
    CodeInfo {
        code: "TASK_REQ_REF_INVALID",
        severity: Severity::Warning,
        description: "_Requirements:_ entry not N, N.M or A<K>.M",
    },
    CodeInfo {
        code: "TASK_DUPLICATE_ID",
        severity: Severity::Warning,
        description: "Task id defined twice",
    },
    CodeInfo {
        code: "TASK_BOUNDARY_UNKNOWN",
        severity: Severity::Warning,
        description: "_Boundary:_ names no design component",
    },
    CodeInfo {
        code: "TASK_REQ_UNKNOWN",
        severity: Severity::Warning,
        description: "_Requirements:_ names an undefined criterion",
    },
    CodeInfo {
        code: "TASK_DEPENDS_UNKNOWN",
        severity: Severity::Warning,
        description: "_Depends:_ names an undefined task",
    },
    CodeInfo {
        code: "DESIGN_COMPONENT_NO_FILES",
        severity: Severity::Info,
        description: "Component with no mapped file",
    },
    CodeInfo {
        code: "DESIGN_FILE_UNASSIGNED",
        severity: Severity::Info,
        description: "Plan file with no component",
    },
    CodeInfo {
        code: "DESIGN_NO_FILE_PLAN",
        severity: Severity::Warning,
        description: "No File Structure Plan section",
    },
    CodeInfo {
        code: "ROADMAP_DEP_UNKNOWN",
        severity: Severity::Warning,
        description: "Dependency names no roadmap entry",
    },
    CodeInfo {
        code: "ROADMAP_NO_SPECS_SECTION",
        severity: Severity::Info,
        description: "No `## Specs (dependency order)` section",
    },
    CodeInfo {
        code: "VOCAB_MISSING",
        severity: Severity::Info,
        description: ".kiro/vocabulary.toml could not be read",
    },
    CodeInfo {
        code: "VOCAB_INVALID",
        severity: Severity::Error,
        description: "Not valid TOML, wrong-typed field or invalid id (line of the first problem)",
    },
    CodeInfo {
        code: "VOCAB_SCHEMA_UNSUPPORTED",
        severity: Severity::Warning,
        description: "schema_version present and not \"2\"",
    },
    CodeInfo {
        code: "TERM_NO_DEFINITION",
        severity: Severity::Warning,
        description: "Empty definition",
    },
    CodeInfo {
        code: "TERM_KIND_INVALID",
        severity: Severity::Warning,
        description: "Missing or unknown kind, enum without values, int with min > max; kind treated as entity",
    },
    CodeInfo {
        code: "TERM_DUPLICATE_LABEL",
        severity: Severity::Error,
        description: "Two terms of one topic share a label or alt label",
    },
    CodeInfo {
        code: "TERM_BROADER_UNKNOWN",
        severity: Severity::Warning,
        description: "broader names no term; relation ignored",
    },
    CodeInfo {
        code: "TERM_BROADER_CROSS_TOPIC",
        severity: Severity::Error,
        description: "broader names a term of another topic; relation ignored",
    },
    CodeInfo {
        code: "TERM_BROADER_CYCLE",
        severity: Severity::Error,
        description: "Term is transitively broader than itself",
    },
    CodeInfo {
        code: "TERM_DEPRECATED_NO_REPLACEMENT",
        severity: Severity::Warning,
        description: "Deprecated without replaced_by",
    },
    CodeInfo {
        code: "TERM_REPLACED_BY_UNKNOWN",
        severity: Severity::Warning,
        description: "replaced_by names no term",
    },
    CodeInfo {
        code: "TERM_REPLACED_BY_CROSS_TOPIC",
        severity: Severity::Warning,
        description: "replaced_by names a term of another topic",
    },
    CodeInfo {
        code: "TOPIC_EMPTY",
        severity: Severity::Info,
        description: "Declared topic without terms",
    },
    CodeInfo {
        code: "TOPICS_UNDECLARED",
        severity: Severity::Info,
        description: "Spec declares no topics; no term resolution",
    },
    CodeInfo {
        code: "TOPIC_UNKNOWN",
        severity: Severity::Warning,
        description: "Spec declares a topic the vocabulary does not have",
    },
    CodeInfo {
        code: "VOCAB_LABEL_CROSS_TOPIC",
        severity: Severity::Info,
        description: "Same normalised label in two topics — a lint only; resolution never crosses topics (2.15, amendment 2026-11-04)",
    },
    CodeInfo {
        code: "EARS_NO_SHALL",
        severity: Severity::Error,
        description: "Criterion has no `shall`",
    },
    CodeInfo {
        code: "EARS_MULTI_SHALL",
        severity: Severity::Warning,
        description: "Criterion has more than one `shall` (\"found N\")",
    },
    CodeInfo {
        code: "EARS_MODAL",
        severity: Severity::Warning,
        description: "Competing modal outside literals and hyphenated compounds (\"use `shall`, not …\")",
    },
    CodeInfo {
        code: "EARS_NO_TRIGGER",
        severity: Severity::Warning,
        description: "Condition keyword without a comma before `shall`",
    },
    CodeInfo {
        code: "EARS_UNPARSED",
        severity: Severity::Warning,
        description: "Missing subject or response around the single `shall`",
    },
    CodeInfo {
        code: "EARS_PATTERN",
        severity: Severity::Info,
        description: "Recognised pattern (\"pattern: <name>\"), only when enabled",
    },
    CodeInfo {
        code: "TERM_UNDECLARED",
        severity: Severity::Error,
        description: "Declared term unknown in the declared topics",
    },
    CodeInfo {
        code: "TERM_TOPIC_MISMATCH",
        severity: Severity::Error,
        description: "Declared term exists only in undeclared topics; never a match",
    },
    CodeInfo {
        code: "TERM_AMBIGUOUS",
        severity: Severity::Warning,
        description: "Declared term matches several terms; use the id",
    },
    CodeInfo {
        code: "TERM_DEPRECATED",
        severity: Severity::Warning,
        description: "Declared term is deprecated",
    },
    CodeInfo {
        code: "TERM_SUGGESTED",
        severity: Severity::Info,
        description: "A visible label occurs in criteria but is not declared",
    },
    CodeInfo {
        code: "CHECK_UNSCOPED",
        severity: Severity::Warning,
        description: "Annotations present but the spec has no topics or the vocabulary is unavailable",
    },
    CodeInfo {
        code: "GUARD_UNPARSED",
        severity: Severity::Error,
        description: "Guard/effect text outside the grammar, term-vs-term comparison, duplicate or misplaced annotation",
    },
    CodeInfo {
        code: "GUARD_TERM_UNDECLARED",
        severity: Severity::Error,
        description: "Guard/effect references a term not declared in `_Terms:_`",
    },
    CodeInfo {
        code: "GUARD_KIND_MISMATCH",
        severity: Severity::Error,
        description: "Term kind incompatible with the literal or operator",
    },
    CodeInfo {
        code: "GUARD_UNSATISFIABLE",
        severity: Severity::Warning,
        description: "A guard holds in no state",
    },
    CodeInfo {
        code: "CONFLICT_REF_UNKNOWN",
        severity: Severity::Warning,
        description: "`_Conflicts:_` names no criterion",
    },
    CodeInfo {
        code: "CONTRADICTION",
        severity: Severity::Error,
        description: "Two incompatible criteria can both apply; witness given",
    },
    CodeInfo {
        code: "SEARCH_REFUSED",
        severity: Severity::Warning,
        description: "A cap was exceeded; the listed pairs were not checked",
    },
    CodeInfo {
        code: "CHECK_COVERAGE",
        severity: Severity::Info,
        description: "\"{criteria} criteria, {annotated} annotated, {pairs} pairs considered, {contradictions} contradictions, {refused} refused\" (line: None) — distinguishes \"checked\" from \"not checked\" whenever the term or contradiction check ran (amendment 2026-11-04)",
    },
    CodeInfo {
        code: "IMPORT_INPUT_INVALID",
        severity: Severity::Error,
        description: "Importer input could not be parsed in the chosen format",
    },
    CodeInfo {
        code: "IMPORT_TERM_SKIPPED",
        severity: Severity::Warning,
        description: "Importer skipped a source term that yields no id or has no name",
    },
    CodeInfo {
        code: "IMPORT_DUPLICATE_ID",
        severity: Severity::Warning,
        description: "Two source terms derive the same id; first wins",
    },
    CodeInfo {
        code: "IMPORT_MERGE_CONFLICT",
        severity: Severity::Warning,
        description: "Imported term collides with an existing id or label; existing kept",
    },
];

/// String constants for every catalogue code, so callers never hand-write one.
pub mod codes {
    pub const SPEC_DOC_MISSING: &str = "SPEC_DOC_MISSING";
    pub const SPEC_JSON_INVALID: &str = "SPEC_JSON_INVALID";
    pub const SPEC_PHASE_UNKNOWN: &str = "SPEC_PHASE_UNKNOWN";
    pub const SPEC_PHASE_MISSING: &str = "SPEC_PHASE_MISSING";
    pub const SPEC_SUPERSEDED_DANGLING: &str = "SPEC_SUPERSEDED_DANGLING";
    pub const SPEC_SUPERSEDED_CYCLE: &str = "SPEC_SUPERSEDED_CYCLE";
    pub const MD_FENCE_UNCLOSED: &str = "MD_FENCE_UNCLOSED";
    pub const REQ_NO_CRITERIA: &str = "REQ_NO_CRITERIA";
    pub const REQ_DUPLICATE_ID: &str = "REQ_DUPLICATE_ID";
    pub const REQ_NON_NUMERIC_ID: &str = "REQ_NON_NUMERIC_ID";
    pub const TASK_HIERARCHY: &str = "TASK_HIERARCHY";
    pub const TASK_REQ_REF_INVALID: &str = "TASK_REQ_REF_INVALID";
    pub const TASK_DUPLICATE_ID: &str = "TASK_DUPLICATE_ID";
    pub const TASK_BOUNDARY_UNKNOWN: &str = "TASK_BOUNDARY_UNKNOWN";
    pub const TASK_REQ_UNKNOWN: &str = "TASK_REQ_UNKNOWN";
    pub const TASK_DEPENDS_UNKNOWN: &str = "TASK_DEPENDS_UNKNOWN";
    pub const DESIGN_COMPONENT_NO_FILES: &str = "DESIGN_COMPONENT_NO_FILES";
    pub const DESIGN_FILE_UNASSIGNED: &str = "DESIGN_FILE_UNASSIGNED";
    pub const DESIGN_NO_FILE_PLAN: &str = "DESIGN_NO_FILE_PLAN";
    pub const ROADMAP_DEP_UNKNOWN: &str = "ROADMAP_DEP_UNKNOWN";
    pub const ROADMAP_NO_SPECS_SECTION: &str = "ROADMAP_NO_SPECS_SECTION";
    pub const VOCAB_MISSING: &str = "VOCAB_MISSING";
    pub const VOCAB_INVALID: &str = "VOCAB_INVALID";
    pub const VOCAB_SCHEMA_UNSUPPORTED: &str = "VOCAB_SCHEMA_UNSUPPORTED";
    pub const TERM_NO_DEFINITION: &str = "TERM_NO_DEFINITION";
    pub const TERM_KIND_INVALID: &str = "TERM_KIND_INVALID";
    pub const TERM_DUPLICATE_LABEL: &str = "TERM_DUPLICATE_LABEL";
    pub const TERM_BROADER_UNKNOWN: &str = "TERM_BROADER_UNKNOWN";
    pub const TERM_BROADER_CROSS_TOPIC: &str = "TERM_BROADER_CROSS_TOPIC";
    pub const TERM_BROADER_CYCLE: &str = "TERM_BROADER_CYCLE";
    pub const TERM_DEPRECATED_NO_REPLACEMENT: &str = "TERM_DEPRECATED_NO_REPLACEMENT";
    pub const TERM_REPLACED_BY_UNKNOWN: &str = "TERM_REPLACED_BY_UNKNOWN";
    pub const TERM_REPLACED_BY_CROSS_TOPIC: &str = "TERM_REPLACED_BY_CROSS_TOPIC";
    pub const TOPIC_EMPTY: &str = "TOPIC_EMPTY";
    pub const TOPICS_UNDECLARED: &str = "TOPICS_UNDECLARED";
    pub const TOPIC_UNKNOWN: &str = "TOPIC_UNKNOWN";
    pub const VOCAB_LABEL_CROSS_TOPIC: &str = "VOCAB_LABEL_CROSS_TOPIC";
    pub const EARS_NO_SHALL: &str = "EARS_NO_SHALL";
    pub const EARS_MULTI_SHALL: &str = "EARS_MULTI_SHALL";
    pub const EARS_MODAL: &str = "EARS_MODAL";
    pub const EARS_NO_TRIGGER: &str = "EARS_NO_TRIGGER";
    pub const EARS_UNPARSED: &str = "EARS_UNPARSED";
    pub const EARS_PATTERN: &str = "EARS_PATTERN";
    pub const TERM_UNDECLARED: &str = "TERM_UNDECLARED";
    pub const TERM_TOPIC_MISMATCH: &str = "TERM_TOPIC_MISMATCH";
    pub const TERM_AMBIGUOUS: &str = "TERM_AMBIGUOUS";
    pub const TERM_DEPRECATED: &str = "TERM_DEPRECATED";
    pub const TERM_SUGGESTED: &str = "TERM_SUGGESTED";
    pub const CHECK_UNSCOPED: &str = "CHECK_UNSCOPED";
    pub const GUARD_UNPARSED: &str = "GUARD_UNPARSED";
    pub const GUARD_TERM_UNDECLARED: &str = "GUARD_TERM_UNDECLARED";
    pub const GUARD_KIND_MISMATCH: &str = "GUARD_KIND_MISMATCH";
    pub const GUARD_UNSATISFIABLE: &str = "GUARD_UNSATISFIABLE";
    pub const CONFLICT_REF_UNKNOWN: &str = "CONFLICT_REF_UNKNOWN";
    pub const CONTRADICTION: &str = "CONTRADICTION";
    pub const SEARCH_REFUSED: &str = "SEARCH_REFUSED";
    pub const CHECK_COVERAGE: &str = "CHECK_COVERAGE";
    pub const IMPORT_INPUT_INVALID: &str = "IMPORT_INPUT_INVALID";
    pub const IMPORT_TERM_SKIPPED: &str = "IMPORT_TERM_SKIPPED";
    pub const IMPORT_DUPLICATE_ID: &str = "IMPORT_DUPLICATE_ID";
    pub const IMPORT_MERGE_CONFLICT: &str = "IMPORT_MERGE_CONFLICT";
}

/// The twenty check finding codes introduced by the constraints engine.
pub const CHECK_CODES: &[&str] = &[
    codes::EARS_NO_SHALL,
    codes::EARS_MULTI_SHALL,
    codes::EARS_MODAL,
    codes::EARS_NO_TRIGGER,
    codes::EARS_UNPARSED,
    codes::EARS_PATTERN,
    codes::TERM_UNDECLARED,
    codes::TERM_TOPIC_MISMATCH,
    codes::TERM_AMBIGUOUS,
    codes::TERM_DEPRECATED,
    codes::TERM_SUGGESTED,
    codes::CHECK_UNSCOPED,
    codes::GUARD_UNPARSED,
    codes::GUARD_TERM_UNDECLARED,
    codes::GUARD_KIND_MISMATCH,
    codes::GUARD_UNSATISFIABLE,
    codes::CONFLICT_REF_UNKNOWN,
    codes::CONTRADICTION,
    codes::SEARCH_REFUSED,
    codes::CHECK_COVERAGE,
];

/// Orders by `(file, line with absent lines first, code, message)`; stable,
/// so identical input always yields identical output. Takes a mutable slice
/// (callers pass `&mut Vec<Finding>`; the design's signature, adapted for the
/// crate's denied `ptr_arg` lint).
pub fn sort_findings(findings: &mut [Finding]) {
    fn line_cmp(a: Option<usize>, b: Option<usize>) -> std::cmp::Ordering {
        match (a, b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => x.cmp(&y),
        }
    }
    findings.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| line_cmp(a.line, b.line))
            .then_with(|| a.code.cmp(b.code))
            .then_with(|| a.message.cmp(&b.message))
    });
}

#[cfg(test)]
mod tests {
    use crate::finding::{CHECK_CODES, CODES, CodeInfo, Finding, Severity, codes, sort_findings};

    #[test]
    fn severity_ord_error_before_warning_before_info() {
        // Frontends map notification levels from Ord: Error < Warning < Info.
        assert!(Severity::Error < Severity::Warning);
        assert!(Severity::Warning < Severity::Info);
    }

    #[test]
    fn catalogue_has_61_entries_and_every_codes_constant() {
        assert_eq!(CODES.len(), 61);
        assert_eq!(CHECK_CODES.len(), 20);
        let all = [
            codes::SPEC_DOC_MISSING,
            codes::SPEC_JSON_INVALID,
            codes::SPEC_PHASE_UNKNOWN,
            codes::SPEC_PHASE_MISSING,
            codes::SPEC_SUPERSEDED_DANGLING,
            codes::SPEC_SUPERSEDED_CYCLE,
            codes::MD_FENCE_UNCLOSED,
            codes::REQ_NO_CRITERIA,
            codes::REQ_DUPLICATE_ID,
            codes::REQ_NON_NUMERIC_ID,
            codes::TASK_HIERARCHY,
            codes::TASK_REQ_REF_INVALID,
            codes::TASK_DUPLICATE_ID,
            codes::TASK_BOUNDARY_UNKNOWN,
            codes::TASK_REQ_UNKNOWN,
            codes::TASK_DEPENDS_UNKNOWN,
            codes::DESIGN_COMPONENT_NO_FILES,
            codes::DESIGN_FILE_UNASSIGNED,
            codes::DESIGN_NO_FILE_PLAN,
            codes::ROADMAP_DEP_UNKNOWN,
            codes::ROADMAP_NO_SPECS_SECTION,
            codes::VOCAB_MISSING,
            codes::VOCAB_INVALID,
            codes::VOCAB_SCHEMA_UNSUPPORTED,
            codes::TERM_NO_DEFINITION,
            codes::TERM_KIND_INVALID,
            codes::TERM_DUPLICATE_LABEL,
            codes::TERM_BROADER_UNKNOWN,
            codes::TERM_BROADER_CROSS_TOPIC,
            codes::TERM_BROADER_CYCLE,
            codes::TERM_DEPRECATED_NO_REPLACEMENT,
            codes::TERM_REPLACED_BY_UNKNOWN,
            codes::TERM_REPLACED_BY_CROSS_TOPIC,
            codes::TOPIC_EMPTY,
            codes::TOPICS_UNDECLARED,
            codes::TOPIC_UNKNOWN,
            codes::VOCAB_LABEL_CROSS_TOPIC,
            codes::EARS_NO_SHALL,
            codes::EARS_MULTI_SHALL,
            codes::EARS_MODAL,
            codes::EARS_NO_TRIGGER,
            codes::EARS_UNPARSED,
            codes::EARS_PATTERN,
            codes::TERM_UNDECLARED,
            codes::TERM_TOPIC_MISMATCH,
            codes::TERM_AMBIGUOUS,
            codes::TERM_DEPRECATED,
            codes::TERM_SUGGESTED,
            codes::CHECK_UNSCOPED,
            codes::GUARD_UNPARSED,
            codes::GUARD_TERM_UNDECLARED,
            codes::GUARD_KIND_MISMATCH,
            codes::GUARD_UNSATISFIABLE,
            codes::CONFLICT_REF_UNKNOWN,
            codes::CONTRADICTION,
            codes::SEARCH_REFUSED,
            codes::CHECK_COVERAGE,
            codes::IMPORT_INPUT_INVALID,
            codes::IMPORT_TERM_SKIPPED,
            codes::IMPORT_DUPLICATE_ID,
            codes::IMPORT_MERGE_CONFLICT,
        ];
        assert_eq!(all.len(), 61);
        for c in all {
            assert_eq!(
                CODES.iter().filter(|e| e.code == c).count(),
                1,
                "code {c} must appear exactly once in CODES"
            );
        }
    }

    #[test]
    fn catalogue_entries_are_documented_and_screaming_snake() {
        for CodeInfo {
            code,
            severity: _,
            description,
        } in CODES.iter()
        {
            assert!(!description.is_empty(), "{code} needs a description");
            assert!(
                code.chars()
                    .enumerate()
                    .all(|(i, ch)| ch.is_ascii_uppercase()
                        || ch.is_ascii_digit()
                        || (ch == '_' && i > 0)),
                "{code} must be SCREAMING_SNAKE"
            );
        }
    }

    #[test]
    fn builder_takes_default_severity_from_catalogue() {
        let f = Finding::new(
            codes::SPEC_JSON_INVALID,
            ".kiro/specs/demo/spec.json",
            "boom",
        );
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.code, "SPEC_JSON_INVALID");
        assert_eq!(f.feature, None);
        assert_eq!(f.line, None);
        let f = Finding::new(codes::MD_FENCE_UNCLOSED, "a.md", "unclosed")
            .at(7)
            .feature("demo");
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(f.line, Some(7));
        assert_eq!(f.feature.as_deref(), Some("demo"));
        let f = f.severity(Severity::Warning);
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn builder_normalises_file_separators() {
        let f = Finding::new(
            codes::SPEC_DOC_MISSING,
            ".kiro\\specs\\demo\\spec.json",
            "m",
        );
        assert_eq!(f.file, ".kiro/specs/demo/spec.json");
    }

    #[test]
    fn sort_orders_file_then_line_none_first_then_code_then_message() {
        let mut v = vec![
            Finding::new(codes::REQ_NO_CRITERIA, "b.md", "second file"),
            Finding::new(codes::REQ_NO_CRITERIA, "a.md", "z-message"),
            Finding::new(codes::MD_FENCE_UNCLOSED, "a.md", "m").at(2),
            Finding::new(codes::REQ_NO_CRITERIA, "a.md", "a-message"),
            Finding::new(codes::SPEC_DOC_MISSING, "a.md", "no line").at(3),
            Finding::new(codes::MD_FENCE_UNCLOSED, "a.md", "m").at(9),
        ];
        sort_findings(&mut v);
        let ordered: Vec<(String, Option<usize>, &str, &str)> = v
            .iter()
            .map(|f| (f.file.clone(), f.line, f.code, f.message.as_str()))
            .collect();
        assert_eq!(
            ordered,
            vec![
                ("a.md".into(), None, "REQ_NO_CRITERIA", "a-message"),
                ("a.md".into(), None, "REQ_NO_CRITERIA", "z-message"),
                ("a.md".into(), Some(2), "MD_FENCE_UNCLOSED", "m"),
                ("a.md".into(), Some(3), "SPEC_DOC_MISSING", "no line"),
                ("a.md".into(), Some(9), "MD_FENCE_UNCLOSED", "m"),
                ("b.md".into(), None, "REQ_NO_CRITERIA", "second file"),
            ][..]
        );
    }

    #[test]
    fn sort_is_stable_for_equal_keys() {
        // The key includes the message, so fully equal keys are constructed by
        // using the same code and message twice; stability keeps insertion order.
        let mut v = vec![
            Finding::new(codes::REQ_NO_CRITERIA, "a.md", "same").feature("first"),
            Finding::new(codes::REQ_NO_CRITERIA, "a.md", "same").feature("second"),
        ];
        sort_findings(&mut v);
        let features: Vec<Option<String>> = v.iter().map(|f| f.feature.clone()).collect();
        assert_eq!(features, vec![Some("first".into()), Some("second".into())]);
    }

    #[test]
    fn json_shape_has_fixed_field_order() {
        let f = Finding::new(
            codes::SPEC_JSON_INVALID,
            ".kiro/specs/demo/spec.json",
            "not an object",
        )
        .at(3)
        .feature("demo");
        let json = serde_json::to_string(&f).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"severity":"error","code":"SPEC_JSON_INVALID","feature":"demo","file":".kiro/specs/demo/spec.json","line":3,"message":"not an object"}"#
        );
        // Absent optionals serialize as null (fixed shape for cli-gate).
        let g = Finding::new(codes::MD_FENCE_UNCLOSED, "a.md", "m");
        let json = serde_json::to_string(&g).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"severity":"info","code":"MD_FENCE_UNCLOSED","feature":null,"file":"a.md","line":null,"message":"m"}"#
        );
    }

    #[test]
    fn severity_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Severity::Warning).unwrap_or_default(),
            r#""warning""#
        );
    }
}
