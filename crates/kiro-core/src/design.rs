//! `design.md` into components, design files with owners, unassigned files
//! and Mermaid blocks (design component `DesignParser`, requirements 6.1–6.3,
//! 6.5–6.10 in this task; the fenced tree shape and its merge are task 2.5).
//! Section regions run from their heading to the next heading of equal or
//! higher level; mapping tables and bold-component bullets live inside the
//! File Structure Plan; components register from level-4 headings in the
//! Components region and from any table's `Component` column.

use crate::finding::{Finding, codes};
use crate::md::{Fence, Heading, LineKind, ScannedLine, scan};

/// One design component and the files mapped to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub line: usize,
    pub files: Vec<String>,
}

/// One file named by the plan, with its owner when mapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignFile {
    pub path: String,
    pub component: Option<String>,
    pub responsibility: Option<String>,
    pub line: usize,
}

/// One fenced Mermaid block with its span and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MermaidBlock {
    pub start: usize,
    pub end: usize,
    pub body: String,
}

/// The parsed design document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignDoc {
    pub file: String,
    pub outline: Vec<Heading>,
    pub fences: Vec<Fence>,
    pub mermaid: Vec<MermaidBlock>,
    pub components: Vec<Component>,
    pub files: Vec<DesignFile>,
    pub findings: Vec<Finding>,
}

impl DesignDoc {
    /// Finds a component by trimmed, case-insensitive name.
    pub fn component(&self, name: &str) -> Option<&Component> {
        let wanted = name.trim();
        self.components
            .iter()
            .find(|c| c.name.trim().eq_ignore_ascii_case(wanted))
    }

    /// The files of one component (trimmed, case-insensitive).
    pub fn files_of(&self, component: &str) -> impl Iterator<Item = &DesignFile> {
        let wanted = component.trim().to_ascii_lowercase();
        self.files.iter().filter(move |f| {
            f.component
                .as_deref()
                .is_some_and(|c| c.trim().eq_ignore_ascii_case(&wanted))
        })
    }

    /// Files without a component mapping.
    pub fn unassigned_files(&self) -> impl Iterator<Item = &DesignFile> {
        self.files.iter().filter(|f| f.component.is_none())
    }
}

/// Strips backticks, converts `\` to `/`, removes a leading `./` and
/// collapses repeated `/`.
pub fn normalize_design_path(raw: &str) -> String {
    let trimmed = raw.trim();
    let unbacked = trimmed.trim_matches('`');
    let slashed = unbacked.replace('\\', "/");
    let mut out = String::with_capacity(slashed.len());
    let mut previous_slash = false;
    let mut chars = slashed.chars().peekable();
    // Drop a leading "./" once.
    if chars.peek() == Some(&'.') {
        let mut clone = chars.clone();
        clone.next();
        if clone.peek() == Some(&'/') {
            chars.next();
            chars.next();
        }
    }
    for ch in chars {
        if ch == '/' && previous_slash {
            continue;
        }
        previous_slash = ch == '/';
        out.push(ch);
    }
    out
}

