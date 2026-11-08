//! EARS shape check and pattern classification (design component `EarsCheck`,
//! requirements 2.1, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10).
//!
//! Analyzes criteria for EARS compliance, offending modals, missing triggers,
//! unparsed subjects/responses, and classifies single-shall criteria into
//! ubiquitous, event-driven, state-driven, unwanted-behaviour, optional-feature,
//! or complex patterns.

use std::collections::BTreeMap;

use crate::checks::text::{Word, find_word_ci, hyphen_adjacent, mask_literals, words};
use crate::finding::{Finding, Severity, codes};
use crate::requirements::RequirementsDoc;

/// EARS requirement pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pattern {
    Ubiquitous,
    EventDriven,
    StateDriven,
    UnwantedBehaviour,
    OptionalFeature,
    Complex,
}

impl Pattern {
    /// Canonical lowercase kebab-case name of the pattern.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Pattern::Ubiquitous => "ubiquitous",
            Pattern::EventDriven => "event-driven",
            Pattern::StateDriven => "state-driven",
            Pattern::UnwantedBehaviour => "unwanted-behaviour",
            Pattern::OptionalFeature => "optional-feature",
            Pattern::Complex => "complex",
        }
    }
}

/// Leading condition keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    When,
    While,
    Where,
    If,
}

/// Structural analysis of a single acceptance criterion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    /// Whole-word occurrences of `shall` in the masked text.
    pub shall: Vec<Word>,
    /// Offending modals as written in document order, deduplicated.
    pub modals: Vec<String>,
    /// Leading condition keyword if criterion starts with one.
    pub leading: Option<Keyword>,
    /// Byte index of the last comma before the first `shall`.
    pub trigger_comma: Option<usize>,
    /// Whether criterion has `If ..., then the ... shall ...` form.
    pub then_form: bool,
    /// Subject from unmasked text (empty when missing).
    pub subject: String,
    /// Response from unmasked text (empty when missing).
    pub response: String,
    /// Classified pattern (Some only when exactly one `shall`).
    pub pattern: Option<Pattern>,
}

/// Structural summary of all criteria checked in a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EarsSummary {
    pub criteria: usize,
    pub patterns: BTreeMap<Pattern, usize>,
    pub unclassified: usize,
}

fn detect_leading_keyword(text: &str) -> Option<Keyword> {
    let trimmed = text.trim_start();
    if trimmed.len() >= 5
        && trimmed
            .get(..5)
            .is_some_and(|s| s.eq_ignore_ascii_case("when "))
    {
        Some(Keyword::When)
    } else if trimmed.len() >= 6
        && trimmed
            .get(..6)
            .is_some_and(|s| s.eq_ignore_ascii_case("while "))
    {
        Some(Keyword::While)
    } else if trimmed.len() >= 6
        && trimmed
            .get(..6)
            .is_some_and(|s| s.eq_ignore_ascii_case("where "))
    {
        Some(Keyword::Where)
    } else if trimmed.len() >= 3
        && trimmed
            .get(..3)
            .is_some_and(|s| s.eq_ignore_ascii_case("if "))
    {
        Some(Keyword::If)
    } else {
        None
    }
}

