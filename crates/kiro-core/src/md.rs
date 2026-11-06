//! One forward pass turning text into typed lines with identical fence,
//! heading and numbering semantics for every Markdown parser (design
//! component `MarkdownScanner`, requirements 3.1–3.6): 1-based line numbers
//! identical for LF and CRLF, a tolerated byte-order mark, an ATX-heading
//! outline, fenced blocks with info string / span / body, list items with
//! bullet, checkbox and numbered markers, `_Key: value_` annotations and
//! table rows. Unclosed fences run to the last line and yield
//! `MD_FENCE_UNCLOSED`.

use crate::finding::{Finding, codes};

/// The result of scanning one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scan<'a> {
    pub lines: Vec<ScannedLine<'a>>,
    pub outline: Vec<Heading>,
    pub fences: Vec<Fence>,
    pub findings: Vec<Finding>,
}

/// One typed line with its 1-based number and indentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedLine<'a> {
    pub line: usize,
    pub raw: &'a str,
    pub indent: usize,
    pub kind: LineKind<'a>,
}

/// The recognised shapes a line can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind<'a> {
    Blank,
    Heading {
        level: u8,
        text: &'a str,
    },
    ListItem {
        bullet: Option<char>,
        checkbox: Option<Checkbox>,
        marker: Option<Numbered<'a>>,
        text: &'a str,
    },
    /// `_Key: value_`, optionally after a bullet.
    Annotation {
        key: &'a str,
        value: &'a str,
    },
    TableRow {
        cells: Vec<&'a str>,
        separator: bool,
    },
    FenceOpen {
        info: &'a str,
    },
    FenceClose,
    Fenced,
    Text,
}

/// `[ ]`, `[x]`, `[X]`, optionally followed by `*` (optional task).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkbox {
    pub checked: bool,
    pub optional: bool,
}

/// A numbered label: digits with optional `.digits` groups, closed by `.` or
/// `)` before a space, or followed directly by a space. `label` keeps every
/// group except the closing character; parsers decide validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Numbered<'a> {
    pub label: &'a str,
    pub closed_by: Option<char>,
}

/// One outline entry (owned: headings travel into navigation output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub line: usize,
}

/// One fenced block: info string, first and last line, body without the
/// fence lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    pub info: String,
    pub start: usize,
    pub end: usize,
    pub body: String,
}

/// Owned form of a `_Key: value_` line; shared by the requirements and tasks
/// parsers (defined here so both import downward).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    pub key: String,
    pub value: String,
    pub line: usize,
}

/// Scans `text` in one forward pass. Never panics; any `&str` is accepted.
pub fn scan<'a>(text: &'a str, file: &str) -> Scan<'a> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut out = Scan {
        lines: Vec::new(),
        outline: Vec::new(),
        fences: Vec::new(),
        findings: Vec::new(),
    };
    // Open fence state: (marker char, marker run length, start line, body).
    let mut open: Option<(char, usize, usize, String)> = None;
    let mut last_line = 0usize;

    for (idx, raw_line) in text.split_inclusive('\n').enumerate() {
        let line_no = idx + 1;
        last_line = line_no;
        let raw = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed_start = raw.trim_start_matches(' ');
        let indent = raw.len() - trimmed_start.len();
        let kind = classify(trimmed_start, raw, &mut open, &mut out, line_no);
        if let LineKind::Heading { level, text } = kind {
            out.outline.push(Heading {
                level,
                text: text.to_owned(),
                line: line_no,
            });
        }
        out.lines.push(ScannedLine {
            line: line_no,
            raw,
            indent,
            kind,
        });
    }

    // An unclosed fence ends at the last line and is reported once.
    if let Some((_, _, start, body)) = open {
        let end = last_line;
        if let Some(fence) = out.fences.last_mut() {
            fence.end = end;
            fence.body = body;
        }
        out.findings.push(
            Finding::new(
                codes::MD_FENCE_UNCLOSED,
                file,
                "fenced block is never closed; it ends at the last line",
            )
            .at(start),
        );
    }
    out
}

