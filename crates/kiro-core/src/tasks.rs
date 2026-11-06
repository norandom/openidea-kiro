//! `tasks.md` into a two-level task tree (design component `TasksParser`,
//! requirements 5.1–5.13): checkbox state, the `*` optional marker, the `(P)`
//! parallel marker, nested majors and sub-tasks, detail lines, the three
//! interpreted annotations (`_Requirements:_`, `_Depends:_`, `_Boundary:_`)
//! with their line positions for the join's findings, generic annotations,
//! reference validation and duplicate/hierarchy findings, plus document and
//! per-major counts.

use crate::finding::{Finding, codes};
use crate::md::{Annotation, Fence, Heading, LineKind, ScannedLine, scan};

/// Totals for a document or one major task.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub checked: usize,
}

/// One task (major or sub-task).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub major: u32,
    pub sub: Option<u32>,
    pub description: String,
    pub checked: bool,
    pub optional: bool,
    pub parallel: bool,
    pub line: usize,
    pub end_line: usize,
    /// Non-annotation detail lines, trimmed.
    pub details: Vec<String>,
    pub requirements: Vec<String>,
    pub depends: Vec<String>,
    pub boundary: Vec<String>,
    /// Line of each interpreted annotation (for Join findings).
    pub requirements_line: Option<usize>,
    pub depends_line: Option<usize>,
    pub boundary_line: Option<usize>,
    /// Other `_Key: value_` lines.
    pub annotations: Vec<Annotation>,
    /// Empty for sub-tasks.
    pub subtasks: Vec<Task>,
    /// Sub-task totals for majors (`{1, checked}` when a major has none);
    /// `{1, checked}` for sub-tasks.
    pub counts: Counts,
}

/// The parsed tasks document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasksDoc {
    pub file: String,
    pub outline: Vec<Heading>,
    pub fences: Vec<Fence>,
    pub tasks: Vec<Task>,
    pub counts: Counts,
    pub findings: Vec<Finding>,
}

impl TasksDoc {
    /// Every task in document order: each major followed by its sub-tasks.
    pub fn all(&self) -> impl Iterator<Item = &Task> {
        fn flatten<'a>(out: &mut Vec<&'a Task>, tasks: &'a [Task]) {
            for t in tasks {
                out.push(t);
                flatten(out, &t.subtasks);
            }
        }
        let mut all = Vec::new();
        flatten(&mut all, &self.tasks);
        all.into_iter()
    }

    /// Finds a task by id at any level.
    pub fn task(&self, id: &str) -> Option<&Task> {
        self.all().find(|t| t.id == id)
    }
}

/// Parses `tasks.md`. Never panics; problems become findings.
pub fn parse(text: &str, file: &str) -> TasksDoc {
    let scanned = scan(text, file);
    let mut doc = TasksDoc {
        file: file.to_owned(),
        outline: scanned.outline,
        fences: scanned.fences,
        tasks: Vec::new(),
        counts: Counts::default(),
        findings: scanned.findings,
    };
    let mut seen_ids: Vec<String> = Vec::new();
    // Details of a dropped duplicate are ignored until the next task line.
    let mut dropped = false;

    for line in &scanned.lines {
        if let Some(task) = task_item(line) {
            dropped = !place_task(&mut doc, &mut seen_ids, task, line.line);
            continue;
        }
        if !dropped {
            attach_details(&mut doc, line);
        }
    }

    // Document totals over every task item, majors and subs alike.
    let mut total = 0usize;
    let mut checked = 0usize;
    for t in doc.tasks.iter_mut() {
        fill_counts(t);
        total += 1 + t.subtasks.len();
        checked += usize::from(t.checked);
        checked += t.subtasks.iter().filter(|s| s.checked).count();
    }
    doc.counts = Counts { total, checked };
    doc
}