/// Parses `design.md`. Never panics; problems become findings.
pub fn parse(text: &str, file: &str) -> DesignDoc {
    let scanned = scan(text, file);
    let mut doc = DesignDoc {
        file: file.to_owned(),
        outline: scanned.outline.clone(),
        fences: scanned.fences.clone(),
        mermaid: scanned
            .fences
            .iter()
            .filter(|f| f.info.trim().eq_ignore_ascii_case("mermaid"))
            .map(|f| MermaidBlock {
                start: f.start,
                end: f.end,
                body: f.body.clone(),
            })
            .collect(),
        components: Vec::new(),
        files: Vec::new(),
        findings: scanned.findings,
    };

    let plan = region(&scanned.outline, &scanned.lines, "file structure plan");
    let components_region = region(
        &scanned.outline,
        &scanned.lines,
        "components and interfaces",
    );

    // Pass 1: register components.
    // (a) level-4 headings inside the Components region.
    if let Some((start, end)) = components_region {
        for line in lines_between(&scanned.lines, start, end) {
            if let LineKind::Heading { level: 4, text } = line.kind {
                let name = text.trim();
                if !name.is_empty() {
                    register_component(&mut doc, name, line.line);
                }
            }
        }
    }
    // (b) the `Component` column of any table in the document.
    for table in tables_of(&scanned.lines) {
        if let Some(col) = table.header_column("component") {
            for row in table.data_rows() {
                let Some(cell) = row_cells(row)
                    .and_then(|(cells, _)| cells.get(col))
                    .map(|c| c.trim())
                else {
                    continue;
                };
                if !cell.is_empty() {
                    let line = row.line;
                    register_component(&mut doc, cell, line);
                }
            }
        }
    }

    // Pass 2: mappings inside the plan region (tree shape is task 2.5).
    if let Some((start, end)) = plan {
        for line in lines_between(&scanned.lines, start, end) {
            if let LineKind::ListItem { text, .. } = line.kind
                && let Some((name, paths)) = bullet_mapping(text)
            {
                for raw in paths {
                    let path = normalize_design_path(&raw);
                    if path.is_empty() {
                        continue;
                    }
                    let line_no = line.line;
                    register_component(&mut doc, name.trim(), line_no);
                    add_file(&mut doc, &path, Some(name.trim()), None, line_no);
                }
            }
        }
        // Mapping tables inside the plan.
        for table in tables_between(&scanned.lines, start, end) {
            let Some(comp_col) = table.header_column("component") else {
                continue;
            };
            let Some(file_col) = file_column(&table) else {
                continue;
            };
            let resp_col = table.header_column("responsibility");
            for row in table.data_rows() {
                let Some((cells, _)) = row_cells(row) else {
                    continue;
                };
                let owner = cells.get(comp_col).map(|c| c.trim()).unwrap_or_default();
                let cell = cells.get(file_col).map(|c| c.trim()).unwrap_or_default();
                let responsibility = resp_col
                    .and_then(|col| cells.get(col).map(|c| c.trim()))
                    .filter(|r| !r.is_empty())
                    .map(str::to_owned);
                for raw in backticked_or_whole(cell) {
                    let path = normalize_design_path(&raw);
                    if path.is_empty() {
                        continue;
                    }
                    let owner_opt = if owner.is_empty() {
                        None
                    } else {
                        register_component(&mut doc, owner, row.line);
                        Some(owner)
                    };
                    add_file(&mut doc, &path, owner_opt, responsibility.clone(), row.line);
                }
            }
        }

        // Fenced directory trees inside the plan (task 2.5), merged into the
        // mappings by path regardless of document order.
        let fences = doc.fences.clone();
        for fence in &fences {
            if fence.start >= start && fence.end <= end {
                parse_tree(&mut doc, fence);
            }
        }
    } else {
        doc.findings.push(Finding::new(
            codes::DESIGN_NO_FILE_PLAN,
            file,
            "design document has no File Structure Plan section",
        ));
    }

    // Findings: components without files, files without a component.
    for component in &doc.components {
        if component.files.is_empty() {
            doc.findings.push(
                Finding::new(
                    codes::DESIGN_COMPONENT_NO_FILES,
                    file,
                    format!("component {} has no files in the plan", component.name),
                )
                .at(component.line),
            );
        }
    }
    let unassigned: Vec<(String, usize)> = doc
        .files
        .iter()
        .filter(|f| f.component.is_none())
        .map(|f| (f.path.clone(), f.line))
        .collect();
    for (path, line) in unassigned {
        doc.findings.push(
            Finding::new(
                codes::DESIGN_FILE_UNASSIGNED,
                file,
                format!("file {path} appears in the plan without a component"),
            )
            .at(line),
        );
    }
    doc
}

