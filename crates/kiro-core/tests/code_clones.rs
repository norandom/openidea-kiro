//! Duplicate blocks (clone-constraints 1.1): runs of equal code lines.
//! The expected blocks were decided by hand from the line sequences.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_core::code::Tag;
use kiro_core::code::clones::{
    CodeRange, DuplicateBlock, FileLines, LineHash, duplicate_blocks, duplicate_findings,
};

/// A file whose code lines carry the given hashes on consecutive lines
/// starting at `first_line`.
fn file(path: &str, first_line: u32, hashes: &[u64]) -> FileLines {
    FileLines {
        path: path.to_string(),
        lines: hashes
            .iter()
            .enumerate()
            .map(|(index, hash)| LineHash {
                line: first_line + u32::try_from(index).unwrap(),
                hash: *hash,
            })
            .collect(),
    }
}

fn run(from: u64, count: u64) -> Vec<u64> {
    (from..from + count).collect()
}

fn range(path: &str, start: u32, end: u32) -> CodeRange {
    CodeRange {
        path: path.to_string(),
        start_line: start,
        end_line: end,
    }
}

#[test]
fn twelve_equal_lines_are_a_block_and_eleven_are_not() {
    let twelve = [
        file("a.rs", 1, &run(100, 12)),
        file("b.rs", 40, &run(100, 12)),
    ];
    let (blocks, coverage) = duplicate_blocks(&twelve, 12);
    assert_eq!(
        blocks,
        [DuplicateBlock {
            first: range("a.rs", 1, 12),
            second: range("b.rs", 40, 51),
            lines: 12,
        }]
    );
    assert_eq!(coverage.files, 2);

    let eleven = [
        file("a.rs", 1, &run(100, 11)),
        file("b.rs", 40, &run(100, 11)),
    ];
    assert!(duplicate_blocks(&eleven, 12).0.is_empty());
}

#[test]
fn a_longer_run_is_one_maximal_block() {
    // 3 different lines, then 15 shared lines, then different lines again.
    let mut a = run(1, 3);
    a.extend(run(100, 15));
    a.extend(run(10, 2));
    let mut b = run(50, 5);
    b.extend(run(100, 15));
    b.extend(run(60, 1));
    let (blocks, _) = duplicate_blocks(&[file("a.rs", 1, &a), file("b.rs", 1, &b)], 12);
    assert_eq!(
        blocks,
        [DuplicateBlock {
            first: range("a.rs", 4, 18),
            second: range("b.rs", 6, 20),
            lines: 15,
        }]
    );
}

#[test]
fn two_copies_in_one_file_count_and_a_run_that_repeats_only_itself_does_not() {
    // Copy at lines 1-12 and again at lines 21-32 of the same file.
    let mut twice = run(100, 12);
    twice.extend(run(1, 8));
    twice.extend(run(100, 12));
    let (blocks, _) = duplicate_blocks(&[file("a.rs", 1, &twice)], 12);
    assert_eq!(
        blocks,
        [DuplicateBlock {
            first: range("a.rs", 1, 12),
            second: range("a.rs", 21, 32),
            lines: 12,
        }]
    );

    // Twenty identical lines: every window equals its neighbour, but no two
    // windows of twelve lines fit side by side.
    let same = vec![7_u64; 20];
    assert!(duplicate_blocks(&[file("a.rs", 1, &same)], 12).0.is_empty());

    // Twenty-four identical lines: two windows fit, and they do not overlap.
    let same = vec![7_u64; 24];
    let (blocks, _) = duplicate_blocks(&[file("a.rs", 1, &same)], 12);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].first, range("a.rs", 1, 12));
    assert_eq!(blocks[0].second, range("a.rs", 13, 24));
}

#[test]
fn line_numbers_follow_the_code_lines_across_gaps() {
    // The same twelve code lines, in b.rs with blank and comment lines
    // between them: every second line number.
    let a = file("a.rs", 1, &run(100, 12));
    let b = FileLines {
        path: "b.rs".to_string(),
        lines: run(100, 12)
            .iter()
            .enumerate()
            .map(|(index, hash)| LineHash {
                line: 10 + 2 * u32::try_from(index).unwrap(),
                hash: *hash,
            })
            .collect(),
    };
    let (blocks, _) = duplicate_blocks(&[a, b], 12);
    assert_eq!(blocks[0].second, range("b.rs", 10, 32));
    assert_eq!(blocks[0].lines, 12);
}

#[test]
fn findings_name_up_to_three_other_locations_from_each_selected_side() {
    let files: Vec<FileLines> = ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"]
        .iter()
        .map(|path| file(path, 1, &run(100, 12)))
        .collect();
    let (blocks, _) = duplicate_blocks(&files, 12);
    assert_eq!(blocks.len(), 10, "five locations make ten pairs");

    let only_a = duplicate_findings(&blocks, &|path| path == "a.rs");
    assert_eq!(only_a.len(), 1);
    let finding = &only_a[0];
    assert_eq!(finding.finding.code, "CODE_DUPLICATE_BLOCK");
    assert_eq!(finding.finding.file, "a.rs");
    assert_eq!(finding.finding.line, Some(1));
    assert_eq!(
        finding.finding.message,
        "lines 1-12 (12 code lines) also stand at b.rs:1-12, c.rs:1-12, d.rs:1-12 and 1 more"
    );
    assert_eq!(finding.tag, Tag::Delete);
    assert!(finding.hint.contains("b.rs:1-12"));

    // Both ends selected: each is reported from its own side.
    let two = duplicate_findings(&blocks, &|path| path == "a.rs" || path == "b.rs");
    let files: Vec<&str> = two.iter().map(|f| f.finding.file.as_str()).collect();
    assert_eq!(files, ["a.rs", "b.rs"]);
    assert!(two[1].finding.message.contains("a.rs:1-12"));

    assert!(duplicate_findings(&blocks, &|_| false).is_empty());
}

#[test]
fn a_window_that_occurs_very_often_is_skipped_and_counted() {
    let files: Vec<FileLines> = (0..70)
        .map(|index| file(&format!("f{index:02}.rs"), 1, &run(100, 12)))
        .collect();
    let (blocks, coverage) = duplicate_blocks(&files, 12);
    assert!(blocks.is_empty());
    assert_eq!(coverage.skipped_windows, 70);
}

#[test]
fn equal_hashes_of_different_windows_do_not_match() {
    // The first eleven lines are equal, the twelfth differs.
    let mut a = run(100, 11);
    a.push(1);
    let mut b = run(100, 11);
    b.push(2);
    assert!(
        duplicate_blocks(&[file("a.rs", 1, &a), file("b.rs", 1, &b)], 12)
            .0
            .is_empty()
    );
}
