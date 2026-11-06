//! `spec.json` into a typed `SpecMeta` (design component `SpecMetaParser`,
//! requirements 2.1–2.7): phase classification with case-insensitive trimmed
//! comparison, per-field degradation of wrong-typed values (a non-boolean
//! approval flag is treated as absent), the required-document table that
//! drives `SPEC_DOC_MISSING` severity, `phase_label` shared by plugin-core
//! and spec-visualization, and successor-name extraction from the notes for
//! the superseded → successor rule. Parsing goes through
//! `serde_json::Value`, never a derive struct, so unknown fields are ignored
//! and wrong-typed fields degrade one field at a time.

use crate::finding::{Finding, codes};

/// The classified phase of a spec (design interface).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    Initialized,
    RequirementsGenerated,
    DesignGenerated,
    TasksGenerated,
    Superseded,
    Reserved,
    Unknown,
    #[default]
    Missing,
}

/// One approval gate (generated / approved), absent defaults to false.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Approval {
    pub generated: bool,
    pub approved: bool,
}

/// The three approval gates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Approvals {
    pub requirements: Approval,
    pub design: Approval,
    pub tasks: Approval,
}

/// The typed content of one `spec.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecMeta {
    pub feature_name: Option<String>,
    pub language: Option<String>,
    pub phase: Phase,
    pub phase_raw: Option<String>,
    pub approvals: Approvals,
    pub ready_for_implementation: bool,
    pub topics: Vec<String>,
    pub notes: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// The documents a phase implies must exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doc {
    SpecJson,
    Requirements,
    Design,
    Tasks,
}

/// Parses `spec.json` text. Never fails: invalid or non-object JSON yields
/// `SpecMeta::default()` (phase `Missing`) plus one `SPEC_JSON_INVALID`.
pub fn parse(text: &str, file: &str) -> (SpecMeta, Vec<Finding>) {
    let mut findings = Vec::new();
    let value = match serde_json::from_str::<serde_json::Value>(text) {
        Ok(v) => v,
        Err(_) => {
            findings.push(Finding::new(
                codes::SPEC_JSON_INVALID,
                file,
                "spec.json is not valid JSON",
            ));
            return (SpecMeta::default(), findings);
        }
    };
    let Some(obj) = value.as_object() else {
        findings.push(Finding::new(
            codes::SPEC_JSON_INVALID,
            file,
            "spec.json is not a JSON object",
        ));
        return (SpecMeta::default(), findings);
    };

    let feature_name = obj
        .get("feature_name")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let language = obj
        .get("language")
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let mut phase = Phase::Missing;
    let mut phase_raw: Option<String> = None;
    match obj.get("phase") {
        None => findings.push(Finding::new(
            codes::SPEC_PHASE_MISSING,
            file,
            "phase is absent",
        )),
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                phase_raw = Some(trimmed.to_owned());
                findings.push(Finding::new(
                    codes::SPEC_PHASE_MISSING,
                    file,
                    "phase is empty",
                ));
            } else {
                phase = classify_phase(trimmed);
                phase_raw = Some(trimmed.to_owned());
                if phase == Phase::Unknown {
                    findings.push(Finding::new(
                        codes::SPEC_PHASE_UNKNOWN,
                        file,
                        format!("phase {trimmed:?} is not a recognised value"),
                    ));
                }
            }
        }
        Some(other) => {
            // A present but non-string phase is unknown, raw value kept.
            phase = Phase::Unknown;
            phase_raw = Some(other.to_string());
            findings.push(Finding::new(
                codes::SPEC_PHASE_UNKNOWN,
                file,
                format!("phase {other} is not a recognised value"),
            ));
        }
    }

    let approvals = Approvals {
        requirements: approval_of(obj, "requirements"),
        design: approval_of(obj, "design"),
        tasks: approval_of(obj, "tasks"),
    };
    let ready_for_implementation = obj
        .get("ready_for_implementation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let topics = match obj.get("topics") {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    let notes = obj
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    let created_at = obj
        .get("created_at")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let updated_at = obj
        .get("updated_at")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let meta = SpecMeta {
        feature_name,
        language,
        phase,
        phase_raw,
        approvals,
        ready_for_implementation,
        topics,
        notes,
        created_at,
        updated_at,
    };
    (meta, findings)
}