/// The line span of a region opened by a heading with the given trimmed,
/// case-insensitive text (trailing colon ignored), ending at the next heading
/// of equal or higher level. Returns `(first_line_inside, last_line_inside)`.
fn region(outline: &[Heading], lines: &[ScannedLine<'_>], name: &str) -> Option<(usize, usize)> {
    let heading = outline.iter().find(|h| {
        h.text
            .trim()
            .trim_end_matches(':')
            .eq_ignore_ascii_case(name)
    })?;
    let start = heading.line + 1;
    let end = lines
        .iter()
        .filter(|l| l.line > heading.line)
        .find_map(|l| match l.kind {
            LineKind::Heading { level, text: _ } if level <= heading.level => Some(l.line - 1),
            _ => None,
        })
        .unwrap_or_else(|| lines.last().map(|l| l.line).unwrap_or(start));
    Some((start, end))
}

fn lines_between<'a>(
    lines: &'a [ScannedLine<'a>],
    start: usize,
    end: usize,
) -> impl Iterator<Item = &'a ScannedLine<'a>> {
    lines
        .iter()
        .filter(move |l| l.line >= start && l.line <= end)
}

/// Registers a component; the first occurrence wins.
fn register_component(doc: &mut DesignDoc, name: &str, line: usize) {
    if doc.component(name).is_none() {
        doc.components.push(Component {
            name: name.trim().to_owned(),
            line,
            files: Vec::new(),
        });
    }
}

/// Appends a file and links it into its component's file list.
fn add_file(
    doc: &mut DesignDoc,
    path: &str,
    component: Option<&str>,
    responsibility: Option<String>,
    line: usize,
) {
    if let Some(name) = component
        && let Some(c) = doc.components.iter_mut().find(|c| c.name == name)
        && !c.files.iter().any(|f| f == path)
    {
        c.files.push(path.to_owned());
    }
    doc.files.push(DesignFile {
        path: path.to_owned(),
        component: component.map(str::to_owned),
        responsibility,
        line,
    });
}

/// One table with its header row and data rows.
struct Table<'a> {
    header: &'a ScannedLine<'a>,
    rows: Vec<&'a ScannedLine<'a>>,
}

impl<'a> Table<'a> {
    fn header_column(&self, name: &str) -> Option<usize> {
        let (header, _) = row_cells(self.header)?;
        header
            .iter()
            .position(|c| c.trim().eq_ignore_ascii_case(name))
    }

    fn data_rows(&self) -> impl Iterator<Item = &'a ScannedLine<'a>> + '_ {
        self.rows.iter().copied()
    }
}

/// The cells and separator flag of a table-row line.
fn row_cells<'a, 'b>(line: &'b ScannedLine<'a>) -> Option<(&'b [&'a str], bool)> {
    match &line.kind {
        LineKind::TableRow { cells, separator } => Some((cells.as_slice(), *separator)),
        _ => None,
    }
}

fn file_column(table: &Table<'_>) -> Option<usize> {
    for name in ["file", "files", "path"] {
        if let Some(col) = table.header_column(name) {
            return Some(col);
        }
    }
    None
}

