//! Duplicate blocks (clone-constraints 1.1).
//!
//! A duplicate block is a run of at least `min_lines` consecutive code
//! lines that the project holds at another location. A code line is the
//! line's tokens with whitespace and comments removed; lines are compared
//! by hash and a match is confirmed line by line. Lines without a token do
//! not count and do not interrupt a run.
//!
//! The line hashes come from the native crate, which takes the tokens from
//! the same syntax tree the metrics use. This module is pure: it finds the
//! runs and words the findings.

use std::collections::{BTreeMap, BTreeSet};

use super::report::{CodeFinding, Tag};
use crate::finding::{Finding, Severity, codes};

/// Shortest run reported.
pub const DEFAULT_MIN_LINES: usize = 12;
/// Most other locations named in one finding.
const MAX_OTHERS: usize = 3;
/// A window that occurs more often than this is not expanded: such blocks
/// are tables or generated code and would cost quadratic work.
const MAX_OCCURRENCES: usize = 64;

/// One code line of a file: its 1-based line number and the hash of its
/// normalized tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineHash {
    pub line: u32,
    pub hash: u64,
}

/// The code lines of one file, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLines {
    pub path: String,
    pub lines: Vec<LineHash>,
}

/// A run of lines in a file, both ends inclusive, 1-based.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CodeRange {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
}

/// Two locations holding the same run of code lines.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DuplicateBlock {
    pub first: CodeRange,
    pub second: CodeRange,
    /// Number of code lines in the run.
    pub lines: usize,
}

/// What the duplicate search covered.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CloneCoverage {
    pub files: usize,
    /// Windows left unexpanded because they occur too often.
    pub skipped_windows: usize,
}