/// Extracts the task shape of a line, if it is a task item.
fn task_item(line: &ScannedLine<'_>) -> Option<Task> {
    let LineKind::ListItem {
        bullet: _,
        checkbox,
        marker,
        text,
    } = line.kind
    else {
        return None;
    };
    let checkbox = checkbox?;
    let marker = marker?;
    let (parallel, description) = match text.strip_prefix("(P)") {
        Some(rest) => (true, rest.trim_start_matches(' ')),
        None => (false, text),
    };
    let groups: Vec<&str> = marker.label.split('.').collect();
    if groups.is_empty()
        || !groups
            .iter()
            .all(|g| !g.is_empty() && g.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let major: u32 = groups.first().and_then(|g| g.parse().ok()).unwrap_or(0);
    let sub: Option<u32> = groups.get(1).map(|g| g.parse().unwrap_or(0));
    Some(Task {
        id: marker.label.to_owned(),
        major,
        sub: if groups.len() >= 2 { sub } else { None },
        description: description.to_owned(),
        checked: checkbox.checked,
        optional: checkbox.optional,
        parallel,
        line: line.line,
        end_line: line.line,
        details: Vec::new(),
        requirements: Vec::new(),
        depends: Vec::new(),
        boundary: Vec::new(),
        requirements_line: None,
        depends_line: None,
        boundary_line: None,
        annotations: Vec::new(),
        subtasks: Vec::new(),
        counts: Counts::default(),
    })
}

/// Place a freshly read task into the tree, handling duplicates and
/// hierarchy findings. Returns false when the task was dropped.
fn place_task(
    doc: &mut TasksDoc,
    seen_ids: &mut Vec<String>,
    mut task: Task,
    line_no: usize,
) -> bool {
    if seen_ids.contains(&task.id) {
        doc.findings.push(
            Finding::new(
                codes::TASK_DUPLICATE_ID,
                &doc.file,
                format!(
                    "task {} is defined twice; the first definition is kept",
                    task.id
                ),
            )
            .at(line_no),
        );
        return false; // dropped, its details with it
    }
    seen_ids.push(task.id.clone());

    let groups = task.id.split('.').count();
    let is_sub = task.sub.is_some();
    let major_seen = doc.tasks.iter().any(|t| t.id == task.major.to_string());
    if groups > 2 || (is_sub && !major_seen) {
        let reason = if groups > 2 {
            format!("task id {} has more than two numeric components", task.id)
        } else {
            format!(
                "sub-task {} appears before its major {}",
                task.id, task.major
            )
        };
        doc.findings
            .push(Finding::new(codes::TASK_HIERARCHY, &doc.file, reason).at(line_no));
        task.sub = None; // kept at top level, id as written
        doc.tasks.push(task);
        return true;
    }

    if is_sub {
        let major_id = task.major.to_string();
        if let Some(m) = doc.tasks.iter_mut().rev().find(|t| t.id == major_id) {
            m.subtasks.push(task);
        }
    } else {
        doc.tasks.push(task);
    }
    true
}

/// Attaches detail lines (indent > 0) to the open task, when one is.
fn attach_details(doc: &mut TasksDoc, line: &ScannedLine<'_>) {
    // Only indented lines continue a task; blanks neither close nor attach.
    if line.indent == 0 {
        return;
    }
    let LineKind::Annotation { key, value } = line.kind else {
        if matches!(line.kind, LineKind::Blank) {
            return;
        }
        if let Some(text) = line_text(line) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                push_detail(doc, trimmed, line.line);
            }
        }
        return;
    };
    let entries: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_owned)
        .collect();
    let key_lower = key.to_ascii_lowercase();
    let line_no = line.line;
    match key_lower.as_str() {
        "requirements" => apply_annotation(doc, |t| {
            t.requirements = entries.clone();
            t.requirements_line = Some(line_no);
            t.end_line = line_no;
        }),
        "depends" => apply_annotation(doc, |t| {
            t.depends = entries.clone();
            t.depends_line = Some(line_no);
            t.end_line = line_no;
        }),
        "boundary" => apply_annotation(doc, |t| {
            t.boundary = entries.clone();
            t.boundary_line = Some(line_no);
            t.end_line = line_no;
        }),
        _ => {
            let ann = Annotation {
                key: key.to_owned(),
                value: value.to_owned(),
                line: line_no,
            };
            last_task_mut(doc, |t| {
                t.annotations.push(ann.clone());
                t.end_line = line_no;
            });
        }
    }
    if key_lower == "requirements" {
        for entry in &entries {
            if !valid_requirement_ref(entry) {
                doc.findings.push(
                    Finding::new(
                        codes::TASK_REQ_REF_INVALID,
                        &doc.file,
                        format!(
                            "requirement reference {entry:?} is not of the form N, N.M or A<K>.M"
                        ),
                    )
                    .at(line.line),
                );
            }
        }
    }
}