/// Every table in the document (a header row followed by rows until a
/// non-table line).
fn tables_of<'a>(lines: &'a [ScannedLine<'a>]) -> Vec<Table<'a>> {
    tables_between(lines, 1, usize::MAX)
}

fn tables_between<'a>(lines: &'a [ScannedLine<'a>], start: usize, end: usize) -> Vec<Table<'a>> {
    let mut tables = Vec::new();
    let mut current: Option<Table<'a>> = None;
    for line in lines_between(lines, start, end) {
        match (current.as_mut(), row_cells(line)) {
            (None, Some((_, false))) => {
                current = Some(Table {
                    header: line,
                    rows: Vec::new(),
                });
            }
            (Some(table), Some((_, false))) => table.rows.push(line),
            // Separator rows belong to the table shape but carry no data.
            (Some(_), Some((_, true))) => {}
            (Some(_), None) => {
                if let Some(done) = current.take()
                    && !done.rows.is_empty()
                {
                    tables.push(done);
                }
            }
            (None, Some((_, true))) => {}
            (None, None) => {}
        }
    }
    if let Some(done) = current.take()
        && !done.rows.is_empty()
    {
        tables.push(done);
    }
    tables
}

/// `- **Name**: `path` `path`` bullets.
fn bullet_mapping(text: &str) -> Option<(String, Vec<String>)> {
    let rest = text.trim().strip_prefix("**")?;
    let (name, after) = rest.split_once("**")?;
    let after = after.trim_start();
    let after = after.strip_prefix(':').unwrap_or(after);
    let paths: Vec<String> = backticked(after).into_iter().map(str::to_owned).collect();
    if paths.is_empty() {
        return None;
    }
    Some((name.trim().to_owned(), paths))
}

/// Parses one fenced directory tree inside the plan region and merges its
/// files into the existing mappings by path (task 2.5): a tree file already
/// named by a mapping only gains a responsibility when the mapping has none;
/// the remaining tree files are appended unassigned.
fn parse_tree(doc: &mut DesignDoc, fence: &Fence) {
    let body_lines: Vec<&str> = fence.body.lines().collect();
    let has_glyphs = fence
        .body
        .chars()
        .any(|c| matches!(c, '│' | '├' | '└' | '─'));
    // Unit of depth: 4 columns with tree glyphs, else the smallest non-zero
    // name column in the block.
    let name_cols: Vec<usize> = body_lines.iter().map(|l| name_column(l)).collect();
    let unit = if has_glyphs {
        4
    } else {
        name_cols
            .iter()
            .copied()
            .filter(|c| *c > 0)
            .min()
            .unwrap_or(1)
    };
    let mut dirs: Vec<(usize, String)> = Vec::new();
    for (idx, raw_line) in body_lines.iter().enumerate() {
        let line_no = fence.start + 1 + idx;
        let name_col = name_cols.get(idx).copied().unwrap_or(0);
        let depth = name_col / unit.max(1);
        let entry: String = raw_line.chars().skip(name_col).collect();
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, comment) = match entry.split_once('#') {
            Some((n, c)) => (n.trim(), Some(c.trim().to_owned())),
            None => (entry, None),
        };
        if name.is_empty() {
            continue;
        }
        // Keep the directory stack consistent with this depth.
        while dirs.last().is_some_and(|(d, _)| *d >= depth) {
            dirs.pop();
        }
        if let Some(dir_name) = name.strip_suffix('/') {
            if !dir_name.is_empty() {
                dirs.push((depth, dir_name.to_owned()));
            }
            continue;
        }
        let mut path = String::new();
        for (_, segment) in &dirs {
            path.push_str(segment);
            path.push('/');
        }
        path.push_str(name);
        let path = normalize_design_path(&path);
        // Merge with an existing mapping of the same path, else append.
        if let Some(existing) = doc.files.iter_mut().find(|f| f.path == path) {
            if existing.responsibility.is_none() {
                existing.responsibility = comment;
            }
        } else {
            doc.files.push(DesignFile {
                path,
                component: None,
                responsibility: comment,
                line: line_no,
            });
        }
    }
}

/// The char column of the first character that is neither a space nor a tree
/// glyph.
fn name_column(line: &str) -> usize {
    line.chars()
        .take_while(|c| *c == ' ' || matches!(c, '│' | '├' | '└' | '─'))
        .count()
}
fn backticked_or_whole(cell: &str) -> Vec<String> {
    let ticks = backticked(cell);
    if ticks.is_empty() {
        let whole = cell.trim();
        if whole.is_empty() {
            Vec::new()
        } else {
            vec![whole.to_owned()]
        }
    } else {
        ticks.into_iter().map(str::to_owned).collect()
    }
}

fn backticked(text: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        if let Some(end) = after.find('`') {
            parts.push(&after[..end]);
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    parts
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::design::{normalize_design_path, parse};
    use crate::finding::{Severity, codes};

    const F: &str = ".kiro/specs/demo/design.md";
    const TABLE_PLAN: &str = include_str!("../fixtures/design/table-plan.md");
    const BULLET_PLAN: &str = include_str!("../fixtures/design/bullet-plan.md");
    const NO_PLAN: &str = include_str!("../fixtures/design/no-plan.md");
    const TREE_PLAN: &str = include_str!("../fixtures/design/tree-plan.md");
    const DEMO: &str = include_str!("../fixtures/minimal/.kiro/specs/demo/design.md");
    const SELF: &str = include_str!("../fixtures/self/design.md");

    #[test]
    fn table_shape_with_multi_path_cells() {
        let doc = parse(TABLE_PLAN, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.components.len(), 2);
        let alpha = doc
            .component("alpha")
            .cloned()
            .unwrap_or_else(|| panic!("Alpha"));
        assert_eq!(
            alpha.files,
            ["src/a.rs", "src/b.rs"],
            "multi-path cell split"
        );
        let beta = doc
            .component("Beta")
            .cloned()
            .unwrap_or_else(|| panic!("Beta"));
        assert_eq!(beta.files, ["src/c.rs"]);
        assert_eq!(doc.files.len(), 3);
        let file = doc.files.first().cloned().unwrap_or_else(|| panic!("file"));
        assert_eq!(
            (file.path.as_str(), file.component.as_deref(), file.line),
            ("src/a.rs", Some("Alpha"), 7)
        );
    }

    #[test]
    fn bullet_shape_with_backticked_paths() {
        let doc = parse(BULLET_PLAN, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        let alpha = doc
            .component("Alpha")
            .cloned()
            .unwrap_or_else(|| panic!("Alpha"));
        assert_eq!(alpha.files, ["src/a.rs", "src/b.rs"]);
        let beta = doc
            .component("Beta")
            .cloned()
            .unwrap_or_else(|| panic!("Beta"));
        assert_eq!(beta.files, ["src/c.rs"]);
    }

    #[test]
    fn missing_plan_warns() {
        let doc = parse(NO_PLAN, F);
        assert!(matches!(
            doc.findings.first(),
            Some(f) if f.code == codes::DESIGN_NO_FILE_PLAN && f.severity == Severity::Warning
        ));
        assert!(doc.files.is_empty());
        assert!(doc.components.is_empty());
    }

    #[test]
    fn demo_fixture_plan_components_and_mermaid_block() {
        let doc = parse(DEMO, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.components.len(), 2);
        let core = doc
            .component("core")
            .cloned()
            .unwrap_or_else(|| panic!("Core"));
        assert_eq!(core.files, ["src/demo.rs"]);
        let ui = doc.component("Ui").cloned().unwrap_or_else(|| panic!("Ui"));
        assert_eq!(ui.files, ["src/ui.rs"]);
        let mermaid = doc
            .mermaid
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("mermaid block"));
        assert_eq!((mermaid.start, mermaid.end), (22, 27));
        assert!(mermaid.body.contains("graph LR"));
        assert_eq!(
            doc.files_of("CORE").count(),
            1,
            "lookup is trimmed and case-insensitive"
        );
        assert_eq!(doc.unassigned_files().count(), 0);
    }

    #[test]
    fn responsibility_column_is_recorded() {
        let text = "# D\n\n## File Structure Plan\n\n| Component | File | Responsibility |\n|---|---|---|\n| Alpha | `src/a.rs` | Owns the query |\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        let file = doc.files.first().cloned().unwrap_or_else(|| panic!("file"));
        assert_eq!(file.responsibility.as_deref(), Some("Owns the query"));
    }

    #[test]
    fn components_from_headings_and_summary_tables_and_no_files_finding() {
        // A level-4 heading in the Components region registers a component;
        // so does a summary table's Component column; both without files
        // yield the info finding at their recorded line.
        let text = "# D\n\n## Components and Interfaces\n\n#### Lonely\n\nProse.\n\n| Component | Domain |\n|---|---|\n| SummaryOnly | Engine |\n\n## File Structure Plan\n\n| Component | File |\n|---|---|\n| Alpha | `src/a.rs` |\n";
        let doc = parse(text, F);
        assert!(
            doc.component("Lonely").is_some(),
            "level-4 heading registers"
        );
        assert!(
            doc.component("SummaryOnly").is_some(),
            "summary table registers"
        );
        assert!(doc.component("Alpha").is_some());
        let no_files: Vec<(String, usize)> = doc
            .findings
            .iter()
            .filter(|f| f.code == codes::DESIGN_COMPONENT_NO_FILES)
            .map(|f| (f.message.clone(), f.line.unwrap_or(0)))
            .collect();
        assert_eq!(no_files.len(), 2, "{:?}", doc.findings);
        assert!(no_files.iter().all(|(_, line)| *line > 0));
        assert!(doc.findings.iter().all(|f| f.severity == Severity::Info));
    }

    #[test]
    fn empty_component_cell_yields_unassigned_file() {
        let text = "# D\n\n## File Structure Plan\n\n| Component | File |\n|---|---|\n|  | `src/free.rs` |\n| Alpha | `src/a.rs` |\n";
        let doc = parse(text, F);
        let unassigned: Vec<&crate::design::DesignFile> = doc.unassigned_files().collect();
        assert_eq!(unassigned.len(), 1);
        assert_eq!(
            unassigned.first().map(|f| f.path.as_str()),
            Some("src/free.rs")
        );
        assert!(matches!(
            doc.findings.first(),
            Some(f) if f.code == codes::DESIGN_FILE_UNASSIGNED && f.severity == Severity::Info
        ));
    }

    #[test]
    fn path_normalisation() {
        assert_eq!(normalize_design_path("`src\\a.rs`"), "src/a.rs");
        assert_eq!(normalize_design_path("./src//a.rs"), "src/a.rs");
        assert_eq!(normalize_design_path("src/a.rs/"), "src/a.rs/");
        assert_eq!(normalize_design_path("  `./x/y.rs`  "), "x/y.rs");
    }

    #[test]
    fn self_design_fixture_table_derived_set() {
        // Task 2.4 observable: the self design fixture parses to the
        // table-derived set only; every component has at least one file and
        // nothing is unassigned (tree support and its extra files are 2.5).
        let doc = parse(SELF, F);
        assert!(!doc.components.is_empty());
        assert!(!doc.files.is_empty());
        for c in &doc.components {
            assert!(
                !c.files.is_empty(),
                "component {} must own at least one file",
                c.name
            );
        }
        assert_eq!(
            doc.unassigned_files().count(),
            0,
            "table rows all name a component"
        );
        let no_files: Vec<&crate::finding::Finding> = doc
            .findings
            .iter()
            .filter(|f| f.code == codes::DESIGN_COMPONENT_NO_FILES)
            .collect();
        assert!(no_files.is_empty(), "{:?}", no_files);
        assert!(
            !doc.mermaid.is_empty(),
            "the self design contains mermaid blocks"
        );
    }

    #[test]
    fn plan_region_ends_at_next_heading_of_equal_or_higher_level() {
        // A table after a deeper subsection still belongs to the plan; a
        // table after an equal-level heading does not.
        let text = "# D\n\n## File Structure Plan\n\n| Component | File |\n|---|---|\n| Alpha | `src/a.rs` |\n\n### Notes\n\n| Component | File |\n|---|---|\n| Beta | `src/b.rs` |\n\n## After\n\n| Component | File |\n|---|---|\n| Gamma | `src/c.rs` |\n";
        let doc = parse(text, F);
        // Alpha and Beta register as components with files (their tables are
        // inside the plan region); Gamma registers as a component (any table)
        // but gets no files from a table outside the plan.
        assert!(doc.component("Alpha").is_some_and(|c| !c.files.is_empty()));
        assert!(doc.component("Beta").is_some_and(|c| !c.files.is_empty()));
        assert!(doc.component("Gamma").is_some());
        assert!(doc.component("Gamma").is_some_and(|c| c.files.is_empty()));
    }

    // ----- Task 2.5: fenced directory-tree shape and merge -----

    #[test]
    fn tree_fixture_paths_responsibilities_and_partial_merge() {
        let doc = parse(TREE_PLAN, F);
        // Tree files with full paths; `src/main.rs` merges into the table's
        // Entry component without a duplicate; the rest stay unassigned.
        let paths: Vec<(String, Option<String>, Option<String>)> = doc
            .files
            .iter()
            .map(|f| {
                (
                    f.path.clone(),
                    f.component.clone(),
                    f.responsibility.clone(),
                )
            })
            .collect();
        assert_eq!(
            paths,
            vec![
                (
                    "demo/src/main.rs".into(),
                    Some("Entry".into()),
                    Some("entry point, assigned below".into())
                ),
                ("demo/src/lib.rs".into(), None, Some("not assigned".into())),
                (
                    "demo/tests/basic.rs".into(),
                    None,
                    Some("not assigned".into())
                ),
            ]
        );
        let unassigned: Vec<&str> = doc.unassigned_files().map(|f| f.path.as_str()).collect();
        assert_eq!(unassigned, ["demo/src/lib.rs", "demo/tests/basic.rs"]);
        let findings: Vec<usize> = doc
            .findings
            .iter()
            .filter(|f| f.code == codes::DESIGN_FILE_UNASSIGNED)
            .map(|f| f.line.unwrap_or(0))
            .collect();
        assert_eq!(findings.len(), 2, "{:?}", doc.findings);
        assert!(
            doc.component("Entry")
                .is_some_and(|c| c.files == ["demo/src/main.rs"])
        );
    }

    #[test]
    fn tree_without_glyphs_uses_smallest_indent_as_unit() {
        let text =
            "# D\n\n## File Structure Plan\n\n```\nroot/\n  alpha.rs\n  sub/\n    beta.rs\n```\n";
        let doc = parse(text, F);
        let paths: Vec<&str> = doc.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["root/alpha.rs", "root/sub/beta.rs"]);
    }

    #[test]
    fn merge_works_in_both_orders_and_keeps_mapping_responsibility() {
        // Tree first, mapping after.
        let tree_first = "# D\n\n## File Structure Plan\n\n```\nsrc/\n  a.rs # tree comment\n```\n\n| Component | File | Responsibility |\n|---|---|---|\n| Alpha | `src/a.rs` | Mapping owns it |\n";
        let doc = parse(tree_first, F);
        assert_eq!(doc.files.len(), 1, "merged into one entry");
        let f = doc.files.first().cloned().unwrap_or_else(|| panic!("file"));
        assert_eq!(
            (f.path.as_str(), f.component.as_deref()),
            ("src/a.rs", Some("Alpha"))
        );
        assert_eq!(
            f.responsibility.as_deref(),
            Some("Mapping owns it"),
            "mapping responsibility wins"
        );

        // Mapping first, tree after (the tree-plan fixture order): the tree
        // comment fills a missing responsibility.
        let mapping_first = "# D\n\n## File Structure Plan\n\n| Component | File |\n|---|---|\n| Alpha | `src/a.rs` |\n\n```\nsrc/\n  a.rs # tree comment\n```\n";
        let doc = parse(mapping_first, F);
        assert_eq!(doc.files.len(), 1);
        let f = doc.files.first().cloned().unwrap_or_else(|| panic!("file"));
        assert_eq!(
            (f.component.as_deref(), f.responsibility.as_deref()),
            (Some("Alpha"), Some("tree comment"))
        );
    }

    #[test]
    fn directories_are_skipped_as_files_and_comments_become_responsibilities() {
        let text = "# D\n\n## File Structure Plan\n\n```text\nproj/\n├── src/          # sources\n│   └── mod.rs    # the module\n└── README.md\n```\n";
        let doc = parse(text, F);
        let paths: Vec<(String, Option<String>)> = doc
            .files
            .iter()
            .map(|f| (f.path.clone(), f.responsibility.clone()))
            .collect();
        assert_eq!(
            paths,
            vec![
                ("proj/src/mod.rs".into(), Some("the module".into())),
                ("proj/README.md".into(), None),
            ]
        );
    }

    #[test]
    fn trees_outside_the_plan_region_are_ignored() {
        let text = "# D\n\n## File Structure Plan\n\n| Component | File |\n|---|---|\n| Alpha | `src/a.rs` |\n\n## Diagrams\n\n```\nother/\n  tree.rs\n```\n";
        let doc = parse(text, F);
        assert_eq!(doc.files.len(), 1);
        assert_eq!(doc.unassigned_files().count(), 0);
    }
}
