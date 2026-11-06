//! `requirements.md` into requirements and `N.M` criteria (design component
//! `RequirementsParser`, requirements 4.1–4.12) under the grammars seen in
//! real cc-sdd specs: G1 canonical (`### Requirement N: title` + `####
//! Acceptance Criteria`), G1 dotted (self-numbered `N.M` anywhere in the
//! block), G2 R-style (`### R<N> — title` with `- R<N>.<M>` bullets) and G3
//! amendment (`### Amendment A<K> — title (extends Requirement X / Y)`).

use crate::finding::{Finding, codes};
use crate::md::{Annotation, Heading, LineKind, scan};

/// Baseline requirement or amendment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReqKind {
    Baseline,
    Amendment,
}

/// One acceptance criterion `N.M` (or `A<K>.M`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criterion {
    pub id: String,
    pub number: String,
    pub text: String,
    pub line: usize,
    pub end_line: usize,
    pub annotations: Vec<Annotation>,
}

/// One requirement block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// `"1"` or `"A1"`.
    pub id: String,
    pub kind: ReqKind,
    pub title: String,
    pub objective: Option<String>,
    /// Amendment only: the requirement numbers after `extends`.
    pub extends: Vec<String>,
    pub line: usize,
    pub end_line: usize,
    pub annotations: Vec<Annotation>,
    pub criteria: Vec<Criterion>,
}

/// The parsed requirements document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementsDoc {
    pub file: String,
    pub outline: Vec<Heading>,
    pub fences: Vec<crate::md::Fence>,
    pub requirements: Vec<Requirement>,
    pub findings: Vec<Finding>,
}

impl RequirementsDoc {
    /// Every criterion of every requirement, in document order.
    pub fn criteria(&self) -> impl Iterator<Item = &Criterion> {
        self.requirements.iter().flat_map(|r| r.criteria.iter())
    }

    pub fn criterion(&self, id: &str) -> Option<&Criterion> {
        self.criteria().find(|c| c.id == id)
    }

    pub fn requirement(&self, id: &str) -> Option<&Requirement> {
        self.requirements.iter().find(|r| r.id == id)
    }
}

