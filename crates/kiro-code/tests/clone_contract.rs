//! Code lines for the duplicate search (clone-constraints 1.1): tokens of
//! the shared syntax tree, per line, without whitespace and comments.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_code::clones::file_lines;

const PLAIN: &str = "fn add(a: u8, b: u8) -> u8 {
    let sum = a + b;
    sum
}
";

/// The same code with other indentation, spacing inside the lines, a
/// comment line, a trailing comment and a blank line.
const DRESSED: &str = "fn add( a: u8,  b: u8 ) -> u8 {
        // adds two numbers

    let   sum = a+b; // the sum
    sum
}
";

#[test]
fn code_lines_carry_their_line_numbers_and_skip_blank_and_comment_lines() {
    let plain = file_lines("a.rs", PLAIN).unwrap();
    let numbers: Vec<u32> = plain.lines.iter().map(|line| line.line).collect();
    assert_eq!(numbers, [1, 2, 3, 4]);

    let dressed = file_lines("b.rs", DRESSED).unwrap();
    let numbers: Vec<u32> = dressed.lines.iter().map(|line| line.line).collect();
    assert_eq!(numbers, [1, 4, 5, 6], "lines 2 and 3 hold no code");
}

#[test]
fn whitespace_and_comments_do_not_change_a_line() {
    let plain = file_lines("a.rs", PLAIN).unwrap();
    let dressed = file_lines("b.rs", DRESSED).unwrap();
    let hashes = |file: &kiro_core::code::clones::FileLines| -> Vec<u64> {
        file.lines.iter().map(|line| line.hash).collect()
    };
    assert_eq!(hashes(&plain), hashes(&dressed));
}

#[test]
fn a_different_token_changes_the_line() {
    let plain = file_lines("a.rs", PLAIN).unwrap();
    let renamed = file_lines("a.rs", &PLAIN.replace("let sum", "let total")).unwrap();
    assert_ne!(plain.lines[1].hash, renamed.lines[1].hash);
    assert_eq!(plain.lines[0].hash, renamed.lines[0].hash);
    // Adjacent tokens are kept apart: `a b` is not `ab`.
    let joined = file_lines("a.rs", "fn f() { let ab = 1; }\n").unwrap();
    let apart = file_lines("a.rs", "fn f() { let a = 1; }\n").unwrap();
    assert_ne!(joined.lines[0].hash, apart.lines[0].hash);
}

#[test]
fn line_endings_do_not_change_the_lines() {
    let lf = file_lines("a.rs", PLAIN).unwrap();
    let crlf = file_lines("a.rs", &PLAIN.replace('\n', "\r\n")).unwrap();
    assert_eq!(lf, crlf);
}

#[test]
fn files_that_do_not_parse_or_have_no_profile_take_no_part() {
    assert!(file_lines("broken.rs", "fn broken( {\n").is_none());
    assert!(file_lines("notes.txt", "hello\n").is_none());
}

#[test]
fn other_languages_use_the_same_rule() {
    let python = file_lines("a.py", "def f(a):\n    # note\n    return a\n").unwrap();
    assert_eq!(
        python
            .lines
            .iter()
            .map(|line| line.line)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    let typescript = file_lines("a.ts", "const f = (a: number) => a; // id\n").unwrap();
    assert_eq!(typescript.lines.len(), 1);
}