/// Runs `f` on the currently open task (the most recent sub-task if the last
/// major has subs, else the last task).
fn last_task_mut(doc: &mut TasksDoc, f: impl FnOnce(&mut Task)) {
    let Some(major) = doc.tasks.last_mut() else {
        return;
    };
    if let Some(sub) = major.subtasks.last_mut() {
        f(sub);
    } else {
        f(major);
    }
}

fn apply_annotation(doc: &mut TasksDoc, f: impl FnOnce(&mut Task)) {
    last_task_mut(doc, f);
}

fn push_detail(doc: &mut TasksDoc, detail: &str, line: usize) {
    last_task_mut(doc, |t| {
        t.details.push(detail.to_owned());
        t.end_line = line;
    });
}

/// Requirement references: `N`, `N.M` or `A<K>.M`. Crate-visible because
/// the join needs the same shape rule to skip already-invalid references
/// (their finding belongs to the tasks parser).
pub(crate) fn valid_requirement_ref(entry: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if digits(entry) {
        return true;
    }
    if let Some(rest) = entry.strip_prefix('A')
        && let Some((k, m)) = rest.split_once('.')
    {
        return digits(k) && digits(m);
    }
    if let Some((a, b)) = entry.split_once('.') {
        return digits(a) && digits(b);
    }
    false
}

/// Ends the open task's detail collection; placement already handled
/// everything, so this is the natural place for future end-of-task hooks.
/// Fills `counts` for one major: sub-task totals, or `{1, checked}` for a
/// childless major.
fn fill_counts(t: &mut Task) {
    if t.subtasks.is_empty() {
        t.counts = Counts {
            total: 1,
            checked: usize::from(t.checked),
        };
        return;
    }
    t.counts = Counts {
        total: t.subtasks.len(),
        checked: t.subtasks.iter().filter(|s| s.checked).count(),
    };
    for sub in t.subtasks.iter_mut() {
        fill_counts(sub);
    }
}