/// Parses `requirements.md` text into the model. Never panics.
pub fn parse(text: &str, file: &str) -> RequirementsDoc {
    let scanned = scan(text, file);
    let mut doc = RequirementsDoc {
        file: file.to_owned(),
        outline: scanned.outline,
        fences: scanned.fences,
        requirements: Vec::new(),
        findings: scanned.findings,
    };
    // Open block state.
    let mut current: Option<Requirement> = None;
    let mut gate = false;
    let mut open_criterion: Option<Criterion> = None;
    let mut skip_block = false;
    let mut seen_req_ids: Vec<String> = Vec::new();
    let mut seen_crit_ids: Vec<String> = Vec::new();

    let close_criterion = |crit: &mut Option<Criterion>, req: &mut Option<Requirement>| {
        if let Some(c) = crit.take()
            && let Some(r) = req.as_mut()
        {
            r.criteria.push(c);
        }
    };
    let close_block =
        |req: &mut Option<Requirement>, doc: &mut RequirementsDoc, end_line: usize| {
            if let Some(mut r) = req.take() {
                r.end_line = end_line;
                if r.kind == ReqKind::Baseline && r.criteria.is_empty() {
                    doc.findings.push(
                        Finding::new(
                            codes::REQ_NO_CRITERIA,
                            &doc.file,
                            format!("requirement {} has no acceptance criteria", r.id),
                        )
                        .at(r.line),
                    );
                }
                doc.requirements.push(r);
            }
        };

    for line in &scanned.lines {
        let line_no = line.line;
        // Headings drive the block state machine.
        if let LineKind::Heading { level: _, text: h } = line.kind {
            let h = h.trim();
            if is_acceptance_criteria(h) {
                gate = true;
                close_criterion(&mut open_criterion, &mut current);
                continue;
            }
            // Any other heading closes the current block first.
            if let Some(req) = current.as_mut() {
                req.end_line = line_no.saturating_sub(1);
                if let Some(c) = open_criterion.as_mut() {
                    c.end_line = line_no.saturating_sub(1);
                }
                close_criterion(&mut open_criterion, &mut current);
                close_block(&mut current, &mut doc, line_no.saturating_sub(1));
            }
            gate = false;
            skip_block = false;

            match open_heading(h) {
                Some(Opened::Baseline { id, title }) => {
                    if seen_req_ids.contains(&id) {
                        doc.findings.push(
                            Finding::new(
                                codes::REQ_DUPLICATE_ID,
                                file,
                                format!("requirement {id} is defined twice; the first definition is kept"),
                            )
                            .at(line_no),
                        );
                        skip_block = true;
                    } else {
                        seen_req_ids.push(id.clone());
                        current = Some(Requirement {
                            id,
                            kind: ReqKind::Baseline,
                            title,
                            objective: None,
                            extends: Vec::new(),
                            line: line_no,
                            end_line: line_no,
                            annotations: Vec::new(),
                            criteria: Vec::new(),
                        });
                    }
                }
                Some(Opened::Amendment { id, title, extends }) => {
                    if seen_req_ids.contains(&id) {
                        doc.findings.push(
                            Finding::new(
                                codes::REQ_DUPLICATE_ID,
                                file,
                                format!(
                                    "amendment {id} is defined twice; the first definition is kept"
                                ),
                            )
                            .at(line_no),
                        );
                        skip_block = true;
                    } else {
                        seen_req_ids.push(id.clone());
                        current = Some(Requirement {
                            id,
                            kind: ReqKind::Amendment,
                            title,
                            objective: None,
                            extends,
                            line: line_no,
                            end_line: line_no,
                            annotations: Vec::new(),
                            criteria: Vec::new(),
                        });
                    }
                }
                Some(Opened::NonNumeric) => {
                    doc.findings.push(
                        Finding::new(
                            codes::REQ_NON_NUMERIC_ID,
                            file,
                            "requirement heading has no numeric identifier",
                        )
                        .at(line_no),
                    );
                    skip_block = true;
                }
                None => {
                    // A different heading: nothing opens here.
                }
            }
            continue;
        }
        if skip_block {
            continue;
        }

        match line.kind {
            // Headings are fully handled above.
            LineKind::Heading { .. } => {}
            LineKind::Blank => close_criterion(&mut open_criterion, &mut current),
            LineKind::FenceOpen { .. } | LineKind::Fenced | LineKind::FenceClose => {
                // Fenced content is invisible to the parser.
            }
            LineKind::Annotation { .. } => {
                let annotation = Annotation {
                    key: annotation_key(line),
                    value: annotation_value(line),
                    line: line_no,
                };
                if open_criterion.is_some()
                    && line.indent > 0
                    && let Some(c) = open_criterion.as_mut()
                {
                    c.annotations.push(annotation);
                } else if let Some(req) = current.as_mut() {
                    req.annotations.push(annotation);
                }
            }
            LineKind::ListItem { .. } => {
                if let Some((id, number, text)) = item_criterion(line, current.as_ref(), gate) {
                    if seen_crit_ids.contains(&id) {
                        doc.findings.push(
                            Finding::new(
                                codes::REQ_DUPLICATE_ID,
                                file,
                                format!(
                                    "criterion {id} is defined twice; the first definition is kept"
                                ),
                            )
                            .at(line_no),
                        );
                        close_criterion(&mut open_criterion, &mut current);
                    } else {
                        seen_crit_ids.push(id.clone());
                        close_criterion(&mut open_criterion, &mut current);
                        open_criterion = Some(Criterion {
                            id,
                            number,
                            text: text.to_owned(),
                            line: line_no,
                            end_line: line_no,
                            annotations: Vec::new(),
                        });
                    }
                } else if let Some(c) = open_criterion.as_mut() {
                    join_text(c, item_text(line), line_no);
                }
            }
            LineKind::Text | LineKind::TableRow { .. } => {
                let raw = line.raw.trim();
                if let Some(rest) = objective_of(raw) {
                    if let Some(req) = current.as_mut() {
                        req.objective = Some(rest.to_owned());
                    }
                    continue;
                }
                // Amendment self-numbered criterion form: `A<K>.M text`.
                if let Some(req) = current.as_ref()
                    && req.kind == ReqKind::Amendment
                    && let Some((id, number, text)) = amendment_text_criterion(raw, &req.id)
                {
                    if seen_crit_ids.contains(&id) {
                        doc.findings.push(
                            Finding::new(
                                codes::REQ_DUPLICATE_ID,
                                file,
                                format!(
                                    "criterion {id} is defined twice; the first definition is kept"
                                ),
                            )
                            .at(line_no),
                        );
                        close_criterion(&mut open_criterion, &mut current);
                    } else {
                        seen_crit_ids.push(id.clone());
                        close_criterion(&mut open_criterion, &mut current);
                        open_criterion = Some(Criterion {
                            id,
                            number,
                            text: text.to_owned(),
                            line: line_no,
                            end_line: line_no,
                            annotations: Vec::new(),
                        });
                    }
                    continue;
                }
                if let Some(c) = open_criterion.as_mut() {
                    join_text(c, raw, line_no);
                }
            }
        }
    }

    // Close the trailing block at the last line.
    let end_line = scanned.lines.last().map(|l| l.line).unwrap_or(0);
    if let Some(req) = current.as_mut() {
        req.end_line = end_line;
    }
    if let Some(c) = open_criterion.as_mut() {
        c.end_line = end_line;
    }
    close_criterion(&mut open_criterion, &mut current);
    close_block(&mut current, &mut doc, end_line);
    doc
}