/// Classifies one indent-stripped line, updating fence state.
fn classify<'a>(
    t: &'a str,
    raw: &'a str,
    open: &mut Option<(char, usize, usize, String)>,
    out: &mut Scan<'a>,
    line_no: usize,
) -> LineKind<'a> {
    // Inside a fence: only a matching close line (same character, at least
    // the opening length, nothing else on the line) ends it.
    if let Some((och, olen)) = open.as_ref().map(|(c, l, _, _)| (*c, *l)) {
        let rest = t.trim_start_matches(och);
        let run_bytes = t.len() - rest.len();
        if run_bytes >= olen && rest.trim().is_empty() {
            if let Some((_, _, _, body)) = open.take()
                && let Some(fence) = out.fences.last_mut()
            {
                fence.end = line_no;
                fence.body = body;
            }
            return LineKind::FenceClose;
        }
        if let Some((_, _, _, body)) = open.as_mut() {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(raw);
        }
        return LineKind::Fenced;
    }

    if t.is_empty() {
        return LineKind::Blank;
    }

    // Fence open: three or more backticks or tildes starting the line.
    for ch in ['`', '~'] {
        if t.starts_with(ch) {
            let run_bytes = t.len() - t.trim_start_matches(ch).len();
            if run_bytes >= 3 {
                let info = t.trim_start_matches(ch).trim();
                *open = Some((ch, run_bytes, line_no, String::new()));
                out.fences.push(Fence {
                    info: info.to_owned(),
                    start: line_no,
                    end: line_no,
                    body: String::new(),
                });
                return LineKind::FenceOpen { info };
            }
        }
    }

    // Heading: 1–6 `#` followed by a space or the end of the line.
    if let Some(kind) = heading(t) {
        return kind;
    }

    // Annotation, optionally after a bullet.
    if let Some((key, value)) = annotation_of(t) {
        return LineKind::Annotation { key, value };
    }
    if starts_with_bullet(t)
        && let Some((key, value)) = annotation_of(skip_bullet(t))
    {
        return LineKind::Annotation { key, value };
    }

    // List item: optional bullet, optional checkbox, optional numbered label.
    if let Some(kind) = list_item(t) {
        return kind;
    }

    // Table row: the line starts with `|`.
    if t.starts_with('|') {
        let cells: Vec<&str> = t
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        let separator = !cells.is_empty()
            && cells.iter().all(|c| {
                let stripped = c.trim_matches([':', '-']);
                stripped.is_empty() && c.contains('-')
            });
        return LineKind::TableRow { cells, separator };
    }

    LineKind::Text
}

/// ATX heading: 1–6 `#` then a space or the end of the line. The text keeps
/// everything after the single following space.
fn heading(t: &str) -> Option<LineKind<'_>> {
    let hashes = t.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let after = t.trim_start_matches('#');
    if !after.is_empty() && !after.starts_with(' ') {
        return None; // `##tag` and the like are plain text
    }
    let text = after.strip_prefix(' ').unwrap_or(after);
    Some(LineKind::Heading {
        level: hashes as u8,
        text,
    })
}

/// `_Key: value_` with a non-empty, space-free key.
fn annotation_of(t: &str) -> Option<(&str, &str)> {
    let inner = t.strip_prefix('_')?;
    let inner = inner.strip_suffix('_')?;
    let pos = inner.find(": ")?;
    let (key, rest) = inner.split_at(pos);
    let value = rest.strip_prefix(": ").unwrap_or(rest);
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    Some((key, value))
}

/// A bullet is `-`, `*` or `+` followed by a space.
fn starts_with_bullet(t: &str) -> bool {
    let mut chars = t.chars();
    matches!(chars.next(), Some('-' | '*' | '+')) && chars.next() == Some(' ')
}

/// The text after a bullet and its space (bullet characters are one byte).
fn skip_bullet(t: &str) -> &str {
    t.get(2..).unwrap_or_default().trim_start_matches(' ')
}