fn line_text<'a>(line: &ScannedLine<'a>) -> Option<&'a str> {
    match line.kind {
        LineKind::Text => Some(line.raw.trim()),
        LineKind::ListItem { text, .. } => Some(text),
        _ => None,
    }
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::finding::{Severity, codes};
    use crate::tasks::{Task, parse};

    const F: &str = ".kiro/specs/demo/tasks.md";

    // Real shape (this repo's own tasks.md).
    const REAL: &str = "# Implementation Plan\n\n- [ ] 1. Foundation\n- [x] 1.1 Implement the scanner\n  - Reads every line once.\n  - _Requirements: 1.1, 1.3_\n  - _Depends: nothing_\n  - _Boundary: MarkdownScanner_\n- [ ] 1.2 (P) Implement the parser\n  - _Requirements: 1.2_\n  - _Boundary: RequirementsParser_\n- [ ]* 2. Optional hardening\n  - _Requirements: 2.1, 2.2_\n  - _Notes: keep an eye on it_\n";

    #[test]
    fn real_shape_tree_flags_annotations_and_details() {
        let doc = parse(REAL, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.tasks.len(), 2, "two majors");
        let major1 = doc
            .tasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("major 1"));
        assert_eq!(
            (major1.id.as_str(), major1.major, major1.sub),
            ("1", 1, None)
        );
        assert!(!major1.checked);
        assert!(!major1.optional);
        assert!(!major1.parallel);
        assert_eq!(major1.line, 3);
        assert!(major1.details.is_empty(), "major 1 has no bare details");
        assert_eq!(major1.subtasks.len(), 2);
        let sub11 = major1
            .subtasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("sub 1.1"));
        assert_eq!(
            (sub11.id.as_str(), sub11.major, sub11.sub),
            ("1.1", 1, Some(1))
        );
        assert!(sub11.checked);
        assert_eq!(sub11.description, "Implement the scanner");
        assert_eq!(sub11.details, ["Reads every line once."]);
        assert_eq!(sub11.requirements, ["1.1", "1.3"]);
        assert_eq!(sub11.depends, ["nothing"]);
        assert_eq!(sub11.boundary, ["MarkdownScanner"]);
        assert_eq!(sub11.requirements_line, Some(6));
        assert_eq!(sub11.end_line, 8, "end_line spans to the last detail line");
        let sub12 = major1
            .subtasks
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("sub 1.2"));
        assert!(sub12.parallel, "(P) recognised");
        assert_eq!(sub12.description, "Implement the parser");
        assert!(!sub12.checked);
        let major2 = doc
            .tasks
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("major 2"));
        assert!(major2.optional, "trailing * marks optional");
        assert_eq!(major2.requirements, ["2.1", "2.2"]);
        assert_eq!(major2.annotations.len(), 1, "other keys stay generic");
        assert_eq!(
            major2.annotations.first().map(|a| a.key.as_str()),
            Some("Notes")
        );
    }

    #[test]
    fn checked_variants_upper_case_and_closed_by_dot() {
        let text = "- [X] 1. Upper case checked\n- [x] 2. Lower case checked\n- [ ] 3. Unchecked\n";
        let doc = parse(text, F);
        assert!(doc.tasks.iter().all(|t| t.subtasks.is_empty()));
        assert!(doc.tasks.first().is_some_and(|t| t.checked));
        assert!(doc.tasks.get(1).is_some_and(|t| t.checked));
        assert!(doc.tasks.get(2).is_some_and(|t| !t.checked));
    }

    #[test]
    fn parallel_marker_is_case_sensitive_and_stripped() {
        let text = "- [ ] 1. (P) parallel major\n- [ ] 1.1 (p) lowercase stays in the text\n";
        let doc = parse(text, F);
        assert!(
            doc.tasks
                .first()
                .is_some_and(|t| t.parallel && t.description == "parallel major")
        );
        let sub = doc
            .tasks
            .first()
            .and_then(|t| t.subtasks.first())
            .cloned()
            .unwrap_or_else(|| panic!("sub 1.1"));
        assert!(!sub.parallel);
        assert_eq!(sub.description, "(p) lowercase stays in the text");
    }

    #[test]
    fn orphan_sub_and_depth_three_report_hierarchy_and_stay_top_level() {
        let text = "- [ ] 1.1 Sub with no major seen\n- [ ] 2. Major\n- [ ] 2.1.1 Depth-three id\n- [ ] 2.2 Regular sub\n";
        let doc = parse(text, F);
        let hierarchy: Vec<usize> = doc
            .findings
            .iter()
            .filter(|f| f.code == codes::TASK_HIERARCHY)
            .map(|f| f.line.unwrap_or(0))
            .collect();
        assert_eq!(
            hierarchy,
            [1, 3],
            "orphan sub at line 1, depth-three at line 3"
        );
        assert!(doc.findings.iter().all(|f| f.severity == Severity::Warning));
        // Both stay at top level with sub = None and the id as written.
        let orphan = doc
            .tasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("orphan"));
        assert_eq!((orphan.id.as_str(), orphan.sub), ("1.1", None));
        let deep = doc
            .tasks
            .get(2)
            .cloned()
            .unwrap_or_else(|| panic!("depth three"));
        assert_eq!((deep.id.as_str(), deep.sub), ("2.1.1", None));
        // The regular sub nests normally.
        let major2 = doc
            .tasks
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("major 2"));
        assert_eq!(major2.subtasks.len(), 1);
        assert_eq!(major2.subtasks.first().map(|t| t.id.as_str()), Some("2.2"));
    }

    #[test]
    fn details_attach_until_next_task_or_heading_and_are_trimmed() {
        let text = "# Implementation Plan\n\n- [ ] 1. First\n  - Bullet detail.\n    Indented continuation text.\n  - _Boundary: A_\n\nLoose text after a blank line.\n\n## Next section\n- [ ] 2. Second\n";
        let doc = parse(text, F);
        let first = doc
            .tasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("task 1"));
        assert_eq!(
            first.details,
            ["Bullet detail.", "Indented continuation text."],
            "details trimmed, annotations excluded"
        );
        assert_eq!(first.boundary, ["A"]);
        assert_eq!(
            first.end_line, 6,
            "blank line then heading ends the task block; details stop at the blank-line-free run"
        );
        let second = doc
            .tasks
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("task 2"));
        assert_eq!(second.description, "Second");
        assert_eq!(doc.tasks.len(), 2, "loose prose creates no task");
    }

    #[test]
    fn annotation_values_split_trim_and_drop_empties() {
        let text = "- [ ] 1. Task\n  - _Requirements: 1.1 , , 1.2 _\n  - _depends: 2.1_\n  - _BOUNDARY: Core, Ui_\n";
        let doc = parse(text, F);
        let t = doc
            .tasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("task 1"));
        assert_eq!(t.requirements, ["1.1", "1.2"]);
        assert_eq!(t.depends, ["2.1"], "interpreted keys are case-insensitive");
        assert_eq!(t.boundary, ["Core", "Ui"]);
    }

    #[test]
    fn requirement_reference_shapes_and_invalid_entries() {
        let text = "- [ ] 1. Task\n  - _Requirements: 1, 1.2, A1.3, 9x, nope_\n";
        let doc = parse(text, F);
        let t = doc
            .tasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("task 1"));
        assert_eq!(
            t.requirements,
            ["1", "1.2", "A1.3", "9x", "nope"],
            "invalid entries are kept raw"
        );
        let invalid: Vec<usize> = doc
            .findings
            .iter()
            .filter(|f| f.code == codes::TASK_REQ_REF_INVALID)
            .map(|f| f.line.unwrap_or(0))
            .collect();
        assert_eq!(
            invalid,
            [2, 2],
            "one finding per invalid entry, at the annotation line"
        );
        assert!(doc.findings.iter().all(|f| f.severity == Severity::Warning));
    }

    #[test]
    fn duplicate_ids_keep_the_first_definition() {
        let text = "- [ ] 1. First\n- [ ] 1.1 Duplicate pair, first\n- [x] 1.1 Duplicate pair, second is dropped\n";
        let doc = parse(text, F);
        assert_eq!(doc.counts.total, 2);
        let kept = doc
            .tasks
            .first()
            .and_then(|t| t.subtasks.first())
            .cloned()
            .unwrap_or_else(|| panic!("sub 1.1"));
        assert_eq!(kept.description, "Duplicate pair, first");
        assert!(!kept.checked, "the first definition is kept");
        assert!(matches!(
            doc.findings.first(),
            Some(f) if f.code == codes::TASK_DUPLICATE_ID && f.line == Some(3)
        ));
    }

    #[test]
    fn counts_total_and_checked_document_and_per_major() {
        let text = "- [x] 1. Major with children\n- [x] 1.1 Done sub\n- [ ] 1.2 Open sub\n- [ ] 2. Childless major\n- [x] 3. Another childless major\n";
        let doc = parse(text, F);
        assert_eq!((doc.counts.total, doc.counts.checked), (5, 3));
        let m1 = doc
            .tasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("major 1"));
        assert_eq!(
            (m1.counts.total, m1.counts.checked),
            (2, 1),
            "major with children counts them"
        );
        let m2 = doc
            .tasks
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("major 2"));
        assert_eq!(
            (m2.counts.total, m2.counts.checked),
            (1, 0),
            "childless major counts itself"
        );
        let m3 = doc
            .tasks
            .get(2)
            .cloned()
            .unwrap_or_else(|| panic!("major 3"));
        assert_eq!((m3.counts.total, m3.counts.checked), (1, 1));
        let sub = m1
            .subtasks
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("sub 1.1"));
        assert_eq!((sub.counts.total, sub.counts.checked), (1, 1));
    }

    #[test]
    fn all_iterates_preorder_and_task_lookup() {
        let doc = parse(REAL, F);
        let ids: Vec<&str> = doc.all().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["1", "1.1", "1.2", "2"]);
        assert!(doc.task("1.2").is_some());
        assert!(doc.task("9.9").is_none());
    }

    #[test]
    fn tasks_without_checkbox_or_marker_are_ignored() {
        let text =
            "- [ ] Plain checkbox without number\n- 1.2 Number without checkbox\n- Just a bullet\n";
        let doc = parse(text, F);
        assert!(doc.tasks.is_empty());
        assert!(doc.findings.is_empty());
    }

    #[test]
    fn closed_by_dot_is_accepted_and_ignored() {
        let text = "- [ ] 1. Foundation\n- [ ] 1.2 (P) Sub task\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.counts.total, 2);
    }

    #[test]
    fn empty_document_is_quiet() {
        let doc = parse("", F);
        assert!(doc.tasks.is_empty());
        assert_eq!((doc.counts.total, doc.counts.checked), (0, 0));
        assert!(doc.findings.is_empty());
    }

    #[test]
    fn task_struct_is_plain_data() {
        let t = Task::default();
        assert_eq!(t.id, "");
        assert!(!t.checked);
    }
}