/// A heading analysis result.
enum Opened {
    Baseline {
        id: String,
        title: String,
    },
    Amendment {
        id: String,
        title: String,
        extends: Vec<String>,
    },
    NonNumeric,
}

/// `### Requirement N[: title]` with sep `:` `-` `–` `—` (case-insensitive).
fn open_heading(h: &str) -> Option<Opened> {
    let lower = h.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("requirement") {
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None; // e.g. `Requirements Document` is a section heading
        }
        let rest_orig = h
            .get("requirement".len()..)
            .unwrap_or_default()
            .trim_start();
        let digits: String = rest_orig
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return Some(Opened::NonNumeric);
        }
        let after = rest_orig
            .get(digits.len()..)
            .unwrap_or_default()
            .trim_start();
        let title = split_sep(after).unwrap_or_default().to_owned();
        return Some(Opened::Baseline { id: digits, title });
    }
    if let Some(rest) = lower.strip_prefix("amendment") {
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }
        let rest_orig = h.get("amendment".len()..).unwrap_or_default().trim_start();
        let rest_orig = match rest_orig.chars().next() {
            Some('a' | 'A') => rest_orig.get(1..).unwrap_or_default(),
            _ => return Some(Opened::NonNumeric),
        };
        let digits: String = rest_orig
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return Some(Opened::NonNumeric);
        }
        let after = rest_orig
            .get(digits.len()..)
            .unwrap_or_default()
            .trim_start();
        let body = split_sep(after).unwrap_or_default();
        let (title, extends) = split_extends(body);
        return Some(Opened::Amendment {
            id: format!("A{digits}"),
            title: title.to_owned(),
            extends,
        });
    }
    // `R<N>` followed by a dash or colon separator.
    if let Some(rest) = h.strip_prefix('R') {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let after = rest.get(digits.len()..).unwrap_or_default();
            if let Some(title) = split_sep(after.trim_start()) {
                return Some(Opened::Baseline {
                    id: digits,
                    title: title.to_owned(),
                });
            }
        }
    }
    None
}

/// Splits one leading separator (`:`, `-`, `–`, `—`, after optional spaces)
/// and returns the title; `None` when no separator is present.
fn split_sep(after: &str) -> Option<&str> {
    let trimmed = after.trim_start();
    let sep_len = match trimmed.chars().next() {
        Some(':') | Some('-') | Some('–') | Some('—') => {
            trimmed.chars().next().map_or(0, char::len_utf8)
        }
        _ => return None,
    };
    Some(trimmed.get(sep_len..).unwrap_or_default().trim())
}