/// Case-insensitive, trimmed phase classification.
fn classify_phase(raw: &str) -> Phase {
    match raw.to_ascii_lowercase().as_str() {
        "initialized" => Phase::Initialized,
        "requirements-generated" => Phase::RequirementsGenerated,
        "design-generated" => Phase::DesignGenerated,
        "tasks-generated" => Phase::TasksGenerated,
        "superseded" => Phase::Superseded,
        "reserved" => Phase::Reserved,
        _ => Phase::Unknown,
    }
}

/// One approval gate, wrong-typed values treated as absent.
fn approval_of(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Approval {
    let Some(gate) = obj
        .get("approvals")
        .and_then(|v| v.as_object())
        .and_then(|a| a.get(key))
    else {
        return Approval::default();
    };
    let flag = |name: &str| {
        gate.as_object()
            .and_then(|g| g.get(name))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };
    Approval {
        generated: flag("generated"),
        approved: flag("approved"),
    }
}

/// Trimmed `phase_raw` when present and non-empty, else `"missing"`. Shared
/// by plugin-core (`ws`) and spec-visualization (`viz`).
pub fn phase_label(meta: Option<&SpecMeta>) -> String {
    meta.and_then(|m| m.phase_raw.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("missing")
        .to_owned()
}

/// Which documents the phase implies must exist (drives `SPEC_DOC_MISSING`
/// severity; `SpecJson` itself missing is always `warning`).
pub fn required_docs(phase: Phase) -> &'static [Doc] {
    match phase {
        Phase::Initialized | Phase::RequirementsGenerated => &[Doc::Requirements],
        Phase::DesignGenerated => &[Doc::Requirements, Doc::Design],
        Phase::TasksGenerated => &[Doc::Requirements, Doc::Design, Doc::Tasks],
        Phase::Superseded | Phase::Reserved | Phase::Unknown | Phase::Missing => &[],
    }
}

/// Known names occurring as whole words (name characters: ASCII alphanumerics,
/// `-`, `_`) in `meta.notes`, checked longest-first; the result is in that
/// order and deduplicated. Lives here (not in `resolve`) so that
/// `Workspace::load` can emit `SPEC_SUPERSEDED_DANGLING` without importing
/// upward.
pub fn successors(meta: &SpecMeta, known: &[String]) -> Vec<String> {
    let mut order: Vec<&String> = known.iter().collect();
    order.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let mut out: Vec<String> = Vec::new();
    for name in order {
        if contains_name(&meta.notes, name) && !out.contains(name) {
            out.push(name.clone());
        }
    }
    out
}

/// Whole-word containment with name-character boundaries.
fn contains_name(notes: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(idx) = notes[from..].find(name) {
        let at = from + idx;
        let after = at + name.len();
        let bounded_before = notes[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !is_name_char(c));
        let bounded_after = notes[after..]
            .chars()
            .next()
            .is_none_or(|c| !is_name_char(c));
        if bounded_before && bounded_after {
            return true;
        }
        from = at + 1;
    }
    false
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::finding::{Severity, codes};
    use crate::spec::{Doc, Phase, parse, phase_label, required_docs, successors};

    const F: &str = ".kiro/specs/demo/spec.json";

    const CANONICAL: &str = r#"{
        "feature_name": "demo",
        "language": "en",
        "phase": "tasks-generated",
        "approvals": {
            "requirements": { "generated": true, "approved": true },
            "design": { "generated": true, "approved": false },
            "tasks": { "generated": false, "approved": false }
        },
        "ready_for_implementation": false,
        "topics": ["sdd", "demo"],
        "notes": "Superseded by demo-two, not by demo.",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-02T00:00:00Z"
    }"#;

    #[test]
    fn well_formed_metadata_is_exposed_unmodified() {
        let (meta, findings) = parse(CANONICAL, F);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(meta.feature_name.as_deref(), Some("demo"));
        assert_eq!(meta.language.as_deref(), Some("en"));
        assert_eq!(meta.phase, Phase::TasksGenerated);
        assert_eq!(meta.phase_raw.as_deref(), Some("tasks-generated"));
        assert!(meta.approvals.requirements.generated);
        assert!(meta.approvals.requirements.approved);
        assert!(meta.approvals.design.generated);
        assert!(!meta.approvals.design.approved);
        assert!(!meta.approvals.tasks.generated);
        assert!(!meta.ready_for_implementation);
        assert_eq!(meta.topics, ["sdd", "demo"]);
        assert_eq!(meta.notes, "Superseded by demo-two, not by demo.");
        assert_eq!(meta.created_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(meta.updated_at.as_deref(), Some("2026-01-02T00:00:00Z"));
    }

    #[test]
    fn phases_are_recognised_case_insensitively_after_trimming() {
        for (raw, phase) in [
            ("initialized", Phase::Initialized),
            (" Requirements-Generated ", Phase::RequirementsGenerated),
            ("DESIGN-GENERATED", Phase::DesignGenerated),
            ("tasks-generated", Phase::TasksGenerated),
            ("superseded", Phase::Superseded),
            ("reserved", Phase::Reserved),
        ] {
            let text = format!(r#"{{"phase": "{raw}"}}"#);
            let (meta, findings) = parse(&text, F);
            assert_eq!(meta.phase, phase, "raw {raw:?}");
            assert!(findings.is_empty(), "raw {raw:?}: {findings:?}");
        }
    }

    #[test]
    fn unknown_phase_warns_and_keeps_the_raw_string() {
        let (meta, findings) = parse(r#"{"phase": "frozen"}"#, F);
        assert_eq!(meta.phase, Phase::Unknown);
        assert_eq!(meta.phase_raw.as_deref(), Some("frozen"));
        let f = findings
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("expected finding"));
        assert_eq!(
            (f.code, f.severity),
            (codes::SPEC_PHASE_UNKNOWN, Severity::Warning)
        );
        assert_eq!(f.line, None);
    }

    #[test]
    fn missing_or_empty_phase_warns() {
        let (meta, findings) = parse("{}", F);
        assert_eq!(meta.phase, Phase::Missing);
        assert_eq!(meta.phase_raw, None);
        assert!(
            matches!(findings.first(), Some(f) if f.code == codes::SPEC_PHASE_MISSING
            && f.severity == Severity::Warning)
        );
        let (meta, findings) = parse(r#"{"phase": "   "}"#, F);
        assert_eq!(meta.phase, Phase::Missing);
        assert!(matches!(findings.first(), Some(f) if f.code == codes::SPEC_PHASE_MISSING));
    }

    #[test]
    fn invalid_or_non_object_json_errors_with_default_meta() {
        for text in ["{ nope", "[]", "\"a string\"", "42"] {
            let (meta, findings) = parse(text, F);
            assert_eq!(meta.phase, Phase::Missing, "{text}");
            assert!(
                matches!(findings.first(), Some(f) if f.code == codes::SPEC_JSON_INVALID
                && f.severity == Severity::Error),
                "{text}"
            );
        }
    }

    #[test]
    fn absent_or_wrong_typed_optionals_default_without_findings() {
        // Absent phase reports SPEC_PHASE_MISSING (2.4); the other absent
        // optionals stay silent (2.6).
        let (meta, findings) = parse("{}", F);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings.first().map(|f| f.code),
            Some(codes::SPEC_PHASE_MISSING)
        );
        assert!(!meta.approvals.requirements.generated);
        assert!(meta.topics.is_empty());
        assert!(meta.notes.is_empty());
        assert_eq!(meta.feature_name, None);
        // Wrong-typed values degrade field by field; only the phase finding
        // (still absent here) appears.
        let text = r#"{
            "feature_name": 7,
            "ready_for_implementation": "yes",
            "approvals": { "requirements": { "generated": "true" } },
            "topics": ["a", 3, "b"],
            "notes": 4,
            "created_at": false
        }"#;
        let (meta, findings) = parse(text, F);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(
            findings.first().map(|f| f.code),
            Some(codes::SPEC_PHASE_MISSING)
        );
        assert_eq!(meta.feature_name, None);
        assert!(!meta.ready_for_implementation);
        assert!(!meta.approvals.requirements.generated);
        assert_eq!(meta.topics, ["a", "b"]);
        assert_eq!(meta.notes, "");
        assert_eq!(meta.created_at, None);
    }

    #[test]
    fn unknown_fields_are_ignored_quietly() {
        let text = r#"{"phase": "initialized", "surprise": {"deep": [1]}, "another": 1}"#;
        let (meta, findings) = parse(text, F);
        assert!(findings.is_empty());
        assert_eq!(meta.phase, Phase::Initialized);
    }

    #[test]
    fn phase_label_variants() {
        assert_eq!(phase_label(None), "missing");
        let (meta, _) = parse(r#"{"phase": "tasks-generated"}"#, F);
        assert_eq!(phase_label(Some(&meta)), "tasks-generated");
        let (meta, _) = parse(r#"{"phase": "  padded  "}"#, F);
        assert_eq!(phase_label(Some(&meta)), "padded");
        let (meta, _) = parse(r#"{"phase": ""}"#, F);
        assert_eq!(phase_label(Some(&meta)), "missing");
        let (meta, _) = parse("{}", F);
        assert_eq!(phase_label(Some(&meta)), "missing");
    }

    #[test]
    fn required_docs_table_per_phase() {
        assert_eq!(required_docs(Phase::Initialized), [Doc::Requirements]);
        assert_eq!(
            required_docs(Phase::RequirementsGenerated),
            [Doc::Requirements]
        );
        assert_eq!(
            required_docs(Phase::DesignGenerated),
            [Doc::Requirements, Doc::Design]
        );
        assert_eq!(
            required_docs(Phase::TasksGenerated),
            [Doc::Requirements, Doc::Design, Doc::Tasks]
        );
        assert!(required_docs(Phase::Superseded).is_empty());
        assert!(required_docs(Phase::Reserved).is_empty());
        assert!(required_docs(Phase::Unknown).is_empty());
        assert!(required_docs(Phase::Missing).is_empty());
    }

    #[test]
    fn successors_whole_word_longest_first_deduplicated() {
        let (meta, _) = parse(CANONICAL, F);
        let known = vec![
            "demo".to_owned(),
            "demo-two".to_owned(),
            "other".to_owned(),
            "demo-two".to_owned(),
        ];
        let succ = successors(&meta, &known);
        assert_eq!(
            succ,
            ["demo-two", "demo"],
            "longest first, deduplicated, whole words only"
        );
        // Substrings are not whole words.
        let known = vec!["em".to_owned(), "demo".to_owned()];
        assert_eq!(successors(&meta, &known), ["demo"]);
        // Notes that only contain the name as part of a longer word do not match.
        let (meta, _) = parse(r#"{"notes": "demos everywhere"}"#, F);
        let known = vec!["demo".to_owned()];
        assert!(successors(&meta, &known).is_empty());
        // The whole notes text being exactly the name matches.
        let (meta, _) = parse(r#"{"notes": "demo"}"#, F);
        assert_eq!(successors(&meta, &known), ["demo"]);
    }
}