/// List item parsing: optional bullet, optional checkbox, optional numbered
/// label, then the item text.
fn list_item(t: &str) -> Option<LineKind<'_>> {
    let (bullet, rest) = if starts_with_bullet(t) {
        (t.chars().next(), skip_bullet(t))
    } else {
        (None, t)
    };
    if bullet.is_none() && !rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut rest = rest;
    let mut checkbox = None;
    if let Some((cb, after)) = checkbox_of(rest) {
        checkbox = Some(cb);
        rest = after;
    }
    let mut marker = None;
    if let Some((m, after)) = numbered_of(rest) {
        marker = Some(m);
        rest = after;
    }
    if bullet.is_none() && checkbox.is_none() && marker.is_none() {
        return None;
    }
    let text = rest.trim_start_matches(' ');
    Some(LineKind::ListItem {
        bullet,
        checkbox,
        marker,
        text,
    })
}

/// `[ ]` / `[x]` / `[X]` followed by an optional `*`, then a space or EOL.
fn checkbox_of(t: &str) -> Option<(Checkbox, &str)> {
    let inner = t.strip_prefix('[')?;
    let mark = inner.chars().next()?;
    let checked = match mark {
        ' ' => false,
        'x' | 'X' => true,
        _ => return None,
    };
    let after = inner.split_at(mark.len_utf8()).1;
    let after_bracket = after.strip_prefix(']')?;
    let (optional, after_star) = match after_bracket.strip_prefix('*') {
        Some(a) => (true, a),
        None => (false, after_bracket),
    };
    if after_star.is_empty() || after_star.starts_with(' ') {
        Some((Checkbox { checked, optional }, after_star))
    } else {
        None
    }
}

/// Digits with optional `.digits` groups, closed by `.`/`)` + space or
/// followed by a space; `label` keeps every group except the closing char.
fn numbered_of(t: &str) -> Option<(Numbered<'_>, &str)> {
    if !t.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    let (token, rest_after_token) = match t.find(' ') {
        Some(pos) => {
            let (token, rest) = t.split_at(pos);
            (token, rest.strip_prefix(' ').unwrap_or(rest))
        }
        None => (t, ""),
    };
    let (label, closed_by) = if let Some(label) = token.strip_suffix(')') {
        (label, Some(')'))
    } else if token.ends_with('.') {
        let label = token.strip_suffix('.')?;
        // A trailing `.` only closes when it follows a digit (`1.`, `1.1.`),
        // so `1..` and `.` stay invalid tokens.
        if !label.chars().last().is_some_and(|c| c.is_ascii_digit()) {
            return None;
        }
        (label, Some('.'))
    } else {
        (token, None)
    };
    let valid = !label.is_empty()
        && label
            .split('.')
            .all(|group| !group.is_empty() && group.chars().all(|c| c.is_ascii_digit()));
    if !valid {
        return None;
    }
    Some((Numbered { label, closed_by }, rest_after_token))
}