/// Splits a trailing `(extends …)` parenthetical from the title and collects
/// the requirement numbers it names.
fn split_extends(body: &str) -> (&str, Vec<String>) {
    let Some(open) = body.rfind('(') else {
        return (body.trim(), Vec::new());
    };
    let Some(close) = body.get(open..).and_then(|t| t.find(')')) else {
        return (body.trim(), Vec::new());
    };
    let paren = body.get(open + 1..open + close).unwrap_or_default().trim();
    let title = body.get(..open).unwrap_or_default().trim();
    let lower = paren.to_ascii_lowercase();
    if !lower.starts_with("extends") {
        return (body.trim(), Vec::new());
    }
    let mut extends = Vec::new();
    let mut rest = paren;
    while let Some(pos) = rest.to_ascii_lowercase().find("requirement ") {
        let after = rest.get(pos + "requirement ".len()..).unwrap_or_default();
        let id: String = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if !id.is_empty() && !extends.contains(&id) {
            extends.push(id);
        }
        rest = after;
    }
    (title, extends)
}

fn is_acceptance_criteria(h: &str) -> bool {
    h.eq_ignore_ascii_case("acceptance criteria")
}

fn objective_of(raw: &str) -> Option<&str> {
    raw.strip_prefix("**Objective:**").map(str::trim)
}

/// Extracts the criterion triple from a list item, honouring the gate and the
/// enclosing requirement's kind.
fn item_criterion<'a>(
    line: &crate::md::ScannedLine<'a>,
    req: Option<&Requirement>,
    gate: bool,
) -> Option<(String, String, &'a str)> {
    let req = req?;
    let LineKind::ListItem {
        bullet: _,
        checkbox: _,
        marker,
        text,
    } = line.kind
    else {
        return None;
    };
    let text_trimmed = text.trim_start_matches(['r', 'R']);
    // Dotted self-numbered form of the enclosing requirement, as a marker
    // (`3.1 …`) or in the item text with an optional leading `r`/`R`.
    let dotted: Option<(String, &str)> = marker
        .map(|m| m.label)
        .filter(|l| is_dotted_of(l, &req.id))
        .map(|label| (label.to_owned(), text))
        .or_else(|| dotted_prefix(text_trimmed).filter(|(id, _)| is_dotted_of(id, &req.id)));
    if let Some((id, rest)) = dotted {
        return Some((id.clone(), dotted_number(&id).to_owned(), rest));
    }
    // Bare `M.` / `M)` items need the gate, or an amendment block.
    if let Some(m) = marker
        && m.label.chars().all(|c| c.is_ascii_digit())
        && m.closed_by.is_some()
        && (gate || req.kind == ReqKind::Amendment)
    {
        let id = format!("{}.{}", req.id, m.label);
        return Some((id, m.label.to_owned(), text));
    }
    None
}

/// `A<K>.M text` on a plain text line inside an amendment block.
fn amendment_text_criterion<'a>(raw: &'a str, req_id: &str) -> Option<(String, String, &'a str)> {
    let rest = raw.strip_prefix(req_id)?;
    let rest = rest.strip_prefix('.')?;
    let number: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if number.is_empty() {
        return None;
    }
    let after = rest.get(number.len()..).unwrap_or_default();
    let text = after.strip_prefix(' ').unwrap_or(after);
    Some((format!("{req_id}.{number}"), number, text))
}

fn dotted_prefix(text: &str) -> Option<(String, &str)> {
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = text.get(digits.len()..).unwrap_or_default();
    if let Some(sub) = rest.strip_prefix('.') {
        let sub_digits: String = sub.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !sub_digits.is_empty() {
            let after = sub.get(sub_digits.len()..).unwrap_or_default();
            let tail = after.strip_prefix(' ').unwrap_or(after);
            return Some((format!("{digits}.{sub_digits}"), tail));
        }
    }
    None
}

fn is_dotted_of(label: &str, req_id: &str) -> bool {
    match label.find('.') {
        Some(pos) => label.get(..pos).unwrap_or_default() == req_id && label.len() > pos + 1,
        None => false,
    }
}

fn dotted_number(id: &str) -> &str {
    id.rfind('.')
        .and_then(|pos| id.get(pos + 1..))
        .unwrap_or_default()
}

fn join_text(c: &mut Criterion, text: &str, line_no: usize) {
    if text.is_empty() {
        return;
    }
    if c.text.is_empty() {
        c.text = text.to_owned();
    } else {
        c.text.push(' ');
        c.text.push_str(text);
    }
    c.end_line = line_no;
}