fn window_hash(lines: &[LineHash]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for line in lines {
        for byte in line.hash.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn same(files: &[FileLines], a: (usize, usize), b: (usize, usize), count: usize) -> bool {
    let left = files
        .get(a.0)
        .and_then(|file| file.lines.get(a.1..a.1.saturating_add(count)));
    let right = files
        .get(b.0)
        .and_then(|file| file.lines.get(b.1..b.1.saturating_add(count)));
    match (left, right) {
        (Some(left), Some(right)) => left.iter().zip(right.iter()).all(|(x, y)| x.hash == y.hash),
        _ => false,
    }
}

fn range(files: &[FileLines], at: (usize, usize), count: usize) -> Option<CodeRange> {
    let file = files.get(at.0)?;
    let first = file.lines.get(at.1)?;
    let last = file.lines.get(at.1.saturating_add(count).checked_sub(1)?)?;
    Some(CodeRange {
        path: file.path.clone(),
        start_line: first.line,
        end_line: last.line,
    })
}

/// Where a window starts: the index of its file and of its first code line.
type Position = (usize, usize);
/// Two windows holding the same lines, the earlier position first.
type WindowPair = (Position, Position);

/// Every window of `min_lines` lines, by hash.
fn windows_by_hash(files: &[FileLines], min_lines: usize) -> BTreeMap<u64, Vec<Position>> {
    let mut windows: BTreeMap<u64, Vec<Position>> = BTreeMap::new();
    for (file_index, file) in files.iter().enumerate() {
        for (start, window) in file.lines.windows(min_lines).enumerate() {
            windows
                .entry(window_hash(window))
                .or_default()
                .push((file_index, start));
        }
    }
    windows
}

/// Adds the pairs of `positions` that really are equal. Two windows in one
/// file may not overlap.
fn insert_equal(
    files: &[FileLines],
    positions: &[Position],
    min_lines: usize,
    pairs: &mut BTreeSet<WindowPair>,
) {
    for (index, a) in positions.iter().enumerate() {
        for b in positions.iter().skip(index.saturating_add(1)) {
            let overlapping = a.0 == b.0 && b.1 < a.1.saturating_add(min_lines);
            if !overlapping && same(files, *a, *b, min_lines) {
                pairs.insert((*a, *b));
            }
        }
    }
}

/// Pairs of windows that really are equal. A window that occurs too often
/// is counted in `coverage` and left out.
fn equal_pairs(
    files: &[FileLines],
    min_lines: usize,
    coverage: &mut CloneCoverage,
) -> BTreeSet<WindowPair> {
    let mut pairs = BTreeSet::new();
    for positions in windows_by_hash(files, min_lines).values() {
        if positions.len() < 2 {
            continue;
        }
        if positions.len() > MAX_OCCURRENCES {
            coverage.skipped_windows = coverage.skipped_windows.saturating_add(positions.len());
            continue;
        }
        insert_equal(files, positions, min_lines, &mut pairs);
    }
    pairs
}

/// Number of code lines of the run that starts at the window pair. A run
/// starts where the preceding window pair does not match; it is extended
/// while the following pairs do. `None` when the pair continues a run, or
/// the run is shorter than `min_lines`.
fn run_length(pairs: &BTreeSet<WindowPair>, pair: WindowPair, min_lines: usize) -> Option<usize> {
    let (a, b) = pair;
    let preceded = a.1 > 0
        && b.1 > 0
        && pairs.contains(&((a.0, a.1.saturating_sub(1)), (b.0, b.1.saturating_sub(1))));
    if preceded {
        return None;
    }
    let mut steps = 0_usize;
    while pairs.contains(&(
        (a.0, a.1.saturating_add(steps).saturating_add(1)),
        (b.0, b.1.saturating_add(steps).saturating_add(1)),
    )) {
        steps = steps.saturating_add(1);
    }
    let mut count = min_lines.saturating_add(steps);
    // Two runs in one file end where the second begins.
    if a.0 == b.0 {
        count = count.min(b.1.saturating_sub(a.1));
    }
    (count >= min_lines).then_some(count)
}

/// Finds every maximal run of at least `min_lines` equal code lines at
/// two locations. Two runs in one file may not overlap: a run that repeats
/// only itself is not reported.
#[must_use]
pub fn duplicate_blocks(
    files: &[FileLines],
    min_lines: usize,
) -> (Vec<DuplicateBlock>, CloneCoverage) {
    let mut coverage = CloneCoverage {
        files: files.len(),
        skipped_windows: 0,
    };
    if min_lines == 0 {
        return (Vec::new(), coverage);
    }
    let pairs = equal_pairs(files, min_lines, &mut coverage);
    let mut blocks = Vec::new();
    for (a, b) in &pairs {
        let Some(count) = run_length(&pairs, (*a, *b), min_lines) else {
            continue;
        };
        if let (Some(first), Some(second)) = (range(files, *a, count), range(files, *b, count)) {
            blocks.push(DuplicateBlock {
                first,
                second,
                lines: count,
            });
        }
    }
    blocks.sort();
    blocks.dedup();
    (blocks, coverage)
}

fn location(range: &CodeRange) -> String {
    format!("{}:{}-{}", range.path, range.start_line, range.end_line)
}

/// The finding for the range `here`, which also stands at `there`.
fn duplicate_finding(here: &CodeRange, there: &BTreeSet<&CodeRange>, lines: usize) -> CodeFinding {
    let named: Vec<String> = there
        .iter()
        .take(MAX_OTHERS)
        .map(|range| location(range))
        .collect();
    let more = there.len().saturating_sub(named.len());
    let suffix = if more > 0 {
        format!(" and {more} more")
    } else {
        String::new()
    };
    let mut finding = Finding::new(
        codes::CODE_DUPLICATE_BLOCK,
        here.path.clone(),
        format!(
            "lines {}-{} ({lines} code lines) also stand at {}{suffix}",
            here.start_line,
            here.end_line,
            named.join(", ")
        ),
    )
    .severity(Severity::Warning);
    finding.line = usize::try_from(here.start_line).ok();
    let hint = format!(
        "keep one copy: move the block into one routine and call it from {}",
        named.first().map_or("the other location", String::as_str)
    );
    CodeFinding::across_files(finding, Tag::Delete, hint)
}

/// Words one finding per duplicated range in a selected file, naming up to
/// three other locations. When both ends of a block are selected, both are
/// reported, each from its own side.
#[must_use]
pub fn duplicate_findings(
    blocks: &[DuplicateBlock],
    selected: &dyn Fn(&str) -> bool,
) -> Vec<CodeFinding> {
    // The other locations of every range.
    let mut others: BTreeMap<&CodeRange, (BTreeSet<&CodeRange>, usize)> = BTreeMap::new();
    for block in blocks {
        for (here, there) in [(&block.first, &block.second), (&block.second, &block.first)] {
            let entry = others
                .entry(here)
                .or_insert_with(|| (BTreeSet::new(), block.lines));
            entry.0.insert(there);
        }
    }
    others
        .into_iter()
        .filter(|(here, _)| selected(&here.path))
        .map(|(here, (there, lines))| duplicate_finding(here, &there, lines))
        .collect()
}