// Assertion failures panic by design; the crate-wide `panic` denial targets
// production code paths.
#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::md::{Annotation, LineKind, scan};

    const F: &str = "doc.md";

    #[test]
    fn crlf_and_lf_number_lines_identically() {
        let lf = scan("a\nb\nc", F);
        let crlf = scan("a\r\nb\r\nc", F);
        assert_eq!(lf.lines.len(), 3);
        assert_eq!(crlf.lines.len(), 3);
        for (a, b) in lf.lines.iter().zip(crlf.lines.iter()) {
            assert_eq!(a.line, b.line);
            assert_eq!(a.raw, b.raw, "CR must be stripped from raw");
        }
    }

    #[test]
    fn bom_is_ignored_for_first_line_markers() {
        let s = scan("\u{FEFF}# Title\n", F);
        assert_eq!(s.lines.first().map(|l| l.line), Some(1));
        assert!(matches!(
            s.lines.first().map(|l| &l.kind),
            Some(LineKind::Heading {
                level: 1,
                text: "Title"
            })
        ));
    }

    #[test]
    fn hash_run_is_heading_only_with_space_or_eol() {
        let s = scan("# A\n##tag\n####### deep\n### C ###\n", F);
        assert_eq!(s.outline.len(), 2, "##tag and 7-# are not headings");
        assert!(
            matches!(s.outline.first(), Some(h) if h.level == 1 && h.text == "A" && h.line == 1)
        );
        assert!(
            matches!(s.outline.get(1), Some(h) if h.level == 3 && h.text == "C ###" && h.line == 4)
        );
    }

    #[test]
    fn fence_hides_everything_and_records_info_span_body() {
        let text = "# Outside\n```rust,ignore\nnot a # heading\n- not a list\n```\n# After\n";
        let s = scan(text, F);
        assert_eq!(s.outline.len(), 2);
        assert!(
            matches!(s.fences.first(), Some(f) if f.info == "rust,ignore" && f.start == 2 && f.end == 5 && f.body == "not a # heading\n- not a list")
        );
        assert!(matches!(
            s.lines.get(2).map(|l| &l.kind),
            Some(LineKind::Fenced)
        ));
        assert!(matches!(
            s.lines.get(4).map(|l| &l.kind),
            Some(LineKind::FenceClose)
        ));
        assert!(matches!(
            s.lines.get(1).map(|l| &l.kind),
            Some(LineKind::FenceOpen {
                info: "rust,ignore"
            })
        ));
    }

    #[test]
    fn tilde_fences_and_same_char_rule() {
        let s = scan("~~~\n~~~\n````\n```\n````\n", F);
        assert_eq!(
            s.fences.len(),
            2,
            "backticks do not close tildes and vice versa"
        );
        assert!(
            matches!(s.fences.first(), Some(f) if f.start == 1 && f.end == 2 && f.info.is_empty())
        );
        assert!(matches!(s.fences.get(1), Some(f) if f.start == 3 && f.end == 5));
    }

    #[test]
    fn shorter_close_does_not_close_the_fence() {
        let s = scan("````\n```\n````\n", F);
        assert_eq!(s.fences.len(), 1);
        assert!(
            matches!(s.fences.first(), Some(f) if f.end == 3),
            "3-backtick line stays inside; fence closes at line 3"
        );
        assert!(matches!(
            s.lines.get(1).map(|l| &l.kind),
            Some(LineKind::Fenced)
        ));
    }

    #[test]
    fn unclosed_fence_runs_to_last_line_with_finding() {
        let s = scan("text\n```mermaid\ngraph TD\n", F);
        assert_eq!(s.fences.len(), 1);
        assert!(
            matches!(s.fences.first(), Some(f) if f.start == 2 && f.end == 3 && f.info == "mermaid")
        );
        assert!(
            matches!(s.findings.first(), Some(fd) if fd.code == "MD_FENCE_UNCLOSED"
            && fd.severity == crate::finding::Severity::Info && fd.line == Some(2))
        );
    }

    #[test]
    fn list_items_bullet_checkbox_and_numbered() {
        let s = scan(
            "- plain\n- [ ] unchecked\n- [x] checked\n- [X] checked upper\n- [ ]* optional star\n* star bullet\n+ plus bullet\n1. dot\n2) paren\n1.1 dotted group\n1.1. dotted closed\n1.1.1 three groups\n",
            F,
        );
        use LineKind::*;
        let kinds: Vec<&LineKind> = s.lines.iter().map(|l| &l.kind).collect();
        assert!(matches!(
            kinds.first(),
            Some(ListItem {
                bullet: Some('-'),
                checkbox: None,
                marker: None,
                text: "plain"
            })
        ));
        assert!(
            matches!(kinds.get(1), Some(ListItem { checkbox: Some(c), text: "unchecked", .. }) if !c.checked && !c.optional)
        );
        assert!(
            matches!(kinds.get(2), Some(ListItem { checkbox: Some(c), text: "checked", .. }) if c.checked && !c.optional)
        );
        assert!(
            matches!(kinds.get(3), Some(ListItem { checkbox: Some(c), text: "checked upper", .. }) if c.checked)
        );
        assert!(
            matches!(kinds.get(4), Some(ListItem { checkbox: Some(c), text: "optional star", .. }) if c.optional)
        );
        assert!(matches!(
            kinds.get(5),
            Some(ListItem {
                bullet: Some('*'),
                text: "star bullet",
                ..
            })
        ));
        assert!(matches!(
            kinds.get(6),
            Some(ListItem {
                bullet: Some('+'),
                text: "plus bullet",
                ..
            })
        ));
        assert!(
            matches!(kinds.get(7), Some(ListItem { marker: Some(m), text: "dot", .. }) if m.label == "1" && m.closed_by == Some('.'))
        );
        assert!(
            matches!(kinds.get(8), Some(ListItem { marker: Some(m), text: "paren", .. }) if m.label == "2" && m.closed_by == Some(')'))
        );
        assert!(
            matches!(kinds.get(9), Some(ListItem { marker: Some(m), text: "dotted group", .. }) if m.label == "1.1" && m.closed_by.is_none())
        );
        assert!(
            matches!(kinds.get(10), Some(ListItem { marker: Some(m), text: "dotted closed", .. }) if m.label == "1.1" && m.closed_by == Some('.'))
        );
        assert!(
            matches!(kinds.get(11), Some(ListItem { marker: Some(m), text: "three groups", .. }) if m.label == "1.1.1")
        );
    }

    #[test]
    fn annotations_plain_and_after_bullet_with_bad_key_rejected() {
        let s = scan(
            "_Requirements: 1.1, 1.2_\n- _Boundary: A_\n_bad key: x_\n_no_colon value_\n_notannotation\n",
            F,
        );
        let anns: Vec<&LineKind> = s.lines.iter().map(|l| &l.kind).collect();
        assert!(matches!(
            anns.first(),
            Some(LineKind::Annotation {
                key: "Requirements",
                value: "1.1, 1.2"
            })
        ));
        assert!(matches!(
            anns.get(1),
            Some(LineKind::Annotation {
                key: "Boundary",
                value: "A"
            })
        ));
        assert!(
            !matches!(anns.get(2), Some(LineKind::Annotation { .. })),
            "key with space is no annotation"
        );
        assert!(
            !matches!(anns.get(3), Some(LineKind::Annotation { .. })),
            "no ': ' separator"
        );
        assert!(!matches!(anns.get(4), Some(LineKind::Annotation { .. })));
    }

    #[test]
    fn owned_annotation_value() {
        let a = Annotation {
            key: "Depends".into(),
            value: "1.1".into(),
            line: 4,
        };
        assert_eq!(
            (a.key.as_str(), a.value.as_str(), a.line),
            ("Depends", "1.1", 4)
        );
    }

    #[test]
    fn table_rows_and_separator_rows() {
        let s = scan("| a | b |\n|---|:--:|\n| 1 | 2 |\nnot a table\n", F);
        let kinds: Vec<&LineKind> = s.lines.iter().map(|l| &l.kind).collect();
        match kinds.first() {
            Some(LineKind::TableRow { cells, separator }) => {
                assert_eq!(cells, &["a", "b"]);
                assert!(!separator);
            }
            other => panic!("first row: {other:?}"),
        }
        assert!(matches!(
            kinds.get(1),
            Some(LineKind::TableRow {
                separator: true,
                ..
            })
        ));
        assert!(matches!(kinds.get(3), Some(LineKind::Text)));
    }

    #[test]
    fn blank_indent_and_text_kinds() {
        let s = scan("   \n  - indented bullet\nplain text\n", F);
        assert!(matches!(
            s.lines.first().map(|l| &l.kind),
            Some(LineKind::Blank)
        ));
        let indented = s.lines.get(1);
        assert_eq!(indented.map(|l| l.indent), Some(2));
        assert!(matches!(
            indented.map(|l| &l.kind),
            Some(LineKind::ListItem {
                bullet: Some('-'),
                ..
            })
        ));
        assert!(matches!(
            s.lines.get(2).map(|l| &l.kind),
            Some(LineKind::Text)
        ));
    }

    #[test]
    fn empty_document_and_no_trailing_newline() {
        let s = scan("", F);
        assert_eq!(s.lines.len(), 0);
        let s = scan("one", F);
        assert_eq!(s.lines.len(), 1);
    }
}