fn annotation_key(line: &crate::md::ScannedLine<'_>) -> String {
    match line.kind {
        LineKind::Annotation { key, value: _ } => key.to_owned(),
        _ => String::new(),
    }
}

fn item_text<'a>(line: &crate::md::ScannedLine<'a>) -> &'a str {
    match line.kind {
        LineKind::ListItem { text, .. } => text,
        _ => "",
    }
}

fn annotation_value(line: &crate::md::ScannedLine<'_>) -> String {
    match line.kind {
        LineKind::Annotation { key: _, value } => value.to_owned(),
        _ => String::new(),
    }
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::finding::{Severity, codes};
    use crate::requirements::{ReqKind, parse};

    const F: &str = ".kiro/specs/demo/requirements.md";

    // Excerpt of a real canonical spec (mmhmem cross-agent-memory-server),
    // trimmed.
    const CANONICAL: &str = "# Requirements Document\n\n## Requirements\n\n### Requirement 1: Remote MCP protocol surface and authentication\n**Objective:** As a harness operator, I want every supported agent harness to connect to one central memory endpoint, so that no per-harness client hooks are needed.\n\n#### Acceptance Criteria\n1. The mmhmem server shall expose its tools via a remote MCP endpoint.\n2. When a client connects without a valid Bearer token, the mmhmem server shall reject the request\n   with an authentication error.\n3. When a client presents a valid Bearer token, the mmhmem server shall accept MCP connections.\n\n### Requirement 2: Cross-agent live session bus\n\n**Objective:** As an engineer switching harnesses, I want the new agent to inherit the state.\n\n#### Acceptance Criteria\n\n1. When an agent calls the tool for a project, the mmhmem server shall return the stored state.\n";

    #[test]
    fn canonical_grammar_objective_criteria_and_continuation() {
        let doc = parse(CANONICAL, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.requirements.len(), 2);
        let r1 = doc
            .requirement("1")
            .cloned()
            .unwrap_or_else(|| panic!("req 1"));
        assert_eq!(r1.kind, ReqKind::Baseline);
        assert_eq!(r1.title, "Remote MCP protocol surface and authentication");
        assert_eq!(r1.line, 5);
        assert_eq!(
            r1.objective.as_deref(),
            Some(
                "As a harness operator, I want every supported agent harness to connect to one central memory endpoint, so that no per-harness client hooks are needed."
            )
        );
        assert_eq!(r1.criteria.len(), 3);
        let c2 = r1
            .criteria
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("criterion 1.2"));
        assert_eq!((c2.id.as_str(), c2.number.as_str()), ("1.2", "2"));
        assert_eq!(c2.line, 10);
        assert_eq!(c2.end_line, 11, "continuation advances end_line");
        assert_eq!(
            c2.text,
            "When a client connects without a valid Bearer token, the mmhmem server shall reject the request with an authentication error."
        );
        assert_eq!(doc.criterion("2.1").map(|c| c.id.as_str()), Some("2.1"));
        assert_eq!(doc.criteria().count(), 4);
    }

    #[test]
    fn r_style_headings_with_em_dash_and_r_bullets() {
        // Real shape from facdrone-results-and-service.
        let text = "### R1 — Fresh, fully independent decision path\n- R1.1 The decision path reads only fresh inputs.\n- R1.2 The reference never drifts.\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        let r1 = doc
            .requirement("1")
            .cloned()
            .unwrap_or_else(|| panic!("req 1"));
        assert_eq!(r1.title, "Fresh, fully independent decision path");
        assert_eq!(r1.criteria.len(), 2);
        assert_eq!(r1.criteria.first().map(|c| c.id.as_str()), Some("1.1"));
    }

    #[test]
    fn amendment_grammar_and_extends_extraction() {
        // Real shapes from facdrone-alpha-port.
        let text = "### Requirement 4: Base controls\n\n#### Acceptance Criteria\n1. Base criterion.\n\n### Amendment A1: Intraday enforcement (extends Requirement 4)\n1. Amendment bare item becomes A1.1.\n\n### Amendment A2: Intrabar cooldown (extends Requirement 4 / Requirement 12.3)\nA2.1 Self-numbered amendment criterion.\n\n### Amendment A3: Regime routing (concrete use of Requirement 7 seam)\n1. Only numbered items count here.\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        let base = doc
            .requirement("4")
            .cloned()
            .unwrap_or_else(|| panic!("req 4"));
        assert_eq!(base.criteria.len(), 1);
        let a1 = doc
            .requirement("A1")
            .cloned()
            .unwrap_or_else(|| panic!("A1"));
        assert_eq!(a1.kind, ReqKind::Amendment);
        assert_eq!(a1.extends, ["4"]);
        assert_eq!(a1.criteria.first().map(|c| c.id.as_str()), Some("A1.1"));
        let a2 = doc
            .requirement("A2")
            .cloned()
            .unwrap_or_else(|| panic!("A2"));
        assert_eq!(a2.extends, ["4", "12.3"]);
        assert_eq!(a2.criteria.first().map(|c| c.id.as_str()), Some("A2.1"));
        let a3 = doc
            .requirement("A3")
            .cloned()
            .unwrap_or_else(|| panic!("A3"));
        assert!(
            a3.extends.is_empty(),
            "parenthetical without extends records nothing"
        );
        assert_eq!(
            a3.criteria.len(),
            1,
            "amendment items need no Acceptance Criteria gate"
        );
    }

    #[test]
    fn dotted_criteria_anywhere_in_the_block() {
        let text = "### Requirement 3: dotted\nIntro prose line.\n3.1 First self-numbered criterion.\n3.2 Second self-numbered criterion.\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        let r3 = doc
            .requirement("3")
            .cloned()
            .unwrap_or_else(|| panic!("req 3"));
        assert_eq!(
            r3.criteria
                .iter()
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>(),
            ["3.1", "3.2"]
        );
    }

    #[test]
    fn separators_and_case_insensitivity() {
        for (sep, upper) in [(":", false), (" - ", false), (" – ", false), (" — ", true)] {
            let title = "Some title";
            let heading = if upper {
                format!("### REQUIREMENT 7{sep}{title}")
            } else {
                format!("### Requirement 7{sep}{title}")
            };
            let doc = parse(
                &format!("{heading}\n\n#### Acceptance Criteria\n1. Criterion.\n"),
                F,
            );
            let r = doc
                .requirement("7")
                .cloned()
                .unwrap_or_else(|| panic!("req 7 for {sep:?}"));
            assert_eq!(r.title, title, "separator {sep:?}");
            assert!(doc.findings.is_empty(), "{:?} for {sep:?}", doc.findings);
        }
    }

    #[test]
    fn annotations_attach_to_requirement_or_indented_criterion() {
        let text = "### Requirement 1: annotated\n_Terms: spec, requirement_\n\n#### Acceptance Criteria\n1. First.\n   - _Terms: component_\n   _Guard: mode = safe_\n2. Second.\n_Conflicts: none expected_\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        let r1 = doc
            .requirement("1")
            .cloned()
            .unwrap_or_else(|| panic!("req 1"));
        assert_eq!(
            r1.annotations
                .iter()
                .map(|a| a.key.as_str())
                .collect::<Vec<_>>(),
            ["Terms", "Conflicts"]
        );
        let c1 = r1
            .criteria
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("criterion 1.1"));
        assert_eq!(
            c1.annotations
                .iter()
                .map(|a| a.key.as_str())
                .collect::<Vec<_>>(),
            ["Terms", "Guard"]
        );
    }

    #[test]
    fn blank_closes_criterion_and_marker_lines_do_not_join() {
        let text = "### Requirement 1: continuation\n\n#### Acceptance Criteria\n1. First line\ncontinued line.\n\nLoose prose between criteria belongs to no criterion.\n2. Second.\n";
        let doc = parse(text, F);
        let r1 = doc
            .requirement("1")
            .cloned()
            .unwrap_or_else(|| panic!("req 1"));
        let c1 = r1
            .criteria
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("criterion 1.1"));
        assert_eq!(c1.text, "First line continued line.");
        let c2 = r1
            .criteria
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("criterion 1.2"));
        assert_eq!(c2.text, "Second.");
    }

    #[test]
    fn gate_bare_markers_need_acceptance_criteria_heading() {
        let text = "### Requirement 1: gated\n1. Not a criterion, the gate is not set.\n\n#### Acceptance Criteria\n1. Now a criterion.\n";
        let doc = parse(text, F);
        let r1 = doc
            .requirement("1")
            .cloned()
            .unwrap_or_else(|| panic!("req 1"));
        assert_eq!(r1.criteria.len(), 1);
        assert_eq!(r1.criteria.first().map(|c| c.id.as_str()), Some("1.1"));
    }

    #[test]
    fn duplicates_keep_the_first_and_warn_at_the_later_line() {
        let text = "### Requirement 1: first\n1. One.\n\n### Requirement 1: duplicate requirement\n1. Later one.\n\n### Requirement 2: criterion duplicate\n#### Acceptance Criteria\n1. First definition.\n1. First definition again, same id.\n";
        let doc = parse(text, F);
        assert_eq!(
            doc.requirements.len(),
            2,
            "later duplicate requirement dropped"
        );
        let r2 = doc
            .requirement("2")
            .cloned()
            .unwrap_or_else(|| panic!("req 2"));
        assert_eq!(r2.criteria.len(), 1, "later duplicate criterion dropped");
        let dup_req = doc
            .findings
            .iter()
            .find(|f| f.code == codes::REQ_DUPLICATE_ID && f.line == Some(4))
            .cloned();
        assert!(dup_req.is_some(), "{:?}", doc.findings);
        let dup_crit = doc
            .findings
            .iter()
            .find(|f| f.code == codes::REQ_DUPLICATE_ID && f.line == Some(10))
            .cloned();
        assert!(dup_crit.is_some(), "{:?}", doc.findings);
    }

    #[test]
    fn non_numeric_requirement_heading_warns_and_skips_the_block() {
        let text = "### Requirement abc: non-numeric\n1. Must not become a criterion.\n\n### Requirement 2: fine\n1. Fine criterion.\n";
        let doc = parse(text, F);
        assert_eq!(doc.requirements.len(), 1);
        let f = doc
            .findings
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("expected finding"));
        assert_eq!(
            (f.code, f.severity, f.line),
            (codes::REQ_NON_NUMERIC_ID, Severity::Warning, Some(1))
        );
    }

    #[test]
    fn baseline_without_criteria_warns_amendment_does_not() {
        let text = "### Requirement 1: empty baseline\nJust prose, no criteria.\n\n### Amendment A1: empty amendment (extends Requirement 1)\nAlso prose only.\n";
        let doc = parse(text, F);
        let f = doc
            .findings
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("expected finding"));
        assert_eq!(
            (f.code, f.severity, f.line),
            (codes::REQ_NO_CRITERIA, Severity::Warning, Some(1))
        );
        assert_eq!(
            doc.findings.len(),
            1,
            "amendment without criteria stays silent: {:?}",
            doc.findings
        );
    }

    #[test]
    fn no_requirements_yields_empty_list_without_findings() {
        let doc = parse(
            "# Requirements Document\n\n## Introduction\n\nNothing here.\n",
            F,
        );
        assert!(doc.requirements.is_empty());
        assert!(doc.findings.is_empty());
        let doc = parse("", F);
        assert!(doc.requirements.is_empty());
        assert!(doc.findings.is_empty());
    }

    #[test]
    fn fenced_requirement_headings_are_ignored() {
        let text = "### Requirement 1: real\n\n#### Acceptance Criteria\n1. Real criterion.\n\n```\n### Requirement 9: fenced, must be ignored\n9.1 Fenced criterion, must be ignored.\n```\n\n### Requirement 2: after\n#### Acceptance Criteria\n1. After the fence.\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.requirements.len(), 2);
        assert_eq!(doc.criteria().count(), 2);
    }

    #[test]
    fn end_line_of_last_requirement_runs_to_block_end() {
        let text = "### Requirement 1: span\n1. One.\nTrailing prose inside the block.\n";
        let doc = parse(text, F);
        let r1 = doc
            .requirement("1")
            .cloned()
            .unwrap_or_else(|| panic!("req 1"));
        assert_eq!(r1.end_line, 3);
    }
}