/// Analyzes a single criterion's text for EARS patterns, modals, and structure.
#[must_use]
pub fn analyze(text: &str) -> Analysis {
    let masked = mask_literals(text);
    let shall_words = find_word_ci(&masked, "shall");

    let mut modals = Vec::new();
    let modal_targets = ["should", "must", "may", "will"];
    for w in words(&masked) {
        let Some(word_slice) = masked.get(w.start..w.end) else {
            continue;
        };
        let matches_modal = modal_targets
            .iter()
            .any(|m| word_slice.eq_ignore_ascii_case(m));
        if matches_modal
            && !hyphen_adjacent(&masked, w)
            && let Some(orig) = text.get(w.start..w.end)
            && !modals.iter().any(|m: &String| m.eq_ignore_ascii_case(orig))
        {
            modals.push(orig.to_string());
        }
    }

    let leading = detect_leading_keyword(&masked);

    let mut trigger_comma = None;
    let mut then_form = false;
    let mut subject = String::new();
    let mut response = String::new();

    if let Some(first_shall) = shall_words.first().copied() {
        if leading.is_some()
            && let Some(before_shall) = masked.get(..first_shall.start)
            && let Some(comma_pos) = before_shall.rfind(',')
        {
            trigger_comma = Some(comma_pos);
            if leading == Some(Keyword::If)
                && let Some(between) = masked.get(comma_pos + 1..first_shall.start)
            {
                let trimmed = between.trim_start();
                if trimmed.len() >= 5
                    && trimmed
                        .get(..5)
                        .is_some_and(|s| s.eq_ignore_ascii_case("then "))
                {
                    then_form = true;
                }
            }
        }

        let start_subj = if leading.is_some() {
            trigger_comma.map_or(0, |c| c.saturating_add(1))
        } else {
            0
        };

        if let Some(raw_subj) = text.get(start_subj..first_shall.start) {
            let mut s = raw_subj.trim();
            if then_form {
                if let Some(rest) = s.strip_prefix("then ") {
                    s = rest.trim();
                } else if let Some(rest) = s.strip_prefix("Then ") {
                    s = rest.trim();
                }
            }
            subject = s
                .trim_matches(|c: char| c.is_whitespace() || c == ',')
                .to_string();
        }

        if let Some(raw_resp) = text.get(first_shall.end..) {
            response = raw_resp
                .trim()
                .trim_matches(|c: char| c.is_whitespace() || c == '.')
                .to_string();
        }
    }

    let pattern = if shall_words.len() == 1 {
        let first_shall = shall_words
            .first()
            .copied()
            .unwrap_or(Word { start: 0, end: 0 });
        let cond_end = if leading.is_some() {
            trigger_comma.unwrap_or(first_shall.start)
        } else {
            0
        };
        let cond_text = masked.get(..cond_end).unwrap_or("");

        // Count condition keywords in condition clause:
        // whole-word when|while|where|if whose preceding non-space character
        // is the start, ',', or whose preceding word is and/or
        let mut keyword_count = 0;
        let cond_words = words(cond_text);
        for (i, w) in cond_words.iter().enumerate() {
            let Some(w_slice) = cond_text.get(w.start..w.end) else {
                continue;
            };
            let is_kw = w_slice.eq_ignore_ascii_case("when")
                || w_slice.eq_ignore_ascii_case("while")
                || w_slice.eq_ignore_ascii_case("where")
                || w_slice.eq_ignore_ascii_case("if");
            if !is_kw {
                continue;
            }

            // Check predecessor:
            let before_str = cond_text.get(..w.start).unwrap_or("");
            let trimmed_before = before_str.trim_end();
            let preceded_by_start_or_comma =
                trimmed_before.is_empty() || trimmed_before.ends_with(',');

            let preceded_by_and_or = if i > 0 {
                cond_words
                    .get(i.saturating_sub(1))
                    .and_then(|prev_w| cond_text.get(prev_w.start..prev_w.end))
                    .is_some_and(|prev_str| {
                        prev_str.eq_ignore_ascii_case("and") || prev_str.eq_ignore_ascii_case("or")
                    })
            } else {
                false
            };

            if preceded_by_start_or_comma || preceded_by_and_or {
                keyword_count += 1;
            }
        }

        if keyword_count >= 2 {
            Some(Pattern::Complex)
        } else {
            match leading {
                Some(Keyword::When) => Some(Pattern::EventDriven),
                Some(Keyword::While) => Some(Pattern::StateDriven),
                Some(Keyword::If) => Some(Pattern::UnwantedBehaviour),
                Some(Keyword::Where) => Some(Pattern::OptionalFeature),
                None => Some(Pattern::Ubiquitous),
            }
        }
    } else {
        None
    };

    Analysis {
        shall: shall_words,
        modals,
        leading,
        trigger_comma,
        then_form,
        subject,
        response,
        pattern,
    }
}

/// Runs the EARS shape check over every acceptance criterion of a document.
#[must_use]
pub fn check(
    doc: &RequirementsDoc,
    feature: &str,
    report_patterns: bool,
) -> (Vec<Finding>, EarsSummary) {
    let mut findings = Vec::new();
    let mut summary = EarsSummary::default();

    for crit in doc.criteria() {
        summary.criteria += 1;
        let analysis = analyze(&crit.text);

        // 1. Shall count
        if analysis.shall.is_empty() {
            findings.push(
                Finding::new(codes::EARS_NO_SHALL, &doc.file, "criterion has no shall")
                    .at(crit.line)
                    .feature(feature),
            );
        } else if analysis.shall.len() >= 2 {
            findings.push(
                Finding::new(
                    codes::EARS_MULTI_SHALL,
                    &doc.file,
                    format!("found {}", analysis.shall.len()),
                )
                .at(crit.line)
                .feature(feature),
            );
        }

        // 2. Offending modals
        if !analysis.modals.is_empty() {
            findings.push(
                Finding::new(
                    codes::EARS_MODAL,
                    &doc.file,
                    format!("use shall, not {}", analysis.modals.join(", ")),
                )
                .at(crit.line)
                .feature(feature),
            );
        }

        // 3. Leading condition without comma before shall
        if analysis.leading.is_some()
            && !analysis.shall.is_empty()
            && analysis.trigger_comma.is_none()
        {
            findings.push(
                Finding::new(
                    codes::EARS_NO_TRIGGER,
                    &doc.file,
                    "condition keyword without comma before shall",
                )
                .at(crit.line)
                .feature(feature),
            );
        }

        // 4. Missing subject or response around single shall
        if analysis.shall.len() == 1 {
            if analysis.subject.is_empty() {
                findings.push(
                    Finding::new(codes::EARS_UNPARSED, &doc.file, "no subject before shall")
                        .at(crit.line)
                        .feature(feature),
                );
            } else if analysis.response.is_empty() {
                findings.push(
                    Finding::new(codes::EARS_UNPARSED, &doc.file, "no response after shall")
                        .at(crit.line)
                        .feature(feature),
                );
            }
        }

        // 5. Pattern classification
        if let Some(pat) = analysis.pattern {
            *summary.patterns.entry(pat).or_insert(0) += 1;
            if report_patterns {
                let f = Finding::new(
                    codes::EARS_PATTERN,
                    &doc.file,
                    format!("pattern: {}", pat.name()),
                )
                .severity(Severity::Info)
                .at(crit.line)
                .feature(feature);
                findings.push(f);
            }
        } else {
            summary.unclassified += 1;
        }
    }

    (findings, summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requirements::parse;

    #[test]
    fn single_shall_ubiquitous_pattern() {
        let text = "The controller shall record all telemetry readings.";
        let a = analyze(text);
        assert_eq!(a.shall.len(), 1);
        assert!(a.modals.is_empty());
        assert_eq!(a.leading, None);
        assert_eq!(a.subject, "The controller");
        assert_eq!(a.response, "record all telemetry readings");
        assert_eq!(a.pattern, Some(Pattern::Ubiquitous));
    }

    #[test]
    fn event_driven_pattern() {
        let text = "When the operator signals, the controller shall start.";
        let a = analyze(text);
        assert_eq!(a.leading, Some(Keyword::When));
        assert!(a.trigger_comma.is_some());
        assert_eq!(a.subject, "the controller");
        assert_eq!(a.response, "start");
        assert_eq!(a.pattern, Some(Pattern::EventDriven));
    }

    #[test]
    fn state_driven_pattern() {
        let text = "While mode is safe, the controller shall arm.";
        let a = analyze(text);
        assert_eq!(a.leading, Some(Keyword::While));
        assert_eq!(a.pattern, Some(Pattern::StateDriven));
    }

    #[test]
    fn optional_feature_pattern() {
        let text = "Where configured, the controller shall alert.";
        let a = analyze(text);
        assert_eq!(a.leading, Some(Keyword::Where));
        assert_eq!(a.pattern, Some(Pattern::OptionalFeature));
    }

    #[test]
    fn unwanted_behaviour_both_if_forms() {
        let t1 = "If depth is zero, the controller shall disarm.";
        let a1 = analyze(t1);
        assert_eq!(a1.leading, Some(Keyword::If));
        assert!(!a1.then_form);
        assert_eq!(a1.subject, "the controller");
        assert_eq!(a1.pattern, Some(Pattern::UnwantedBehaviour));

        let t2 = "If depth is zero, then the controller shall disarm.";
        let a2 = analyze(t2);
        assert_eq!(a2.leading, Some(Keyword::If));
        assert!(a2.then_form);
        assert_eq!(a2.subject, "the controller");
        assert_eq!(a2.pattern, Some(Pattern::UnwantedBehaviour));
    }

    #[test]
    fn complex_condition_pattern() {
        let text = "While mode is safe and when operator signals, the controller shall arm.";
        let a = analyze(text);
        assert_eq!(a.pattern, Some(Pattern::Complex));
    }

    #[test]
    fn ears_no_shall_and_multi_shall() {
        let doc = parse(
            "# Title\n\n### Requirement 1: Test\n\n#### Acceptance Criteria\n\n1. No modal here.\n2. One shall and second shall.\n",
            "test.md",
        );
        let (findings, summary) = check(&doc, "test", false);
        assert_eq!(summary.criteria, 2);
        assert_eq!(summary.unclassified, 2);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings.first().map(|f| f.code), Some(codes::EARS_NO_SHALL));
        assert_eq!(
            findings.get(1).map(|f| f.code),
            Some(codes::EARS_MULTI_SHALL)
        );
    }

    #[test]
    fn ears_modal_and_no_trigger() {
        let doc = parse(
            "# Title\n\n### Requirement 1: Test\n\n#### Acceptance Criteria\n\n1. The system must shall work.\n2. When ready the controller shall start.\n",
            "test.md",
        );
        let (findings, _) = check(&doc, "test", false);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings.first().map(|f| f.code), Some(codes::EARS_MODAL));
        assert_eq!(
            findings.get(1).map(|f| f.code),
            Some(codes::EARS_NO_TRIGGER)
        );
    }

    #[test]
    fn ears_unparsed_subject_and_response() {
        let doc = parse(
            "# Title\n\n### Requirement 1: Test\n\n#### Acceptance Criteria\n\n1. When ready, shall start.\n2. When ready, the controller shall.\n",
            "test.md",
        );
        let (findings, _) = check(&doc, "test", false);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings.first().map(|f| f.code), Some(codes::EARS_UNPARSED));
        assert_eq!(
            findings.first().map(|f| f.message.as_str()),
            Some("no subject before shall")
        );
        assert_eq!(findings.get(1).map(|f| f.code), Some(codes::EARS_UNPARSED));
        assert_eq!(
            findings.get(1).map(|f| f.message.as_str()),
            Some("no response after shall")
        );
    }

    #[test]
    fn v1_false_positives_produce_no_finding() {
        let doc = parse(
            "# Title\n\n### Requirement 1: Regressions\n\n#### Acceptance Criteria\n\n1. The system shall display \"you must enter a password\".\n2. The tool shall run the must-gather script.\n3. The validator shall verify the tree's root hash.\n4. The system shall send the email, which was queued, to the user.\n",
            "test.md",
        );
        let (findings, summary) = check(&doc, "test", false);
        assert!(findings.is_empty());
        assert_eq!(summary.criteria, 4);
        assert_eq!(summary.unclassified, 0);
    }

    #[test]
    fn report_patterns_flag() {
        let doc = parse(
            "# Title\n\n### Requirement 1: Regressions\n\n#### Acceptance Criteria\n\n1. The system shall log.\n",
            "test.md",
        );
        let (findings_off, _) = check(&doc, "test", false);
        assert!(findings_off.is_empty());

        let (findings_on, _) = check(&doc, "test", true);
        assert_eq!(findings_on.len(), 1);
        assert_eq!(
            findings_on.first().map(|f| f.code),
            Some(codes::EARS_PATTERN)
        );
        assert_eq!(
            findings_on.first().map(|f| f.message.as_str()),
            Some("pattern: ubiquitous")
        );
    }
}
